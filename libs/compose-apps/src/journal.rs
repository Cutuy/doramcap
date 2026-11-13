//! Compose identities saved in the run's shared journal index.
use crate::compose::adapter::ARTIFACTS_LABEL;
use std::{collections::HashMap, path::Path};
use workload_lifecycle::journal::{self, JournalMatchKind, LogState, ProgramLog};

/// Frozen logging policy; explicit project logging is always skipped, including journald.
pub(crate) const EXPORT_LABEL: &str = "io.doramcap.journal-export";

/// Record an owned container during serialized startup or cleanup, including admission reset.
/// Export metadata never changes launch or cleanup success; callers expose index read failures.
pub async fn remember_container(id: &str, labels: &HashMap<String, String>) -> std::io::Result<()> {
    let Some(artifacts) = labels.get(ARTIFACTS_LABEL) else {
        return Ok(());
    };
    let Some(policy) = labels.get(EXPORT_LABEL) else {
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
    let skipped = policy != "true";
    journal::remember(
        Path::new(artifacts),
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
