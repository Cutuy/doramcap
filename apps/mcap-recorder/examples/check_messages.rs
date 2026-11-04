//! Acceptance readback for arbitrary user bytes, independent of recorder internals.
fn main() -> eyre::Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or_else(|| eyre::eyre!("usage: check_messages FILE"))?;
    let topic = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "user/messages".into());
    let bytes = std::fs::read(path)?;
    let mut count = 0;
    for message in mcap::MessageStream::new(&bytes)? {
        let message = message?;
        if message.channel.topic == topic {
            if topic == "user/messages" {
                eyre::ensure!(message.data.as_ref() == [0, 1, 255], "user bytes changed");
            } else {
                eyre::ensure!(!message.data.is_empty(), "empty system publication");
            }
            count += 1;
        }
    }
    eyre::ensure!(count > 0, "no user messages recorded");
    println!("MCAP: {count} messages on {topic}");
    Ok(())
}
