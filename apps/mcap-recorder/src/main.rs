//! A separate Dora process; collection never calls the MCAP writer.
mod config;
mod recording;
#[cfg(test)]
mod tests;

use dora_node_api::{Event, futures::StreamExt};
use eyre::{Result, bail, eyre};
use fault_dora::Health;
use std::{collections::BTreeSet, time::Duration};

const HEALTH_INTERVAL: Duration = Duration::from_millis(100);

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let config = match args.as_slice() {
        [flag, path] if flag == "--config" => config::load(std::path::Path::new(path))?,
        _ => bail!("usage: mcap-recorder --config FILE.pbtxt"),
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let mut timer = runtime.block_on(async {
        let mut timer = tokio::time::interval(HEALTH_INTERVAL);
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        timer
    });
    let (mut node, mut events) = dora_app::environment::connect()?;
    let mut health = Health::new()?;
    let writes = health.register("mcap-recorder", "recording", None)?;
    let inputs = health.register("mcap-recorder", "inputs", None)?;
    let mut recording = match recording::Recording::create(config) {
        Ok(recording) => recording,
        Err(error) => {
            let _ = Health::failure(&writes, &error.to_string());
            let _ = health.tick(&mut node);
            return Err(error);
        }
    };
    writes.success()?;
    inputs.success()?;
    if let Err(error) = dora_app::environment::ready() {
        return recording.finish(config::State::Failed, Err(eyre!(error)));
    }
    let mut closed = BTreeSet::new();
    let (state, result) = recording::consume(
        &mut recording,
        std::iter::from_fn(|| {
            loop {
                let event = runtime.block_on(async {
                    tokio::select! {
                        event = events.next() => Some(event),
                        _ = timer.tick() => None,
                    }
                });
                match event {
                    Some(Some(event)) => {
                        let observed = match &event {
                            Event::InputClosed { id } => {
                                closed.insert(id.to_string());
                                Health::failure(&inputs, "input closed")
                            }
                            Event::InputRecovered { id } => {
                                closed.remove(id.as_str());
                                if closed.is_empty() {
                                    inputs.success().map(|_| ()).map_err(Into::into)
                                } else {
                                    Ok(())
                                }
                            }
                            Event::NodeFailed { error, .. } | Event::Error(error) => {
                                Health::failure(&inputs, error)
                            }
                            _ => Ok(()),
                        };
                        // Health loss leaves the recording and its event handling untouched.
                        if let Err(error) = observed {
                            eprintln!("mcap-recorder: health update failed: {error}");
                        }
                        return Some(event);
                    }
                    Some(None) => return None,
                    None => {
                        if let Err(error) = health.tick(&mut node) {
                            eprintln!("mcap-recorder: health publication failed: {error}");
                        }
                    }
                }
            }
        }),
    );
    if let Err(error) = &result {
        let _ = Health::failure(&writes, &error.to_string());
    }
    let _ = health.tick(&mut node);
    let finished = recording.finish(state, result);
    if let Err(error) = &finished {
        let _ = Health::failure(&writes, &error.to_string());
        let _ = health.tick(&mut node);
    }
    finished
}
