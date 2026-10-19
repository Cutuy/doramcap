use std::collections::BTreeMap;
fn read(path: &str) -> BTreeMap<(String, u32, u64), Vec<u8>> {
    let bytes = std::fs::read(path).unwrap();
    let mut records = BTreeMap::new();
    for message in mcap::MessageStream::new(&bytes).unwrap() {
        let message = message.unwrap();
        let key = (
            message.channel.topic.clone(),
            message.sequence,
            message.log_time,
        );
        assert!(
            records.insert(key, message.data.to_vec()).is_none(),
            "duplicate record"
        );
    }
    records
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    let embedded = read(&args[1]);
    let received = read(&args[2]);
    assert_eq!(embedded, received, "published and embedded records differ");
    let count = embedded
        .keys()
        .filter(|(topic, _, _)| topic == "doramcap/snapshots")
        .count();
    assert_eq!(count, 24);
    println!(
        "Dora publication/recording equality: {count} snapshots, {} total messages",
        embedded.len()
    );
}
