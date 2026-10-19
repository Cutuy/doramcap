fn main() -> eyre::Result<()> {
    std::fs::write(
        std::env::args().nth(1).expect("descriptor output path"),
        monitor_wire::FILE_DESCRIPTOR_SET,
    )?;
    Ok(())
}
