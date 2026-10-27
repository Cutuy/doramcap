use crate::{Error, Event, Invocation, Observer, Result, State, Step};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    any::{Any, TypeId, type_name},
    cell::Cell,
    collections::HashMap,
    rc::Rc,
    time::Instant,
};

// Scoped per future poll: concurrent sibling calls cannot inherit each other's ID.
tokio::task_local! {
    static ACTIVE: (Rc<()>, u64);
}

/// An output plus producer identity. Pass directly to `LocalExecutor::run` for lineage.
///
/// The private executor token prevents output references from a different root
/// being mistaken for local dependencies. Values are serialized at call boundaries;
/// this local wrapper itself is not a wire protocol.
#[derive(Debug)]
pub struct Output<T> {
    value: T,
    invocation: Invocation,
    owner: Rc<()>,
}

impl<T> Output<T> {
    /// Read the value while retaining its producer reference.
    pub fn value(&self) -> &T {
        &self.value
    }
    /// Read producer identity.
    pub fn invocation(&self) -> &Invocation {
        &self.invocation
    }
    /// Extract the value, deliberately discarding automatic dependency tracking.
    pub fn into_value(self) -> T {
        self.value
    }
}

impl<T: Clone> Clone for Output<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            invocation: self.invocation.clone(),
            owner: self.owner.clone(),
        }
    }
}

/// Owned arguments and their declared producer references.
///
/// Ordinary values convert automatically. Passing an `Output<T>` directly adds
/// its producer automatically. Use `Input::new(value).after(&a).after(&b)` for fan-in
/// or transformed arguments. Equal values/paths alone never imply dependency.
pub struct Input<T> {
    value: T,
    dependencies: Vec<(Rc<()>, u64)>,
}

impl<T> Input<T> {
    /// Construct external or otherwise untracked input.
    pub fn new(value: T) -> Self {
        Self {
            value,
            dependencies: Vec::new(),
        }
    }
    /// Declare an output as a prerequisite, without copying its value.
    pub fn after<U>(mut self, output: &Output<U>) -> Self {
        self.dependencies
            .push((output.owner.clone(), output.invocation.id));
        self
    }
}

impl<T> From<T> for Input<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}
impl<T> From<Output<T>> for Input<T> {
    fn from(output: Output<T>) -> Self {
        Self {
            value: output.value,
            dependencies: vec![(output.owner, output.invocation.id)],
        }
    }
}

/// Execute registered steps as local coroutines.
///
/// # Synopsis
///
/// Create with [`Self::new`], register typed factories with [`Self::register`],
/// then await [`Self::run`] from ordinary procedural Rust.
///
/// # Ownership
///
/// Owns factories and the observer until dropped. Each invocation creates and
/// drops a fresh step. Factories own captured dependencies; `Rc`/`Arc` services
/// can outlive the executor when another owner retains them.
///
/// # Concurrency
///
/// Calls execute in the caller's task; futures and services may be non-`Send`.
/// Use `tokio::join!` for cooperative concurrent calls. Nested calls through this
/// executor inherit the active parent ID; spawned tasks do not inherit that scope.
/// Factory registration requires exclusive access. Callers coordinate side effects.
///
/// # Failure semantics
///
/// Calls report validation, serialization and step errors. Factory/body panics
/// unwind. Observer panics are counted and suppressed. Cancellation drops the step
/// future; callers retain responsibility for asynchronous cleanup. No retry occurs.
pub struct LocalExecutor {
    factories: HashMap<TypeId, Box<dyn Any>>,
    session: String,
    next_id: Cell<u64>,
    origin: Instant,
    owner: Rc<()>,
    capture_payloads: bool,
    observer: Box<dyn Observer>,
    observer_failures: Cell<u64>,
}

impl Default for LocalExecutor {
    fn default() -> Self {
        Self::new(|event: &Event| {
            tracing::info!(target: "orchestrator", session = %event.invocation.session,
                step = %event.invocation.step, invocation = event.invocation.id,
                state = ?event.state, duration_ns = event.duration_ns, "step");
        })
    }
}

impl LocalExecutor {
    /// Construct an executor owning `observer`, with no registered factories.
    ///
    /// The observer receives synchronous lifecycle callbacks and must not block.
    /// Callback panics are suppressed and counted by [`Self::observer_failures`].
    /// The initial telemetry label is `local`; payload capture is disabled.
    pub fn new(observer: impl Observer + 'static) -> Self {
        Self {
            factories: HashMap::new(),
            session: "local".into(),
            next_id: Cell::new(1),
            origin: Instant::now(),
            owner: Rc::new(()),
            capture_payloads: false,
            observer: Box::new(observer),
            observer_failures: Cell::new(0),
        }
    }

