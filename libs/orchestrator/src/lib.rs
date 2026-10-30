//! Observable steps in ordinary procedural Rust.
//!
//! A [`Step`] is a transient struct: [`LocalExecutor::run`] creates a fresh instance,
//! checks serialized boundaries, awaits it in place, and publishes lifecycle events.
//! Factories inject typed dependencies into step fields. Workflows are ordinary
//! async Rust. Futures need not be `Send`; registered factories own their captures.
//!
//! [`Output`] carries producer identity when passed directly into another step.
//! Use [`Input::after`] for dependencies of transformed or combined inputs.
//! See the crate README and the `experiment` example for explicit error cleanup.

mod error;
mod event;
mod executor;
pub mod process;
pub mod telemetry;

pub use error::{Error, Result};
pub use event::{Event, Invocation, Observer, State};
pub use executor::{Input, LocalExecutor, Output};
use serde::{Serialize, de::DeserializeOwned};

/// One transient execution object, created by its registered factory for each invocation.
///
/// Inject services through typed fields. Input and output must round-trip through
/// JSON on every call, including local execution. Services are never serialized.
/// `run` executes cooperatively on the caller's executor; it must not block it.
///
/// Errors propagate to the procedural caller. Dropping the call drops its future
/// and instance, but does not asynchronously undo effects: steps and callers own
/// partial-failure cleanup. Panics unwind normally. No automatic retry is performed.
#[allow(async_fn_in_trait)]
pub trait Step {
    /// Explicit, owned invocation arguments.
    type Input: Serialize + DeserializeOwned;
    /// Serializable result; use identifiers rather than OS handles.
    type Output: Serialize + DeserializeOwned;

    /// Caller-declared telemetry revision; currently not used for decoding, dispatch,
    /// or compatibility checks.
    const VERSION: u32 = 1;

    /// Stable step identity; None uses the Rust type name.
    const ID: Option<&'static str> = None;

    /// Optional display label, captured once at registration.
    const DISPLAY_NAME: Option<&'static str> = None;

    /// Perform one invocation using injected fields and owned inputs.
    async fn run(&mut self, input: Self::Input) -> Result<Self::Output>;
}
