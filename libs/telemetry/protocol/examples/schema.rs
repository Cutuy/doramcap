fn main() -> eyre::Result<()> {
    std::fs::write(
        std::env::args().nth(1).expect("descriptor output path"),
        telemetry_protocol::FILE_DESCRIPTOR_SET,
    )?;
    Ok(())
}
