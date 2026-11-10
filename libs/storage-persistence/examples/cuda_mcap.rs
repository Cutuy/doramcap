//! CUDA-produced MCAP slabs: mapped output, staged device output, and storage-only baseline.
#[path = "support/cuda_mcap.rs"]
mod support;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if !(5..=6).contains(&args.len()) {
        return Err(
            "usage: cuda_mcap OUTPUT.mcap mapped|staged|io-only PAYLOAD_BYTES MESSAGES [SLABS=4]"
                .into(),
        );
    }
    let stats = support::run(
        std::path::Path::new(&args[1]),
        support::Mode::parse(&args[2])?,
        args[3].parse()?,
        args[4].parse()?,
        args.get(5).map_or(Ok(4), |s| s.parse())?,
    )?;
    stats.print();
    Ok(())
}
