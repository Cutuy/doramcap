//! Validate a completed storage-demo activity recording.
use std::collections::{BTreeMap, BTreeSet};
fn main() {
    let bytes = std::fs::read(std::env::args().nth(1).expect("activity MCAP path")).unwrap();
    let mut starts = BTreeMap::new();
    let mut ends = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for message in mcap::MessageStream::new(&bytes).unwrap() {
        let batch = activity_tracing::wire::decode(&message.unwrap().data).unwrap();
        assert_eq!(
            batch.dropped_records
                + batch.dropped_fields
                + batch.rejected_spans
                + batch.unregistered_records,
            0
        );
        for r in &batch.records {
            if r.kind == 0 {
                starts.insert(r.span_id, batch.dictionary[r.name as usize].clone());
                if let Some(f) = r
                    .fields
                    .iter()
                    .find(|f| batch.dictionary[f.name as usize] == "path")
                {
                    paths.insert(
                        String::from_utf8(
                            batch.strings[f.offset as usize..(f.offset + f.length) as usize]
                                .to_vec(),
                        )
                        .unwrap(),
                    );
                }
            }
            if r.kind == 4 {
                ends.insert(r.span_id);
            }
        }
    }
    assert_eq!(starts.keys().copied().collect::<BTreeSet<_>>(), ends);
    for path in ["managed", "combined", "large", "slab"] {
        assert!(paths.contains(path), "missing {path}");
    }
    assert!(starts.values().any(|name| name == "storage.combining"));
    assert!(starts.values().any(|name| name == "storage.io"));
    println!(
        "Activity capture: {} closed spans; paths {paths:?}; zero recorded loss",
        starts.len()
    );
}
