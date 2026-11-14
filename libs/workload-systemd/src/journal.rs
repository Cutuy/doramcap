//! systemd invocation identities, with a runtime pointer to the single durable index.
use crate::{Scope, manager::Unit};
use std::path::Path;
use workload_lifecycle::journal::{self, JournalMatchKind, LogState, ProgramLog};

pub(crate) const POINTER: &str = "journal-run";

/// Read uncached invocation IDs while the run still owns its unit references.
pub(crate) async fn remember(artifacts: &Path, scope: &Scope, units: &[Unit]) {
    let mut programs = Vec::new();
    for unit in units {
        let identity = unit.unit.invocation_id().await.unwrap_or_default();
        let identity = if identity.len() == 16 && identity.iter().any(|byte| *byte != 0) {
            hex::encode(identity)
        } else {
            String::new()
        };
        programs.push(ProgramLog {
            program_id: unit
                .name
                .strip_prefix(&format!("{}-", scope.prefix))
                .and_then(|name| name.strip_suffix(".service"))
                .unwrap_or(&unit.name)
                .into(),
            match_kind: JournalMatchKind::SystemdInvocation.into(),
            identity,
            state: LogState::Pending.into(),
            ..Default::default()
        });
    }
    // A storage failure must not turn a successful program into a failed run.
    let _ = journal::remember(artifacts, programs).await;
}
