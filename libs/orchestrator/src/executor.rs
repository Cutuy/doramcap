use crate::{Error, Event, Invocation, Observer, Result, State, Step};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    any::{Any, TypeId, type_name},
    cell::Cell,
    collections::{HashMap, HashSet},
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

struct Registration {
    factory: Box<dyn Any>,
    step: &'static str,
    display_name: Option<&'static str>,
}

/// Execute registered steps as local coroutines.
///
/// # Ownership and concurrency
///
/// The executor owns its factories and observer; each call creates a fresh step.
/// Factories own their captures, and shared services can outlive the executor.
/// Calls run in the caller's task and may be non-`Send`; `tokio::join!` runs them
/// cooperatively. Nested calls inherit the parent ID; spawned tasks do not.
/// Registration requires exclusive access; callers coordinate side effects.
///
/// # Errors and cancellation
///
/// Validation, codec and step errors are returned; factory, body and codec panics
/// unwind. Observer panics are counted and suppressed. Cancellation drops the step
/// future; callers remain responsible for async cleanup. Calls are never retried.
pub struct LocalExecutor {
    factories: HashMap<TypeId, Registration>,
    registration_error: Option<String>,
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
    /// Construct an executor with no factories and the supplied observer.
    ///
    /// # Behavior
    ///
    /// Callbacks are synchronous and must not block; panics are counted and
    /// suppressed. The default label is `local` and payload capture is disabled.
    pub fn new(observer: impl Observer + 'static) -> Self {
        Self {
            factories: HashMap::new(),
            registration_error: None,
            session: "local".into(),
            next_id: Cell::new(1),
            origin: Instant::now(),
            owner: Rc::new(()),
            capture_payloads: false,
            observer: Box::new(observer),
            observer_failures: Cell::new(0),
        }
    }

    /// Set the event stream namespace (wire field `session`).
    ///
    /// # Behavior
    ///
    /// Each executor in a combined stream, including replacements, needs a distinct
    /// label: invocation IDs and elapsed-time origins restart per executor. Labels
    /// do not affect dependency ownership or service lifetimes.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.session = label.into();
        self
    }

    /// Include JSON input/output in events. Default false; payloads may be sensitive.
    pub fn capture_payloads(mut self, enabled: bool) -> Self {
        self.capture_payloads = enabled;
        self
    }

    /// Register a factory for `S`, replacing any previous factory.
    ///
    /// # Behavior
    ///
    /// The factory owns captures; registration captures static step identity and label.
    /// Invalid/duplicate declared metadata fails before invocation; factory panics unwind.
    pub fn register<S: Step + 'static>(&mut self, factory: impl Fn() -> S + 'static) {
        let factory: Box<dyn Fn() -> S> = Box::new(factory);
        self.factories.insert(
            TypeId::of::<S>(),
            Registration {
                factory: Box::new(factory),
                step: S::ID.unwrap_or(type_name::<S>()),
                display_name: S::DISPLAY_NAME,
            },
        );
        self.registration_error = self.check_registrations();
    }

    /// Reject invalid registered names before invoking any step.
    pub fn validate_steps(&self) -> Result<()> {
        match &self.registration_error {
            Some(error) => Err(Error::Configuration(error.clone())),
            None => Ok(()),
        }
    }

    fn check_registrations(&self) -> Option<String> {
        let mut ids = HashSet::new();
        for registration in self.factories.values() {
            if registration.step.trim().is_empty()
                || registration
                    .display_name
                    .is_some_and(|name| name.trim().is_empty())
                || !ids.insert(registration.step)
            {
                return Some(format!(
                    "invalid or duplicate step identity: {}",
                    registration.step
                ));
            }
        }
        None
    }

    /// Number of observer callbacks that panicked. Check evidence coverage separately.
    pub fn observer_failures(&self) -> u64 {
        self.observer_failures.get()
    }

    /// Invoke a registered step and return its result with producer metadata.
    ///
    /// # Behavior
    ///
    /// Input and output cross a JSON round-trip. The original input is dropped
    /// after input preparation, before `Started` and factory construction; the
    /// step receives the decoded value. Invalid input prevents step execution.
    ///
    /// `input` may declare producer dependencies. Outputs from another executor
    /// are rejected; nested calls inherit the current invocation as parent. The
    /// returned [`Output`] owns the decoded result and producer identity.
    ///
    /// # Errors
    ///
    /// [`Error::UnregisteredStep`], [`Error::InvalidDependency`] and
    /// [`Error::IdExhausted`] occur before events. JSON failures return
    /// [`Error::Serialization`]; step errors propagate. Output errors do not undo effects.
    ///
    /// # Cancellation and panics
    ///
    /// Dropping an unpolled future does nothing. Dropping a pending call emits
    /// `Cancelled`; callers remain responsible for async cleanup. Panics unwind
    /// and emit `Panicked`: input codec or destructor panics can precede `Started`;
    /// factory, body and output codec panics follow it.
    pub async fn run<S: Step + 'static>(
        &self,
        input: impl Into<Input<S::Input>>,
    ) -> Result<Output<S::Output>> {
        self.validate_steps()?;
        let registration = self
            .factories
            .get(&TypeId::of::<S>())
            .ok_or(Error::UnregisteredStep(type_name::<S>()))?;
        let factory = registration
            .factory
            .downcast_ref::<Box<dyn Fn() -> S>>()
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
            step: registration.step.into(),
            display_name: registration.display_name.map(str::to_owned),
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
        // Only decoded input needs to survive observation and step execution.
        drop(input.value);
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
