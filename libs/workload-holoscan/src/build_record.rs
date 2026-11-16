//! Build-record assessment using supplied source facts; capture and Engine observations have other owners.
use crate::{BuildAssessment, BuildRecord, BuildState, Selection};
use workload_lifecycle::launch::{SourceRecord, SourceState};

/// Assess stored artifact/selection facts and caller-supplied current source facts.
/// Unknown/dirty observations remain Unverified unless independent known facts changed.
/// This function makes no Git or Engine query and does not decide image availability.
///
/// # Scope
/// The caller supplies a record stored for the same canonical project root as admission.
/// Equal selections from different roots do not identify the same Build. Source facts are current
/// observations, without a cached-commit fallback; absent source evidence is valid input.
///
/// # Outcomes
/// Missing required image/selection facts are NotBuilt; absent Git evidence is Unverified.
/// Known selection/commit/clean-state changes need rebuilding; matching known Clean facts are Fresh.
/// Unverified alone does not block preparation. Callers separately check actual image identity/availability
/// and preserve Engine errors separately from a missing image.
pub fn assess_build_record(
    record: Option<&BuildRecord>,
    selection: &Selection,
    current: Option<&SourceRecord>,
) -> BuildAssessment {
    let Some(record) = record else {
        return assessment(BuildState::NotBuilt, "No Build record is available");
    };
    if record.image_id.is_empty()
        || record
            .selection
            .as_ref()
            .is_none_or(|selection| selection.application.is_empty())
    {
        return assessment(
            BuildState::NotBuilt,
            "Build record has no required image or application identity",
        );
    }
    if record.selection.as_ref() != Some(selection) {
        return assessment(
            BuildState::RebuildNeeded,
            "Application, language or mode differs from Build",
        );
    }
    let unknown = SourceRecord::default();
    let built = record.source.as_ref().unwrap_or(&unknown);
    let current = current.unwrap_or(&unknown);
    if !built.source_commit.is_empty()
        && !current.source_commit.is_empty()
        && built.source_commit != current.source_commit
    {
        return assessment(
            BuildState::RebuildNeeded,
            "Source commit changed since Build",
        );
    }
    let known = |state| {
        matches!(
            SourceState::try_from(state),
            Ok(SourceState::Clean | SourceState::Dirty)
        )
    };
    if known(built.source_state)
        && known(current.source_state)
        && built.source_state != current.source_state
    {
        return assessment(
            BuildState::RebuildNeeded,
            "Source clean state changed since Build",
        );
    }
    let mut diagnostics = Vec::new();
    for (name, source) in [("Build", built), ("Current", current)] {
        if source.source_commit.is_empty() {
            diagnostics.push(format!(
                "{name} source commit was unavailable; freshness is unverified"
            ));
        }
        match SourceState::try_from(source.source_state) {
            Ok(SourceState::Clean) => {}
            Ok(SourceState::Dirty) => {
                diagnostics.push(format!("{name} source is Dirty; freshness is unverified"))
            }
            Ok(SourceState::Unknown) => diagnostics.push(format!(
                "{name} source state is Unknown; freshness is unverified"
            )),
            Err(_) => diagnostics.push(format!(
                "{name} source state {} is unrecognized; freshness is unverified",
                source.source_state
            )),
        }
    }
    if !diagnostics.is_empty() {
        return BuildAssessment {
            state: BuildState::Unverified as i32,
            diagnostics,
        };
    }

    assessment(BuildState::Fresh, "Known clean source facts match Build")
}
fn assessment(state: BuildState, diagnostic: &str) -> BuildAssessment {
    BuildAssessment {
        state: state as i32,
        diagnostics: vec![diagnostic.into()],
    }
}
