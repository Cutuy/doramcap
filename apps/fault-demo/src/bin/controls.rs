//! Bounded control submission, independently owned and stopped before reporters.
use dora_node_api::{Event, futures::StreamExt};
use eyre::Result;

#[tokio::main(worker_threads = 1)]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or_else(|| eyre::eyre!("missing control socket path"))?;
    let socket = tokio::net::UnixDatagram::bind(path)?;
    let (mut node, mut events) = dora_app::environment::connect()?;
    if args.next().is_some() {
        eyre::bail!("unexpected argument");
    }
    dora_app::environment::ready()?;
    let mut bytes = [0u8; 1024];
    loop {
        tokio::select! {
            received = socket.recv(&mut bytes) => {
                let len = received?;
                node.send_output_raw("controls".into(), Default::default(), len, |output| output.copy_from_slice(&bytes[..len]))?;
            }
            event = events.next() => match event {
                Some(Event::Stop(_)) | None => break,
                Some(Event::Error(error)) => eyre::bail!("Dora control error: {error}"),
                _ => {},
            },
        }
    }
    Ok(())
}
