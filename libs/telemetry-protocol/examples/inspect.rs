fn main() -> eyre::Result<()> {
    let bytes = std::fs::read(std::env::args().nth(1).expect("protobuf frame path"))?;
    let f = telemetry_protocol::decode(&bytes)?;
    println!(
        "protobuf={} bytes, cpu_records={}, task_records={}, inline_dictionary={}",
        bytes.len(),
        f.snapshot.records.len(),
        f.snapshot.task_records.len(),
        f.dictionary.len()
    );
    Ok(())
}
