use workload_holoscan::{BuildRecord, BuildState, Selection, build_record::assess_build_record};
use workload_lifecycle::launch::{SourceRecord, SourceState};

fn source(commit: &str, state: SourceState) -> SourceRecord {
    SourceRecord {
        source_commit: commit.into(),
        source_state: state as i32,
    }
}
fn record(source: Option<SourceRecord>) -> BuildRecord {
    BuildRecord {
        selection: Some(Selection {
            application: "sample".into(),
            language: "Python".into(),
            mode: "headless".into(),
        }),
        image_id: "sha256:built-image".into(),
        source,
    }
}

#[test]
fn only_missing_required_artifact_and_selection_facts_mean_not_built() {
    // Arrange
    let selection = record(None).selection.unwrap();
    let current = source("abc", SourceState::Clean);
    let mut missing_image = record(None);
    missing_image.image_id.clear();
    let mut missing_application = record(None);
    missing_application
        .selection
        .as_mut()
        .unwrap()
        .application
        .clear();
    let mut missing_selection = record(None);
    missing_selection.selection = None;
    // Act / Assert
    for stored in [
        None,
        Some(&missing_image),
        Some(&missing_application),
        Some(&missing_selection),
    ] {
        assert_eq!(
            assess_build_record(stored, &selection, Some(&current)).state,
            BuildState::NotBuilt as i32
        );
    }
    // Act / Assert: absent source evidence is Unverified and does not erase artifact facts.
    assert_eq!(
        assess_build_record(Some(&record(None)), &selection, Some(&current)).state,
        BuildState::Unverified as i32
    );
}

#[test]
fn known_source_and_selection_changes_require_rebuilding() {
    // Arrange
    let stored = record(Some(source("abc", SourceState::Clean)));
    let selection = stored.selection.as_ref().unwrap();
    // Act / Assert: independently known commit changes remain known even with unknown status.
    for current in [
        source("def", SourceState::Clean),
        source("def", SourceState::Unknown),
        source("abc", SourceState::Dirty),
    ] {
        assert_eq!(
            assess_build_record(Some(&stored), selection, Some(&current)).state,
            BuildState::RebuildNeeded as i32
        );
    }
    for (application, language, mode) in [
        ("other", "Python", "headless"),
        ("sample", "C++", "headless"),
        ("sample", "Python", "live"),
    ] {
        let selected = Selection {
            application: application.into(),
            language: language.into(),
            mode: mode.into(),
        };
        assert_eq!(
            assess_build_record(Some(&stored), &selected, stored.source.as_ref()).state,
            BuildState::RebuildNeeded as i32
        );
    }
}

#[test]
fn dirty_unknown_and_missing_commit_facts_stay_unverified_without_invented_states() {
    for (built, current, expected) in [
        (SourceState::Clean, SourceState::Clean, BuildState::Fresh),
        (
            SourceState::Clean,
            SourceState::Dirty,
            BuildState::RebuildNeeded,
        ),
        (
            SourceState::Dirty,
            SourceState::Clean,
            BuildState::RebuildNeeded,
        ),
        (
            SourceState::Dirty,
            SourceState::Dirty,
            BuildState::Unverified,
        ),
        (
            SourceState::Unknown,
            SourceState::Clean,
            BuildState::Unverified,
        ),
        (
            SourceState::Clean,
            SourceState::Unknown,
            BuildState::Unverified,
        ),
        (
            SourceState::Dirty,
            SourceState::Unknown,
            BuildState::Unverified,
        ),
        (
            SourceState::Unknown,
            SourceState::Dirty,
            BuildState::Unverified,
        ),
        (
            SourceState::Unknown,
            SourceState::Unknown,
            BuildState::Unverified,
        ),
    ] {
        // Arrange
        let stored = record(Some(source("abc", built)));
        let observed = source("abc", current);
        // Act / Assert
        assert_eq!(
            assess_build_record(
                Some(&stored),
                stored.selection.as_ref().unwrap(),
                Some(&observed)
            )
            .state,
            expected as i32,
            "{built:?}/{current:?}"
        );
    }
    // Arrange / Act / Assert: commit and status observations remain independent.
    for (built, current) in [("", "abc"), ("abc", ""), ("", "")] {
        let stored = record(Some(source(built, SourceState::Clean)));
        let observed = source(current, SourceState::Clean);
        assert_eq!(
            assess_build_record(
                Some(&stored),
                stored.selection.as_ref().unwrap(),
                Some(&observed)
            )
            .state,
            BuildState::Unverified as i32
        );
    }
}

#[test]
fn diagnostics_name_observed_unknown_and_dirty_facts_separately() {
    // Arrange
    let mut stored = record(Some(source("abc", SourceState::Unknown)));
    let selection = stored.selection.clone().unwrap();
    let current = source("abc", SourceState::Clean);
    // Act / Assert
    let unknown = assess_build_record(Some(&stored), &selection, Some(&current));
    assert!(
        unknown
            .diagnostics
            .iter()
            .any(|message| message.contains("Unknown"))
    );
    assert!(
        unknown
            .diagnostics
            .iter()
            .all(|message| !message.contains("Dirty"))
    );
    stored.source = Some(source("abc", SourceState::Dirty));
    let dirty = source("abc", SourceState::Dirty);
    let unverified = assess_build_record(Some(&stored), &selection, Some(&dirty));
    assert!(
        unverified
            .diagnostics
            .iter()
            .any(|message| message.contains("Dirty"))
    );
    assert!(
        unverified
            .diagnostics
            .iter()
            .all(|message| !message.contains("Unknown"))
    );
    stored.source.as_mut().unwrap().source_state = 99;
    let unsupported = assess_build_record(Some(&stored), &selection, Some(&current));
    assert_eq!(unsupported.state, BuildState::Unverified as i32);
    assert!(
        unsupported
            .diagnostics
            .iter()
            .any(|message| message.contains("99") && message.contains("unrecognized"))
    );
}
