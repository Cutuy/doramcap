use console_native::project::discover;
use std::{fs, os::unix::fs::symlink, path::Path};

#[test]
fn discovers_each_base_and_matching_override_with_explicit_env() {
    // Arrange
    for (base, override_name) in [
        ("compose.yaml", "compose.override.yaml"),
        ("compose.yml", "compose.override.yml"),
        ("docker-compose.yaml", "docker-compose.override.yaml"),
        ("docker-compose.yml", "docker-compose.override.yml"),
    ] {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join(base), "services: {}").unwrap();
        fs::write(temp.path().join(override_name), "services: {}").unwrap();
        fs::write(temp.path().join(".env"), "KEY=value").unwrap();
        // Act
        let project = discover(temp.path(), temp.path(), vec!["extra".into()]).unwrap();
        // Assert
        assert_eq!(project.files, [Path::new(base), Path::new(override_name)]);
        assert_eq!(project.env_files, [Path::new(".env")]);
        assert_eq!(project.profiles, ["extra"]);
        assert!(project.environment.is_empty() && project.services.is_empty());
    }
}

#[test]
fn rejects_missing_and_ambiguous_defaults_and_overrides() {
    // Arrange
    let temp = tempfile::tempdir().unwrap();
    // Act / Assert
    assert!(discover(temp.path(), temp.path(), vec![]).is_err());
    fs::write(temp.path().join("compose.yaml"), "").unwrap();
    fs::write(temp.path().join("compose.yml"), "").unwrap();
    assert!(discover(temp.path(), temp.path(), vec![]).is_err());
    fs::remove_file(temp.path().join("compose.yml")).unwrap();
    fs::write(temp.path().join("compose.override.yaml"), "").unwrap();
    fs::write(temp.path().join("compose.override.yml"), "").unwrap();
    assert!(discover(temp.path(), temp.path(), vec![]).is_err());
}

#[test]
fn rejects_outside_directory_and_symlink_escape() {
    // Arrange
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("compose.yaml"), "").unwrap();
    symlink(outside.path(), root.path().join("escape")).unwrap();
    // Act / Assert
    assert!(discover(root.path(), Path::new("."), vec![]).is_err());
    assert!(discover(root.path(), outside.path(), vec![]).is_err());
    assert!(discover(root.path(), &root.path().join("escape"), vec![]).is_err());
}
