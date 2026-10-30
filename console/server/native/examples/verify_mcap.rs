//! Validate one Console CPU recording for the acceptance harness.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os().nth(1).ok_or("expected MCAP path")?;
    let bytes = std::fs::read(path)?;
    mcap::read::footer(&bytes)?;
    let mut session = None;
    let mut expected = 0_u64;
    for message in mcap::MessageStream::new(&bytes)? {
        let message = message?;
        if message.channel.topic != "doramcap/snapshots" {
            continue;
        }
        let frame = telemetry_protocol::decode(&message.data)?;
        if frame.source != "bpf"
            || frame.snapshot.dropped_batches != 0
            || frame.snapshot.sequence != expected
        {
            return Err("capture contains an unexpected source, loss or sequence gap".into());
        }
        match &session {
            Some(previous) if previous != &frame.session_id => {
                return Err("capture session changed".into());
            }
            None => session = Some(frame.session_id),
            _ => {}
        }
        expected += 1;
    }
    if expected < 3 {
        return Err("fewer than three snapshots".into());
    }
    println!(
        "session={} snapshots={expected} footer=valid",
        session.unwrap()
    );
    Ok(())
}
