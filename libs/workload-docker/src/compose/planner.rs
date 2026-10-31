//! Pure TypeScript planning behind generated protobuf messages and an owned byte-buffer boundary.
use super::{Result, error::invalid};
use prost::Message;
use rquickjs::{Context, Ctx, Exception, Function, Object, Runtime, TypedArray};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

pub(crate) use crate::proto::doramcap::compose::planner::v1::*;
const LIMIT: usize = 64 * 1024 * 1024;

struct Cancel(Arc<AtomicBool>);
impl Drop for Cancel {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}
/// Invoke an isolated, bounded engine. Cancellation interrupts JS; no host I/O APIs are exposed.
async fn invoke(operation: &'static str, input: Vec<u8>) -> Result<Vec<u8>> {
    if input.len() > LIMIT {
        return Err(invalid("planner input exceeds 64 MiB"));
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    let _cancel = Cancel(cancelled.clone());
    tokio::task::spawn_blocking(move || {
        evaluate(
            operation,
            input,
            cancelled,
            Duration::from_secs(30),
            include_str!("../../planner/bundle.js"),
        )
    })
    .await
    .map_err(|e| invalid(format!("planner task: {e}")))?
}
fn evaluate(
    operation: &str,
    input: Vec<u8>,
    cancelled: Arc<AtomicBool>,
    budget: Duration,
    script: &str,
) -> Result<Vec<u8>> {
    let runtime = Runtime::new().map_err(|e| invalid(e.to_string()))?;
    runtime.set_memory_limit(256 * 1024 * 1024);
    runtime.set_max_stack_size(512 * 1024);
    let deadline = Instant::now() + budget;
    runtime.set_interrupt_handler(Some(Box::new(move || {
        cancelled.load(Ordering::Relaxed) || Instant::now() >= deadline
    })));
    let context = Context::full(&runtime).map_err(|e| invalid(e.to_string()))?;
    context.with(|ctx| {
        let run = || -> rquickjs::Result<Vec<u8>> {
            ctx.globals().set(
                "__plannerUtf8",
                Function::new(
                    ctx.clone(),
                    |ctx: Ctx<'_>, bytes: TypedArray<'_, u8>, fatal: bool| {
                        // SAFETY: copy the live buffer before any engine allocation or exception.
                        let bytes = unsafe { bytes.as_bytes() }
                            .ok_or(rquickjs::Error::Unknown)?
                            .to_vec();
                        if fatal {
                            String::from_utf8(bytes)
                                .map_err(|_| Exception::throw_type(&ctx, "invalid UTF-8"))
                        } else {
                            Ok(String::from_utf8_lossy(&bytes).into_owned())
                        }
                    },
                )?,
            )?;
            ctx.eval::<(), _>(script)?;
            let planner: Object = ctx.globals().get("doramcapPlanner")?;
            let function: Function = planner.get(operation)?;
            let argument = TypedArray::new_copy(ctx.clone(), input)?;
            let output: TypedArray<u8> = function.call((argument,))?;
            if output.len() > LIMIT {
                return Err(Exception::throw_range(
                    &ctx,
                    "planner output exceeds 64 MiB",
                ));
            }
            // SAFETY: no JS calls occur while copying the borrowed, live typed-array buffer.
            Ok(unsafe { output.as_bytes() }
                .ok_or(rquickjs::Error::Unknown)?
                .to_vec())
        };
        let output = run().map_err(|error| {
            let exception = ctx.catch();
            let detail = exception
                .as_object()
                .and_then(|o| o.get::<_, String>("message").ok())
                .unwrap_or_else(|| error.to_string());
            invalid(format!("Compose planner {operation}: {detail}"))
        })?;
        if output.len() > LIMIT {
            return Err(invalid("planner output exceeds 64 MiB"));
        }
        Ok(output)
    })
}

/// Discover image, input-source and name requirements without host side effects.
pub(crate) async fn analyze(input: &AnalyzeInput) -> Result<AnalyzeOutput> {
    AnalyzeOutput::decode(invoke("analyze", input.encode_to_vec()).await?.as_slice())
        .map_err(|e| invalid(e.to_string()))
}
/// Apply supplied host facts and return the generated, typed configuration result.
pub(crate) async fn complete(input: CompleteInput) -> Result<CompleteOutput> {
    CompleteOutput::decode(invoke("complete", input.encode_to_vec()).await?.as_slice())
        .map_err(|e| invalid(e.to_string()))
}
/// Export a typed Compose model to native Docker JSON using the same planner bundle.
pub(crate) async fn render(model: &super::model::ComposeModel) -> Result<Vec<u8>> {
    invoke("render", model.encode_to_vec()).await
}

#[cfg(test)]
mod tests;
