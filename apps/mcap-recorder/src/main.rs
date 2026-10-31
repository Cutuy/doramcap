//! A separate Dora process; collection never calls the MCAP writer.
mod config;
mod recording;
#[cfg(test)]
mod tests;

use dora_node_api::futures::{StreamExt, executor::block_on};
use eyre::{Result, bail, eyre};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let config = match args.as_slice() {
        [flag, path] if flag == "--config" => config::load(std::path::Path::new(path))?,
        [path] if !path.starts_with('-') => config::cpu(path.clone()),
        _ => bail!("usage: mcap-recorder OUTPUT.mcap | --config FILE.pbtxt"),
    };
    let (_node, mut events) = dora_app::environment::connect()?;
    let mut recording = recording::Recording::create(config)?;
    if let Err(error) = dora_app::environment::ready() {
        return recording.finish(config::State::Failed, Err(eyre!(error)));
    }
    let (state, result) = recording::consume(
        &mut recording,
        std::iter::from_fn(|| block_on(events.next())),
    );
    recording.finish(state, result)
}
