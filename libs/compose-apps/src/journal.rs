//! Compose identities saved in the run's shared journal index.
use docker_engine::docker_proto::ResourceLabels;
use std::{collections::HashMap, path::Path};
use workload_lifecycle::journal::{self, JournalMatchKind, LogState, ProgramLog};

/// Frozen logging policy; explicit project logging is always skipped, including journald.
pub(crate) use docker_engine::labels::JOURNAL_EXPORT_LABEL as EXPORT_LABEL;

/// Record an owned container during serialized startup or cleanup, including admission reset.
/// Export metadata never changes launch or cleanup success; callers expose index read failures.
pub async fn remember_container(id: &str, labels: &HashMap<String, String>) -> std::io::Result<()> {
    let metadata = ResourceLabels::from_docker_labels(labels);
    let Some(artifacts) = metadata.artifacts else {
        return Ok(());
    };
    let Some(export) = metadata.journal_export else {
        return Ok(());
    };
    let service = labels
        .get("com.docker.compose.service")
        .map(String::as_str)
        .unwrap_or("unknown");
    let project = labels
        .get("com.docker.compose.project")
        .map(String::as_str)
        .unwrap_or("unknown");
    let replica = labels
        .get("com.docker.compose.container-number")
        .map(String::as_str)
        .unwrap_or("1");
    let skipped = !export;
    journal::remember(
        Path::new(&artifacts),
        [ProgramLog {
            program_id: format!("{project}/{service}/{replica}"),
            match_kind: JournalMatchKind::Container.into(),
            identity: id.into(),
            state: if skipped {
                LogState::Skipped
            } else {
                LogState::Pending
            }
            .into(),
            reason: if skipped {
                "Logging configured by project".into()
            } else {
                String::new()
            },
            ..Default::default()
        }],
    )
    .await
}
