//! Asserts two recordings hold equal payloads for one topic and the expected count.
use std::collections::BTreeMap;
fn read(path: &str, topic: &str) -> BTreeMap<(u32, u64), Vec<u8>> {
    let bytes = std::fs::read(path).unwrap();
    let mut records = BTreeMap::new();
    for message in mcap::MessageStream::new(&bytes).unwrap() {
        let message = message.unwrap();
        if message.channel.topic == topic {
            // Publish time is source time in both writers; log time may be receipt time.
            let key = (message.sequence, message.publish_time);
            assert!(
                records.insert(key, message.data.to_vec()).is_none(),
                "duplicate record"
            );
        }
    }
    records
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 5, "compare_recordings FIRST SECOND TOPIC COUNT");
    let first = read(&args[1], &args[3]);
    assert_eq!(first, read(&args[2], &args[3]), "recorded messages differ");
    assert_eq!(first.len(), args[4].parse::<usize>().unwrap());
    println!(
        "Recording equality: {} messages on {}",
        first.len(),
        args[3]
    );
}
