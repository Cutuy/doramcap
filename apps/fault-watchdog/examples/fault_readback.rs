//! Validate an end-to-end recording with the independent MCAP reader.
fn main() -> eyre::Result<()> {
    let paths: Vec<_> = std::env::args_os().skip(1).collect();
    eyre::ensure!(!paths.is_empty(), "usage: fault_readback FILE...");
    let mut count = 0;
    let mut snapshot = false;
    let mut active = false;
    for path in paths {
        let bytes = std::fs::read(path)?;
        for message in mcap::MessageStream::new(&bytes)? {
            let message = message?;
            let schema = message
                .channel
                .schema
                .as_ref()
                .ok_or_else(|| eyre::eyre!("missing schema"))?;
            eyre::ensure!(
                schema.name == fault_protocol::AGGREGATION_SCHEMA_NAME,
                "schema mismatch"
            );
            eyre::ensure!(
                schema.data.as_ref() == fault_protocol::FILE_DESCRIPTOR_SET,
                "descriptor mismatch"
            );
            let frame = fault_protocol::decode_aggregation(&message.data, Default::default())?;
            match frame.payload {
                fault_protocol::AggregationPayload::Snapshot(_) => snapshot = true,
                fault_protocol::AggregationPayload::Updates(b) => {
                    active |= b.updates.iter().any(|u| {
                        u.state
                            .is_some_and(|s| s.health == fault_aggregation::Health::Active)
                    });
                }
            }
            count += 1;
        }
    }
    eyre::ensure!(
        count > 0 && snapshot && active,
        "missing fault history/snapshot"
    );
    println!("MCAP: {count} valid publications, active fault and snapshot");
    Ok(())
}
