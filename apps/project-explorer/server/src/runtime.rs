//! Local task supervisor connecting launch coordination to Compose workflows.
use crate::{
    lifecycle::{Action, AppLifecycle, Launch, Status, Token},
    manager::{Error, Manager},
    registry::Registration,
    workflow,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tokio::sync::{RwLock, mpsc, oneshot};
use workload_docker::compose::DockerAdapter;

enum Command {
    Start {
        app: Box<Registration>,
        lifecycle: AppLifecycle,
        generation: Token,
    },
    Stop {
        id: String,
        generation: Token,
    },
}

/// Client handle for a local workflow supervisor. Dropping it does not drain
/// running apps; the hosting process must explicitly coordinate shutdown.
#[derive(Clone)]
pub struct Service {
    manager: Manager,
    commands: mpsc::UnboundedSender<Command>,
    closing: Arc<AtomicBool>,
    launch_gate: Arc<RwLock<()>>,
    endpoints: Arc<RwLock<HashMap<(String, Token), std::net::SocketAddr>>>,
    result_root: PathBuf,
}

struct Worker {
    adapter: Rc<DockerAdapter>,
    root: PathBuf,
    uid: u32,
    gid: u32,
    running: RefCell<HashMap<(String, Token), oneshot::Sender<()>>>,
    endpoints: Arc<RwLock<HashMap<(String, Token), std::net::SocketAddr>>>,
}

impl Service {
    /// Return whether the trusted registration uses the shared headless shell.
    pub fn is_headless(&self, id: &str) -> Result<bool, Error> {
        self.manager
            .get(id)
            .map(|entry| entry.registration.headless)
            .ok_or(Error::UnknownApp)
    }
    /// Return the registered result format for the shared viewer shell.
    pub fn result_format(&self, id: &str) -> Result<crate::registry::ResultFormat, Error> {
        self.manager
            .get(id)
            .map(|entry| entry.registration.result_format())
            .ok_or(Error::UnknownApp)
    }
    /// Start the local supervisor within a running Tokio `LocalSet`. The caller
    /// must already have performed manager startup discovery. External workflow
    /// futures are owned by spawned local tasks, independent of request lifetime.
    pub fn spawn_local(
        manager: Manager,
        adapter: Rc<DockerAdapter>,
        root: PathBuf,
        uid: u32,
        gid: u32,
    ) -> Self {
        let (commands, mut receiver) = mpsc::unbounded_channel();
        let endpoints = Arc::new(RwLock::new(HashMap::new()));
        let worker = Rc::new(Worker {
            adapter,
            root: root.clone(),
            uid,
            gid,
            running: RefCell::new(HashMap::new()),
            endpoints: endpoints.clone(),
        });
        tokio::task::spawn_local(async move {
            while let Some(command) = receiver.recv().await {
                match command {
                    Command::Start {
                        app,
                        lifecycle,
                        generation,
                    } => {
                        let worker = worker.clone();
                        let guard = lifecycle.clone();
                        let task = tokio::task::spawn_local(async move {
                            worker.start(*app, lifecycle, generation).await;
                        });
                        tokio::task::spawn_local(async move {
                            if let Err(error) = task.await {
                                guard.fail(generation).await;
                                guard
                                    .cleaned(
                                        generation,
                                        Err(format!(
                                            "startup task failed: {error}; cleanup unverified"
                                        )),
                                    )
                                    .await;
                            }
                        });
                    }
                    Command::Stop { id, generation, .. } => {
                        worker.stop(id, generation).await;
                    }
                }
            }
        });
        Self {
            manager,
            commands,
            closing: Arc::new(AtomicBool::new(false)),
            launch_gate: Arc::new(RwLock::new(())),
            endpoints,
            result_root: root,
        }
    }
    /// Accept a session and return immediately while any new startup runs in the
    /// supervisor. Requests joining a generation never start a second workflow.
    pub async fn launch(&self, id: &str) -> Result<Launch, Error> {
        let _admission = self.launch_gate.read().await;
        if self.closing.load(Ordering::Acquire) {
            return Err(Error::ShuttingDown);
        }
        let launch = self.manager.launch(id, Instant::now()).await?;
        if launch.start {
            let entry = self.manager.get(id).expect("manager accepted this ID");
            if self
                .commands
                .send(Command::Start {
                    app: Box::new(entry.registration.clone()),
                    lifecycle: entry.lifecycle.clone(),
                    generation: launch.generation,
                })
                .is_err()
            {
                entry.lifecycle.fail(launch.generation).await;
                entry
                    .lifecycle
                    .cleaned(
                        launch.generation,
                        Ok(Some("startup supervisor unavailable".into())),
                    )
                    .await;
                return Err(Error::Blocked("startup supervisor unavailable".into()));
            }
        }
        if self.closing.load(Ordering::Acquire) {
            let entry = self.manager.get(id).expect("manager accepted this ID");
            self.dispatch_stop(
                entry.registration.id.clone(),
                &entry.lifecycle,
                entry.lifecycle.request_stop().await,
            )
            .await;
            return Err(Error::ShuttingDown);
        }
        Ok(launch)
    }
    /// Renew a session in the specified generation.
    pub async fn heartbeat(
        &self,
        id: &str,
        generation: Token,
        session: Token,
    ) -> Result<bool, Error> {
        let app = self.manager.get(id).ok_or(Error::UnknownApp)?;
        Ok(app
            .lifecycle
            .heartbeat(generation, session, Instant::now())
            .await)
    }
    /// Return an internal endpoint only for the current ready generation.
    /// The caller must not expose the returned Docker address to browsers.
    pub async fn upstream(
        &self,
        id: &str,
        generation: Token,
    ) -> Result<Option<std::net::SocketAddr>, Error> {
        let app = self.manager.get(id).ok_or(Error::UnknownApp)?;
        if app.lifecycle.status().await != (Status::Ready { generation }) {
            return Ok(None);
        }
        let address = self
            .endpoints
            .read()
            .await
            .get(&(app.registration.id.clone(), generation))
            .copied();
        if app.lifecycle.status().await != (Status::Ready { generation }) {
            return Ok(None);
        }
        Ok(address)
    }
    /// Return trusted paths for a registered generation's finalized recording.
    pub fn recording_paths(
        &self,
        id: &str,
        generation: Token,
    ) -> Result<(PathBuf, PathBuf, String), Error> {
        let app = self.manager.get(id).ok_or(Error::UnknownApp)?;
        let directory = self
            .result_root
            .join(format!("{}-{generation:032x}", app.registration.id));
        Ok((
            directory.join("metadata/recording.complete"),
            directory
                .join("results")
                .join(&app.registration.result_file),
            app.registration.result_file.clone(),
        ))
    }
    /// Whether the finalized generation contained its primary result.
    /// Generations created before primary-state metadata are treated as present.
    pub fn primary_present(&self, id: &str, generation: Token) -> Result<bool, Error> {
        use std::{io::Read, os::unix::fs::OpenOptionsExt};
        let app = self.manager.get(id).ok_or(Error::UnknownApp)?;
        let path = self
            .result_root
            .join(format!("{}-{generation:032x}", app.registration.id))
            .join("metadata/primary-state.json");
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
            Err(_) => return Ok(false),
        };
        if !file.metadata().is_ok_and(|metadata| metadata.is_file()) {
            return Ok(false);
        }
        let mut bytes = Vec::new();
        if file.take(1024).read_to_end(&mut bytes).is_err() {
            return Ok(false);
        }
        let Ok(state) = serde_json::from_slice::<Vec<String>>(&bytes) else {
            return Ok(false);
        };
        Ok(state == ["present", app.registration.result_file.as_str()])
    }
    /// Return the optional server-defined directory for capture recordings.
    pub fn captures_directory(
        &self,
        id: &str,
        generation: Token,
    ) -> Result<Option<PathBuf>, Error> {
        let app = self.manager.get(id).ok_or(Error::UnknownApp)?;
        let directory = self
            .result_root
            .join(format!("{}-{generation:032x}", app.registration.id))
            .join("results");
        Ok(app
            .registration
            .capture_directory
            .as_ref()
            .map(|name| directory.join(name)))
    }
    /// Return the optional flat files declared by the trusted registration.
    pub fn additional_files(
        &self,
        id: &str,
        generation: Token,
    ) -> Result<Vec<(String, PathBuf)>, Error> {
        let app = self.manager.get(id).ok_or(Error::UnknownApp)?;
        use std::{io::Read, os::unix::fs::OpenOptionsExt};
        let generation_directory = self
            .result_root
            .join(format!("{}-{generation:032x}", app.registration.id));
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(generation_directory.join("metadata/additional-index.json"))
        {
            Ok(file) => file,
            Err(_) => return Ok(Vec::new()),
        };
        if !file.metadata().is_ok_and(|metadata| metadata.is_file()) {
            return Ok(Vec::new());
        }
        let mut bytes = Vec::new();
        if file.take(16384).read_to_end(&mut bytes).is_err() {
            return Ok(Vec::new());
        }
        let Ok(files) = serde_json::from_slice::<Vec<String>>(&bytes) else {
            return Ok(Vec::new());
        };
        if files.len() > 32 || files.iter().any(|name| !crate::registry::simple_name(name)) {
            return Ok(Vec::new());
        }
        let directory = generation_directory.join("results");
        Ok(files
            .iter()
            .map(|name| (name.clone(), directory.join(name)))
            .collect())
    }
    /// Return the frozen, verified artifact collections of a generation.
    pub fn artifact_collections(
        &self,
        id: &str,
        generation: Token,
    ) -> Result<Vec<workflow::PublishedCollection>, Error> {
        use std::{io::Read, os::unix::fs::OpenOptionsExt};
        let app = self.manager.get(id).ok_or(Error::UnknownApp)?;
        let path = self
            .result_root
            .join(format!("{}-{generation:032x}", app.registration.id))
            .join("metadata/collections-index.json");
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
        {
            Ok(file) => file,
            Err(_) => return Ok(Vec::new()),
        };
        if !file.metadata().is_ok_and(|metadata| metadata.is_file()) {
            return Ok(Vec::new());
        }
        let mut bytes = Vec::new();
        if file.take(2 * 1024 * 1024).read_to_end(&mut bytes).is_err() {
            return Ok(Vec::new());
        }
        let Ok(collections) = serde_json::from_slice::<Vec<workflow::PublishedCollection>>(&bytes)
        else {
            return Ok(Vec::new());
        };
        if collections.len() > 8
            || collections.iter().any(|collection| {
                !crate::registry::simple_name(&collection.id)
                    || collection.files.len() > 512
                    || collection
                        .files
                        .iter()
                        .any(|name| !crate::registry::artifact_path(name))
            })
        {
            return Ok(Vec::new());
        }
        Ok(collections)
    }
    /// Registered public application IDs in stable order.
    pub fn app_ids(&self) -> Vec<String> {
        self.manager
            .iter()
            .map(|entry| entry.registration.id.clone())
            .collect()
    }
    /// Return the app's current status.
    pub async fn status(&self, id: &str) -> Result<Status, Error> {
        self.manager.status(id).await
    }
    async fn dispatch_stop(&self, id: String, lifecycle: &AppLifecycle, action: Action) {
        if let Action::Stop(generation) = action
            && self
                .commands
                .send(Command::Stop { id, generation })
                .is_err()
        {
            lifecycle
                .cleaned(generation, Err("cleanup supervisor unavailable".into()))
                .await;
        }
    }
    /// Expire sessions using monotonic time and dispatch any required stops.
    /// Call periodically, including during startup.
    pub async fn expire(&self, now: Instant) {
        for app in self.manager.iter() {
            self.dispatch_stop(
                app.registration.id.clone(),
                &app.lifecycle,
                app.lifecycle.expire(now).await,
            )
            .await;
        }
    }
    /// Refuse new sessions and drain all owned generations. Starting operations
    /// finish before their cleanup. This future waits for each cleanup result;
    /// callers must keep the local executor alive until it resolves.
    pub async fn shutdown(&self) -> Vec<(String, Status)> {
        {
            let _admission = self.launch_gate.write().await;
            self.closing.store(true, Ordering::Release);
        }
        for app in self.manager.iter() {
            self.dispatch_stop(
                app.registration.id.clone(),
                &app.lifecycle,
                app.lifecycle.request_stop().await,
            )
            .await;
        }
        loop {
            let mut pending = false;
            let mut results = Vec::new();
            for app in self.manager.iter() {
                let status = app.lifecycle.status().await;
                pending |= matches!(
                    status,
                    Status::Starting { .. } | Status::Ready { .. } | Status::Stopping { .. }
                );
                results.push((app.registration.id.to_owned(), status));
            }
            if !pending {
                return results;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }
}

impl Worker {
    async fn start(self: Rc<Self>, app: Registration, lifecycle: AppLifecycle, generation: Token) {
        let id = app.id.clone();
        let key = (id.clone(), generation);
        let (stop, stopped) = oneshot::channel();
        let (ready, mut started) = oneshot::channel();
        self.running.borrow_mut().insert(key.clone(), stop);
        let completion = workflow::run(
            self.adapter.clone(),
            &app,
            generation,
            &self.root,
            (self.uid, self.gid),
            ready,
            stopped,
        );
        tokio::pin!(completion);
        let report = tokio::select! {
            report = &mut completion => report,
            endpoint = &mut started => {
                if let Ok(endpoint) = endpoint {
                    if let Some(endpoint) = endpoint { self.endpoints.write().await.insert(key.clone(), endpoint); }
                    if lifecycle.started(generation, Instant::now()).await == Action::Stop(generation) {
                        self.stop(id.clone(), generation).await;
                    }
                }
                completion.await
            }
        };
        self.running.borrow_mut().remove(&key);
        self.endpoints.write().await.remove(&key);
        lifecycle.fail(generation).await;
        let diagnostic = (!report.errors.is_empty()).then(|| report.errors.join("; "));
        lifecycle
            .cleaned(
                generation,
                if report.cleanup_verified {
                    Ok(diagnostic)
                } else {
                    Err(diagnostic.unwrap_or_else(|| "cleanup unverified".into()))
                },
            )
            .await;
    }

    async fn stop(&self, id: String, generation: Token) {
        self.endpoints
            .write()
            .await
            .remove(&(id.clone(), generation));
        if let Some(stop) = self.running.borrow_mut().remove(&(id, generation)) {
            let _ = stop.send(());
        }
    }
}
