use orchestrator::Step;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use workload_holoscan::{
    Catalog, CatalogOutcome, DiscoverApplications, DiscoverRequest, query_diagnostic::Termination,
};

struct Fixture {
    root: PathBuf,
    cli: PathBuf,
}
impl Fixture {
    fn new(script: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "holoscan-discovery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let cli = root.join("holoscan");
        fs::write(&cli, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&cli, fs::Permissions::from_mode(0o700)).unwrap();
        Self { root, cli }
    }
    async fn discover(&self) -> Catalog {
        DiscoverApplications
            .run(DiscoverRequest {
                cli: self.cli.to_str().unwrap().into(),
                project_root: self.root.to_str().unwrap().into(),
            })
            .await
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[tokio::test]
async fn catalog_preserves_native_origins_languages_modes_and_query_diagnostics() {
    // Arrange: CLI5 list/modes envelopes, including a component and upstream metadata diagnostic.
    let fixture = Fixture::new(
        r#"
[ "$1" = --project-root ] && [ "$2" = "$PWD" ] || exit 8
if [ "$3" = list ]; then
    echo 'metadata.json: malformed metadata retained as a diagnostic' >&2
    printf '{"schema_version":1,"projects":[{"name":"demo","project_type":"application","source_folder":"%s/app","language":["C++","Python"],"modes":["headless"]},{"name":"filter","project_type":"operator","source_folder":"%s/operator","language":["C++"],"modes":[]}]}' "$PWD" "$PWD"
elif [ "$3" = modes ] && [ "$4" = demo ] && [ "$5" = --language ]; then
    case "$6" in cpp) lang='C++';; python) lang='Python';; *) exit 9;; esac
    printf '{"schema_version":1,"project":"demo","language":["%s"],"modes":{"headless":{"description":"Finite headless execution","run":{"command":"native command"}}}}' "$lang"
else exit 10
fi
"#,
    );
    // Act
    let catalog = fixture.discover().await;
    // Assert
    assert_eq!(catalog.outcome, CatalogOutcome::Available as i32);
    assert_eq!(catalog.entries.len(), 2);
    assert_eq!(
        catalog.entries[0].source_folder,
        fixture.root.join("app").to_str().unwrap()
    );
    assert_eq!(
        catalog.entries[0]
            .variants
            .iter()
            .map(|item| item.language.as_str())
            .collect::<Vec<_>>(),
        ["C++", "Python"]
    );
    assert_eq!(
        catalog.entries[0].variants[1].modes[0].description,
        "Finite headless execution"
    );
    assert_eq!(catalog.entries[1].project_type, "operator");
    assert_eq!(catalog.diagnostics.len(), 3);
    assert!(String::from_utf8_lossy(&catalog.diagnostics[0].stderr).contains("malformed metadata"));
    assert!(String::from_utf8_lossy(&catalog.diagnostics[1].stdout).contains("native command"));
}

#[tokio::test]
async fn default_language_and_no_modes_keep_the_upstream_default() {
    // Arrange
    let fixture = Fixture::new(
        r#"
if [ "$3" = list ]; then
    printf '{"schema_version":1,"projects":[{"name":"demo","project_type":"application","source_folder":"%s/app","language":[],"modes":[]}]}' "$PWD"
else
    [ "$5" = --json ] || exit 12
    echo '{"schema_version":1,"project":"demo","language":[],"modes":{}}'
fi
"#,
    );
    // Act
    let catalog = fixture.discover().await;
    // Assert
    assert_eq!(catalog.outcome, CatalogOutcome::Available as i32);
    assert_eq!(catalog.entries[0].variants.len(), 1);
    assert!(catalog.entries[0].variants[0].language.is_empty());
    assert!(catalog.entries[0].variants[0].modes.is_empty());
}

#[tokio::test]
async fn empty_unavailable_failed_and_unreadable_queries_remain_distinct() {
    for (script, outcome) in [
        (
            "echo '{\"schema_version\":1,\"projects\":[]}'",
            CatalogOutcome::Empty,
        ),
        (
            "echo 'error: unrecognized arguments: --json' >&2; exit 2",
            CatalogOutcome::Unavailable,
        ),
        (
            "echo \"argument command: invalid choice: 'modes'\" >&2; exit 2",
            CatalogOutcome::Unavailable,
        ),
        (
            "echo \"argument --language: invalid choice: 'java'\" >&2; exit 2",
            CatalogOutcome::Failed,
        ),
        (
            "echo output; echo 'project directory error' >&2; exit 1",
            CatalogOutcome::Failed,
        ),
        ("echo not-json", CatalogOutcome::Failed),
        (
            "echo '{\"schema_version\":2,\"projects\":[]}'",
            CatalogOutcome::Failed,
        ),
    ] {
        // Arrange
        let fixture = Fixture::new(script);
        // Act
        let catalog = fixture.discover().await;
        // Assert
        assert_eq!(
            catalog.outcome, outcome as i32,
            "{script}: {}",
            catalog.error
        );
        assert_eq!(catalog.diagnostics.len(), 1);
        assert!(catalog.diagnostics[0].termination.is_some());
        assert!(
            !catalog.diagnostics[0].stdout.is_empty() || !catalog.diagnostics[0].stderr.is_empty()
        );
    }
    // Arrange
    let fixture = Fixture::new("exit 0");
    fs::remove_file(&fixture.cli).unwrap();
    // Act
    let catalog = fixture.discover().await;
    // Assert
    assert_eq!(catalog.outcome, CatalogOutcome::Unavailable as i32);
    assert!(catalog.diagnostics[0].termination.is_none());
    assert!(!catalog.diagnostics[0].error.is_empty());
}

#[tokio::test]
async fn a_failed_modes_query_keeps_the_catalog_and_its_process_output() {
    // Arrange
    let fixture = Fixture::new(
        r#"
if [ "$3" = list ]; then
    printf '{"schema_version":1,"projects":[{"name":"demo","project_type":"application","source_folder":"%s/app","language":["Python"],"modes":[]}]}' "$PWD"
else
    echo partial-output
    echo 'application unavailable after catalog' >&2
    exit 1
fi
"#,
    );
    // Act
    let catalog = fixture.discover().await;
    // Assert
    assert_eq!(catalog.outcome, CatalogOutcome::Failed as i32);
    assert_eq!(catalog.entries[0].name, "demo");
    assert_eq!(
        catalog.diagnostics[1].termination,
        Some(Termination::ExitCode(1))
    );
    assert_eq!(catalog.diagnostics[1].stdout, b"partial-output\n");
    assert_eq!(
        catalog.diagnostics[1].stderr,
        b"application unavailable after catalog\n"
    );
}
