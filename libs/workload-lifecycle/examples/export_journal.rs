//! Export a run's saved identities after its writers have stopped.
use workload_lifecycle::journal::{ExportJournal, ExportJournalInput, load};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let artifacts = std::env::args()
        .nth(1)
        .ok_or("usage: export_journal RUN_FOLDER")?;
    let index = load(std::path::Path::new(&artifacts)).await?;
    let mut executor = orchestrator::LocalExecutor::default();
    executor.register::<ExportJournal>(|| ExportJournal);
    executor
        .run::<ExportJournal>(ExportJournalInput {
            artifacts,
            index: Some(index),
        })
        .await?;
    Ok(())
}
