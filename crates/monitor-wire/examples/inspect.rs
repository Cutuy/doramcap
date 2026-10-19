// Compare a captured binary frame against the exact previous JSON representation.
use serde::Serialize;
#[derive(Serialize)]
struct Legacy<'a> {
    #[serde(flatten)]
    snapshot: &'a monitor_core::Snapshot,
    source: &'a str,
    task_collection_enabled: bool,
    task_capacity: usize,
    session_id: &'a str,
    dictionary_revision: u64,
    dictionary_full: bool,
    dictionary_overflow: u64,
    dictionary: &'a [monitor_wire::DictionaryEntry],
}
fn main() -> eyre::Result<()> {
    let bytes = std::fs::read(std::env::args().nth(1).expect("binary frame path"))?;
    let mut f = monitor_wire::decode(&bytes)?;
    // Compare identical self-contained snapshots without viewer-only metrics.
    f.viewer_ingress = None;
    let mut encoded = Vec::new();
    monitor_wire::encode(&f, &mut encoded)?;
    let legacy = Legacy {
        snapshot: &f.snapshot,
        source: &f.source,
        task_collection_enabled: f.task_collection_enabled,
        task_capacity: f.task_capacity,
        session_id: &f.session_id,
        dictionary_revision: f.dictionary_revision,
        dictionary_full: f.dictionary_full,
        dictionary_overflow: f.dictionary_overflow,
        dictionary: &f.dictionary,
    };
    let json = serde_json::to_vec(&legacy)?;
    println!(
        "same snapshot: binary={} bytes, previous JSON={} bytes, reduction={:.1}%",
        encoded.len(),
        json.len(),
        100.0 * (1.0 - encoded.len() as f64 / json.len() as f64)
    );
    Ok(())
}
