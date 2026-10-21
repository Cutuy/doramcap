use dora_node_api::{DoraNode, Event, futures::StreamExt};
use eyre::Result;
use fault_watchdog::{
    config,
    engine::Engine,
    incarnation,
    sinks::{DoraSink, Output},
};
use std::time::{Duration, Instant};
#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() -> Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or_else(|| eyre::eyre!("usage: fault-watchdog CONFIG.pbtxt"))?;
    let config = config::load(std::path::Path::new(&path))?;
    let mut engine = Engine::new(config.clone(), incarnation()?, incarnation()?)?;
    let (node, mut events) = DoraNode::init_from_env_force()?;
    let mut outputs = Vec::new();
    let output = config.live_output.clone().unwrap();
    outputs.push(Output::spawn(
        "live",
        &config,
        engine.limits(),
        engine.sink_check("live")?,
        move || Ok(Box::new(DoraSink { node, output })),
    )?);
    #[cfg(feature = "mcap")]
    if let Some(c) = config.mcap.clone().filter(|c| c.enabled) {
        let max = engine.limits().max_frame_bytes;
        outputs.push(Output::spawn(
            "mcap",
            &config,
            engine.limits(),
            engine.sink_check("mcap")?,
            move || {
                Ok(Box::new(fault_watchdog::sinks::mcap::Recording::new(
                    c, max,
                )?))
            },
        )?);
    }
    #[cfg(feature = "otel")]
    if let Some(c) = config.otel.clone().filter(|c| c.enabled) {
        outputs.push(Output::spawn(
            "otel",
            &config,
            engine.limits(),
            engine.sink_check("otel")?,
            move || Ok(Box::new(fault_watchdog::sinks::otel::Logs::new(c)?)),
        )?);
    }
    let start = Instant::now();
    let run: Result<()> = async {
        loop {
            let now = start.elapsed();
            for output in &outputs {
                output.observe()?;
            }
            engine.advance(now)?;
            // Repair has admission priority so sustained ingress cannot starve it.
            for output in &outputs {
                if output.take_repair() {
                    output.offer(&engine.snapshot(now))?;
                }
            }
            for frame in engine.publications(now) {
                for output in &outputs {
                    output.offer(&frame)?;
                }
            }
            tokio::select! {
                event = events.next() => match event {
                    Some(Event::Input { id, data, .. }) => {
                        match <&[u8]>::try_from(&data) {
                            Ok(bytes) => {
                                // receive records the diagnostic; malformed traffic
                                // remains isolated from other inputs.
                                let _ = engine.receive(id.as_str(), bytes, start.elapsed());
                            }
                            Err(e) => engine.reject(id.as_str(), &e, start.elapsed())?,
                        }
                    }
                    Some(Event::InputClosed { id }) => {
                        engine.disconnect(id.as_str(), start.elapsed())?;
                    }
                    Some(Event::NodeFailed { affected_input_ids, error, .. }) => {
                        for id in affected_input_ids {
                            engine.reject(id.as_str(), &error, start.elapsed())?;
                        }
                    }
                    Some(Event::Stop(_)) | None => break,
                    Some(Event::Error(e)) => engine.reject("transport", &e, start.elapsed())?,
                    _ => {}
                },
                _ = tokio::time::sleep(engine.wait(now)) => {},
                _ = tokio::signal::ctrl_c() => break,
            }
        }
        for id in &config.inputs {
            engine.disconnect(id, start.elapsed())?;
        }
        engine.advance(start.elapsed())?;
        for frame in engine.publications(start.elapsed()) {
            for output in &outputs {
                output.offer(&frame)?;
            }
        }
        let frame = engine.snapshot(start.elapsed());
        for output in &outputs {
            output.offer(&frame)?;
        }
        Ok(())
    }
    .await;
    let deadline = Instant::now() + Duration::from_millis(config.shutdown_ms.unwrap().into());
    let mut finish = Ok(());
    let completions: Vec<_> = outputs.into_iter().map(Output::close).collect();
    for completion in completions {
        if let Err(e) = completion.finish(deadline) {
            eprintln!("unfinished output: {e}");
            finish = Err(e);
        }
    }
    run.and(finish)
}
