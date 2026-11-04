//! Test participant publishing arbitrary bytes and a reproducible health transition.
use dora_node_api::{Event, TryRecvError};
use fault_dora::Health;
use std::time::{Duration, Instant};

fn main() -> eyre::Result<()> {
    let (mut node, mut events) = dora_app::environment::connect()?;
    let mut health = Health::new()?;
    let check = health.register("validation", "write", None)?;
    let start = Instant::now();
    dora_app::environment::ready()?;
    loop {
        match events.try_recv() {
            Ok(Event::Stop(_)) | Err(TryRecvError::Closed) => break,
            Ok(Event::Error(error)) => eyre::bail!(error),
            _ => {}
        }
        let elapsed = start.elapsed();
        if elapsed >= Duration::from_secs(1) && elapsed < Duration::from_secs(2) {
            Health::failure(&check, "validation disk full")?;
        } else {
            check.success()?;
        }
        node.send_output_raw("messages".into(), Default::default(), 3, |out| {
            out.copy_from_slice(&[0, 1, 255])
        })?;
        // Continue data while health pauses, so watchdog staleness is observable.
        if elapsed < Duration::from_secs(3) || elapsed >= Duration::from_secs(7) {
            health.tick(&mut node)?;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    health.finish_to(&mut node, "health")
}
