use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use workload_holoscan::{DeclaredProjectKind, project::read_project_inputs};

struct Project(PathBuf);
impl Project {
    fn new(metadata: Option<&str>, configuration: Option<&str>) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "holoscan-inputs-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        if let Some(text) = metadata {
            fs::write(root.join("metadata.json"), text).unwrap();
        }
        if let Some(text) = configuration {
            fs::write(root.join("pyproject.toml"), text).unwrap();
        }
        Self(root)
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[tokio::test]
async fn source_and_application_declarations_ignore_toml() {
    for (metadata, kind) in [
        (None, DeclaredProjectKind::Source),
        (
            Some(r#"{"application":{"name":"sample"}}"#),
            DeclaredProjectKind::Application,
        ),
        (Some(r#"{"operator":{}}"#), DeclaredProjectKind::Source),
    ] {
        // Arrange
        let project = Project::new(metadata, Some("broken = [ SECRET_SOURCE_CONTENT"));
        // Act
        let inputs = read_project_inputs(&project.0).await.unwrap();
        // Assert
        assert_eq!(inputs.declared_kind, kind as i32);
        assert!(inputs.configuration.is_none());
        assert!(inputs.docker_run_args.is_none());
        assert_eq!(inputs.metadata.is_some(), metadata.is_some());
    }
}

#[tokio::test]
async fn module_arrays_preserve_token_boundaries_whitespace_and_provenance() {
    // Arrange
    let text = r#"
[project]
name = "unrelated"
[tool.other]
value = "untouched"
[tool.holoscan]
docker-build-args = []
docker-run-args = ["  --mount  ", "type=bind,source=${HOME}/with spaces,target=/data", "--label", "greeting=hello world"]
"#;
    let project = Project::new(Some(r#"{"module":{"name":"sample"}}"#), Some(text));
    // Act
    let inputs = read_project_inputs(&project.0).await.unwrap();
    // Assert
    assert_eq!(inputs.declared_kind, DeclaredProjectKind::Module as i32);
    assert_eq!(inputs.declarations, ["module"]);
    assert!(inputs.docker_build_args.unwrap().values.is_empty());
    assert_eq!(
        inputs.docker_run_args.unwrap().values,
        [
            "  --mount  ",
            "type=bind,source=${HOME}/with spaces,target=/data",
            "--label",
            "greeting=hello world"
        ]
    );
    let record = inputs.configuration.unwrap();
    assert_eq!(
        record.path,
        project.0.join("pyproject.toml").to_str().unwrap()
    );
    assert_eq!(
        record.sha256,
        host_filesystem::files::digest(text.as_bytes())
    );
    // Act / Assert: a whole-file identity changes even when only unrelated TOML changes.
    fs::write(
        project.0.join("pyproject.toml"),
        text.replace("untouched", "changed"),
    )
    .unwrap();
    let changed = read_project_inputs(&project.0).await.unwrap();
    assert_ne!(changed.configuration.unwrap().sha256, record.sha256);
}

#[tokio::test]
async fn absent_defaults_and_optional_non_files_are_distinct_from_empty_arrays() {
    // Arrange
    let project = Project::new(Some(r#"{"module":{"name":"sample"}}"#), None);
    // Act / Assert
    assert!(
        read_project_inputs(&project.0)
            .await
            .unwrap()
            .configuration
            .is_none()
    );
    fs::create_dir(project.0.join("pyproject.toml")).unwrap();
    assert!(
        read_project_inputs(&project.0)
            .await
            .unwrap()
            .configuration
            .is_none()
    );
    fs::remove_dir(project.0.join("pyproject.toml")).unwrap();
    fs::write(
        project.0.join("pyproject.toml"),
        "[tool.other]\nname='unused'\n",
    )
    .unwrap();
    let unrelated = read_project_inputs(&project.0).await.unwrap();
    assert!(unrelated.configuration.is_some());
    assert!(unrelated.docker_run_args.is_none());
    fs::write(
        project.0.join("pyproject.toml"),
        "[tool.holoscan]\ndocker-run-args=[]\n",
    )
    .unwrap();
    assert!(
        read_project_inputs(&project.0)
            .await
            .unwrap()
            .docker_run_args
            .unwrap()
            .values
            .is_empty()
    );
}

#[tokio::test]
async fn malformed_inputs_report_constraints_without_source_content() {
    for configuration in [
        "secret='SECRET_SOURCE_CONTENT'\nbroken=[",
        "tool='SECRET_SOURCE_CONTENT'",
        "[tool]\nholoscan='SECRET_SOURCE_CONTENT'",
        "[tool.holoscan]\ndocker-run-args='SECRET_SOURCE_CONTENT'",
        "[tool.holoscan]\ndocker-run-args=['SECRET_SOURCE_CONTENT',1]",
        "[tool.holoscan]\ndocker-build-args=['SECRET_SOURCE_CONTENT','  ']",
        "[tool.holoscan]\ndocker-run-args=[\"\\u001c\"]",
    ] {
        // Arrange
        let project = Project::new(Some(r#"{"module":{"name":"sample"}}"#), Some(configuration));
        // Act
        let error = read_project_inputs(&project.0)
            .await
            .unwrap_err()
            .to_string();
        // Assert
        assert!(error.contains("pyproject.toml"), "{error}");
        assert!(!error.contains("SECRET_SOURCE_CONTENT"), "{error}");
    }
    for metadata in [
        r#"{"module":"SECRET_SOURCE_CONTENT"}"#,
        r#"["SECRET_SOURCE_CONTENT"]"#,
        r#"{"module": {"secret":"SECRET_SOURCE_CONTENT"},"#,
    ] {
        let project = Project::new(Some(metadata), None);
        let error = read_project_inputs(&project.0)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("metadata.json"));
        assert!(!error.contains("SECRET_SOURCE_CONTENT"));
    }
}

#[tokio::test]
async fn multiple_envelopes_are_explicitly_ambiguous_without_claiming_native_context() {
    // Arrange
    let project = Project::new(
        Some(r#"{"application":{},"module":{"name":"sample"}}"#),
        Some("[tool.holoscan]\ndocker-run-args=['--log-driver=journald']\n"),
    );
    // Act
    let inputs = read_project_inputs(&project.0).await.unwrap();
    // Assert
    assert_eq!(inputs.declared_kind, DeclaredProjectKind::Ambiguous as i32);
    assert_eq!(inputs.declarations, ["application", "module"]);
    assert_eq!(
        inputs.docker_run_args.unwrap().values,
        ["--log-driver=journald"]
    );
}