    /// Attach a caller-defined telemetry grouping label (wire field `session`).
    /// Labels do not determine dependency ownership or service lifetimes.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.session = label.into();
        self
    }

    /// Include JSON input/output in events. Default false; payloads may be sensitive.
    pub fn capture_payloads(mut self, enabled: bool) -> Self {
        self.capture_payloads = enabled;
        self
    }

    /// Register a factory for S. Re-registering replaces the previous factory.
    ///
    /// The factory owns its captures and runs once per validated invocation. It
    /// must return a fresh S; shared dependencies belong in its typed fields.
    /// Missing factories fail before emitting events or invoking user code.
    /// Factory panics unwind with a Panicked event, like step-body panics.
    pub fn register<S: Step + 'static>(&mut self, factory: impl Fn() -> S + 'static) {
        let factory: Box<dyn Fn() -> S> = Box::new(factory);
        self.factories.insert(TypeId::of::<S>(), Box::new(factory));
    }

    /// Number of observer callbacks that panicked. Check evidence coverage separately.
    pub fn observer_failures(&self) -> u64 {
        self.observer_failures.get()
    }

    /// Invoke a registered step and return its result with producer metadata.
    ///
    /// # Parameters and return value
    ///
    /// `input` supplies owned arguments and optional producer dependencies. The
    /// returned [`Output`] owns the decoded result and this invocation's identity.
    ///
    /// # Behavior
    ///
    /// Input and output are JSON-round-tripped. Invalid input prevents body
    /// entry; output errors occur after the body and do not undo its effects.
    /// Metadata includes the automatically derived Rust type name and declared
    /// producers. Passing outputs from another executor fails before invocation.
    /// Nested calls on this executor receive this invocation as their parent.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnregisteredStep`], [`Error::InvalidDependency`] or
    /// [`Error::IdExhausted`] before invocation events. JSON failures return
    /// [`Error::Serialization`]; body errors propagate unchanged. Output failures
    /// occur after side effects and do not roll them back.
    ///
    /// # Cancellation and panics
    ///
    /// Dropping an unpolled future performs no work. Dropping a polled, pending call
    /// emits `Cancelled` (or `Panicked` during unwinding). It does not imply OS-level
    /// effects ended; the caller and step retain explicit cleanup obligations.
    pub async fn run<S: Step + 'static>(
        &self,
        input: impl Into<Input<S::Input>>,
    ) -> Result<Output<S::Output>> {
        let factory = self
            .factories
            .get(&TypeId::of::<S>())
            .and_then(|entry| entry.downcast_ref::<Box<dyn Fn() -> S>>())
            .ok_or(Error::UnregisteredStep(type_name::<S>()))?;
        let input = input.into();
        if input
            .dependencies
            .iter()
            .any(|(owner, _)| !Rc::ptr_eq(owner, &self.owner))
        {
            return Err(Error::InvalidDependency);
        }
        let id = self.next_id.get();
        self.next_id
            .set(id.checked_add(1).ok_or(Error::IdExhausted)?);
        let mut dependencies: Vec<_> = input.dependencies.into_iter().map(|(_, id)| id).collect();
        dependencies.sort_unstable();
        dependencies.dedup();
        let invocation = Invocation {
            session: self.session.clone(),
            id,
            step: type_name::<S>().into(),
            version: S::VERSION,
            executor: "local".into(),
            parent: ACTIVE
                .try_with(|(owner, id)| Rc::ptr_eq(owner, &self.owner).then_some(*id))
                .ok()
                .flatten(),
            dependencies,
        };
        let mut guard = Guard {
            executor: self,
            invocation,
            started: Instant::now(),
            finished: false,
        };
        let prepared = roundtrip(&input.value, "input");
        let payload = if self.capture_payloads {
            prepared
                .as_ref()
                .ok()
                .map(|(_, bytes)| serde_json::from_slice(bytes).expect("validated JSON"))
        } else {
            None
        };
        guard.emit(State::Started, payload, None);
        let result: Result<(S::Output, Vec<u8>)> = async {
            let (input, _) = prepared?;
            let mut step = factory();
            let value = ACTIVE
                .scope((self.owner.clone(), id), step.run(input))
                .await?;
            let (value, output_bytes) = roundtrip(&value, "output")?;
            Ok((value, output_bytes))
        }
        .await;
        match result {
            Ok((value, bytes)) => {
                let payload = if self.capture_payloads {
                    Some(serde_json::from_slice(&bytes).expect("round-trip validated JSON"))
                } else {
                    None
                };
                guard.finish(State::Succeeded, payload, None);
                Ok(Output {
                    value,
                    invocation: guard.invocation.clone(),
                    owner: self.owner.clone(),
                })
            }
            Err(error) => {
                guard.finish(State::Failed, None, Some(error.to_string()));
                Err(error)
            }
        }
    }
}

struct Guard<'a> {
    executor: &'a LocalExecutor,
    invocation: Invocation,
    started: Instant,
    finished: bool,
}
impl Guard<'_> {
    fn emit(&self, state: State, payload: Option<serde_json::Value>, error: Option<String>) {
        let event = Event {
            invocation: self.invocation.clone(),
            state,
            elapsed_ns: nanos(self.executor.origin.elapsed()),
            duration_ns: nanos(self.started.elapsed()),
            payload,
            error,
        };
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.executor.observer.observe(&event)
        }))
        .is_err()
        {
            self.executor
                .observer_failures
                .set(self.executor.observer_failures.get().saturating_add(1));
        }
    }
    fn finish(&mut self, state: State, payload: Option<serde_json::Value>, error: Option<String>) {
        self.finished = true;
        self.emit(state, payload, error);
    }
}
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        if !self.finished {
            self.emit(
                if std::thread::panicking() {
                    State::Panicked
                } else {
                    State::Cancelled
                },
                None,
                None,
            );
        }
    }
}
fn nanos(duration: std::time::Duration) -> u64 {
    duration.as_nanos().min(u64::MAX as u128) as u64
}

pub(crate) fn roundtrip<T: Serialize + DeserializeOwned>(
    value: &T,
    boundary: &'static str,
) -> Result<(T, Vec<u8>)> {
    let bytes =
        serde_json::to_vec(value).map_err(|source| Error::Serialization { boundary, source })?;
    let decoded = serde_json::from_slice(&bytes)
        .map_err(|source| Error::Serialization { boundary, source })?;
    Ok((decoded, bytes))
}
