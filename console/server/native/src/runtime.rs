//! Bounded command dispatch to a single thread owning non-Send state.
use crate::policy::{COMMAND_QUEUE_CAPACITY, CONTROL_QUEUE_CAPACITY};
use std::{future::Future, thread};
use tokio::sync::{mpsc, oneshot};

type Command<T> = Box<dyn FnOnce(&mut T) + Send + 'static>;

/// A handle to state constructed, used and destroyed on its owner thread.
pub struct Owner<T: 'static> {
    commands: mpsc::Sender<Command<T>>,
    control: mpsc::Sender<Command<T>>,
    close: Option<mpsc::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}

/// Admission failure for an owner command.
#[derive(Debug, PartialEq, Eq)]
pub enum DispatchError {
    Busy,
    Closed,
}

impl<T: 'static> Owner<T> {
    /// Create the owner with a factory executed on its dedicated thread.
    pub async fn open<F, Fut, E>(factory: F) -> Result<Self, String>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        E: std::fmt::Display,
    {
        let (commands, mut receiver) = mpsc::channel::<Command<T>>(COMMAND_QUEUE_CAPACITY);
        let (control, mut control_rx) = mpsc::channel::<Command<T>>(CONTROL_QUEUE_CAPACITY);
        let keep_control = control.clone();
        let (close, mut close_rx) = mpsc::channel(1);
        let keep_commands = commands.clone();
        let keep_close = close.clone();
        let (opened, opened_rx) = oneshot::channel();
        let thread = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let Ok(runtime) = runtime else {
                let _ = opened.send(Err("failed to create owner runtime".to_owned()));
                return;
            };
            let local = tokio::task::LocalSet::new();
            local.block_on(&runtime, async move {
                // Finalizing a JavaScript handle must not implicitly stop a run.
                let (_keep_commands, _keep_control, _keep_close) =
                    (keep_commands, keep_control, keep_close);
                let mut state = match factory().await {
                    Ok(state) => state,
                    Err(error) => {
                        let _ = opened.send(Err(error.to_string()));
                        return;
                    }
                };
                if opened.send(Ok(())).is_err() {
                    return;
                }
                loop {
                    tokio::select! {
                        _ = close_rx.recv() => break,
                        Some(command) = control_rx.recv() => command(&mut state),
                        command = receiver.recv() => match command {
                            Some(command) => command(&mut state),
                            None => break,
                        },
                    }
                }
            });
        });
        match opened_rx.await {
            Ok(Ok(())) => Ok(Self {
                commands,
                control,
                close: Some(close),
                thread: Some(thread),
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error)
            }
            Err(_) => {
                let _ = thread.join();
                Err("owner exited during initialization".into())
            }
        }
    }

    /// Queue an owned operation without waiting for mailbox capacity.
    pub fn dispatch<R: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut T) -> R + Send + 'static,
    ) -> Result<oneshot::Receiver<R>, DispatchError> {
        let (reply, receiver) = oneshot::channel();
        let command = Box::new(move |state: &mut T| {
            let _ = reply.send(operation(state));
        });
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => DispatchError::Busy,
                mpsc::error::TrySendError::Closed(_) => DispatchError::Closed,
            })?;
        Ok(receiver)
    }

    /// Queue a control operation independently of the bounded ordinary mailbox.
    pub fn dispatch_control<R: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut T) -> R + Send + 'static,
    ) -> Result<oneshot::Receiver<R>, DispatchError> {
        let (reply, receiver) = oneshot::channel();
        self.control
            .try_send(Box::new(move |state| {
                let _ = reply.send(operation(state));
            }))
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => DispatchError::Busy,
                mpsc::error::TrySendError::Closed(_) => DispatchError::Closed,
            })?;
        Ok(receiver)
    }

    /// Wait for control capacity during serialized shutdown; ordinary admission stays nonblocking.
    pub async fn dispatch_shutdown<R: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut T) -> R + Send + 'static,
    ) -> Result<oneshot::Receiver<R>, DispatchError> {
        let (reply, receiver) = oneshot::channel();
        self.control
            .send(Box::new(move |state| {
                let _ = reply.send(operation(state));
            }))
            .await
            .map_err(|_| DispatchError::Closed)?;
        Ok(receiver)
    }

    /// Signal shutdown and join without blocking the asynchronous caller.
    pub async fn close(mut self) -> Result<(), String> {
        if let Some(close) = self.close.take() {
            let _ = close.try_send(());
        }
        let thread = self.thread.take().expect("owner thread present");
        tokio::task::spawn_blocking(move || thread.join())
            .await
            .map_err(|e| e.to_string())?
            .map_err(|_| "owner thread panicked".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::{Arc, Mutex, mpsc},
        thread::ThreadId,
    };

    struct State {
        ids: Arc<Mutex<Vec<ThreadId>>>,
        _local: Rc<RefCell<()>>,
    }
    impl Drop for State {
        fn drop(&mut self) {
            self.ids.lock().unwrap().push(thread::current().id());
        }
    }

    #[test]
    fn construction_commands_and_destruction_stay_on_owner_thread() {
        // Arrange
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let ids = Arc::new(Mutex::new(Vec::new()));
        let factory_ids = ids.clone();
        let caller = thread::current().id();
        runtime.block_on(async {
            let owner = Owner::open(move || async move {
                factory_ids.lock().unwrap().push(thread::current().id());
                Ok::<_, String>(State {
                    ids: factory_ids,
                    _local: Rc::new(RefCell::new(())),
                })
            })
            .await
            .unwrap();
            // Act
            let reply = owner
                .dispatch(|state| state.ids.lock().unwrap().push(thread::current().id()))
                .unwrap();
            reply.await.unwrap();
            owner.close().await.unwrap();
        });
        // Assert
        let ids = ids.lock().unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.iter().all(|id| *id == ids[0] && *id != caller));
    }

    #[test]
    fn full_mailbox_returns_busy_without_waiting() {
        // Arrange
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime.block_on(async {
            let owner = Owner::open(|| async { Ok::<_, String>(()) }).await.unwrap();
            let (started_tx, started_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let first = owner
                .dispatch(move |_| {
                    started_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                })
                .unwrap();
            started_rx.recv().unwrap();
            for _ in 0..COMMAND_QUEUE_CAPACITY {
                drop(owner.dispatch(|_| ()).unwrap());
            }
            // Act / Assert
            assert!(matches!(owner.dispatch(|_| ()), Err(DispatchError::Busy)));
            let control = owner.dispatch_control(|_| 7).unwrap();
            for _ in 1..CONTROL_QUEUE_CAPACITY {
                drop(owner.dispatch_control(|_| ()).unwrap());
            }
            assert!(matches!(
                owner.dispatch_control(|_| ()),
                Err(DispatchError::Busy)
            ));
            release_tx.send(()).unwrap();
            first.await.unwrap();
            assert_eq!(control.await.unwrap(), 7);
            owner.close().await.unwrap();
        });
    }
}
