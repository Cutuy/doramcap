//! Trusted, declarative application registrations and Compose launch settings.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};
use workload_docker::compose::ComposeProject;

mod proto {
    include!(concat!(env!("OUT_DIR"), "/doramcap.hosting.v1.rs"));
}
pub use proto::{AppRegistration, ArtifactCollection, RecordingCount, ResultFormat};

// Keep the existing private checkpoint representation during configuration migration.
mod result_format_serde {
    use super::ResultFormat;
    use serde::{Deserialize, Serialize};
    pub fn serialize<S: serde::Serializer>(value: &i32, serializer: S) -> Result<S::Ok, S::Error> {
        ResultFormat::try_from(*value)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<i32, D::Error> {
        ResultFormat::deserialize(deserializer).map(i32::from)
    }
}

impl ArtifactCollection {
    pub(crate) fn matches_run(&self, name: &str) -> bool {
        if self.generation_runs {
            return name
                .strip_prefix("gal-")
                .and_then(|value| value.split_once('-'))
                .is_some_and(|(generation, number)| {
                    run_directory(generation)
                        && !number.is_empty()
                        && !number.starts_with('0')
                        && number.len() <= 3
                        && number.bytes().all(|b| b.is_ascii_digit())
                });
        }
        match &self.run_prefix {
            None => run_directory(name),
            Some(prefix) => name.strip_prefix(prefix).is_some_and(|suffix| {
                !suffix.is_empty()
                    && suffix
                        .split('-')
                        .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
                    && simple_name(name)
            }),
        }
    }
    pub(crate) fn matches(&self, name: &str) -> bool {
        if self.base_file.as_deref() == Some(name) {
            return true;
        }
        name.strip_prefix(&self.prefix)
            .and_then(|value| value.strip_suffix(&self.suffix))
            .is_some_and(|number| {
                (number.len() == self.digits as usize
                    || (self.variable_digits && (1..=self.digits as usize).contains(&number.len())))
                    && number.bytes().all(|b| b.is_ascii_digit())
            })
    }
    pub(crate) fn valid(&self) -> bool {
        let fragment = |s: &str| {
            s.len() <= 100
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        };
        identifier(&self.id)
            && ResultFormat::try_from(self.format).is_ok()
            && !self.prefix.is_empty()
            && fragment(&self.prefix)
            && fragment(&self.suffix)
            && (1..=16).contains(&self.digits)
            && (1..=512).contains(&self.max_files)
            && self.base_file.as_deref().is_none_or(simple_name)
            && self.directory.as_deref().is_none_or(simple_name)
            && self.run_prefix.as_deref().is_none_or(|prefix| {
                self.run_directories && identifier(prefix) && prefix.ends_with('-')
            })
            && (!self.generation_runs || (self.run_directories && self.run_prefix.is_none()))
            && self.prefix.len() + self.digits as usize + self.suffix.len() <= 255
    }
}

/// Validated protobuf configuration with its loader-owned deployment directory.
/// The directory is runtime context and cannot be supplied in a registration.
#[derive(Clone, Debug)]
pub struct Registration {
    pub directory: PathBuf,
    /// Canonical app directory defining this gallery app's independent experiment scope.
    pub repository_root: PathBuf,
    pub config: AppRegistration,
}
impl std::ops::Deref for Registration {
    type Target = AppRegistration;
    fn deref(&self) -> &Self::Target {
        &self.config
    }
}
impl std::ops::DerefMut for Registration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.config
    }
}
impl Registration {
    /// Parse one protobuf text registration. Unknown fields and malformed text fail.
    /// The caller validates semantic constraints before admitting the registration.
    fn parse(text: &str, directory: PathBuf) -> Result<Self, String> {
        use prost_reflect::{DescriptorPool, DynamicMessage};
        static POOL: std::sync::OnceLock<DescriptorPool> = std::sync::OnceLock::new();
        let pool = POOL.get_or_init(|| {
            DescriptorPool::decode(
                include_bytes!(concat!(env!("OUT_DIR"), "/registration.bin")).as_slice(),
            )
            .expect("compiled registration descriptor")
        });
        let descriptor = pool
            .get_message_by_name("doramcap.hosting.v1.AppRegistration")
            .expect("compiled registration message");
        let config = DynamicMessage::parse_text_format(descriptor, text)
            .map_err(|e| e.to_string())?
            .transcode_to()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            repository_root: directory.parent().unwrap_or(&directory).to_owned(),
            directory,
            config,
        })
    }
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
pub(crate) fn simple_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        && matches!(
            Path::new(value).components().collect::<Vec<_>>().as_slice(),
            [Component::Normal(_)]
        )
}

pub(crate) fn artifact_path(value: &str) -> bool {
    let parts: Vec<_> = value.split('/').collect();
    (1..=3).contains(&parts.len()) && parts.iter().all(|part| simple_name(part))
}

pub(crate) fn run_directory(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl Registration {
    pub(crate) fn environment_key(&self, suffix: &str) -> String {
        format!("{}_{suffix}", self.env_prefix)
    }

    fn validate(&self) -> Result<(), String> {
        if let Some(config) = &self.system_apps {
            for flow in &config.dora_flows {
                workload_docker::workflows::dora::validate(flow).map_err(|e| e.to_string())?;
                if flow.project != self.id || !flow.results_directory.is_empty() {
                    return Err("Dora bindings must use the registered project and manager-owned results path".into());
                }
            }
        }
        if !identifier(&self.id) {
            return Err("invalid app id".into());
        }
        if !simple_name(&self.compose_file)
            || !simple_name(&self.service)
            || !simple_name(&self.network)
            || !simple_name(&self.result_file)
            || self
                .capture_directory
                .as_deref()
                .is_some_and(|value| !simple_name(value))
            || self.shutdown_marker.as_deref().is_some_and(|value| {
                !simple_name(value)
                    || value == self.result_file
                    || self.capture_directory.as_deref() == Some(value)
            })
        {
            return Err("invalid file, directory, service or network name".into());
        }
        if self
            .port
            .is_some_and(|port| port == 0 || port > u32::from(u16::MAX))
            || ResultFormat::try_from(self.result_format).is_err()
            || (!self.headless && self.port.is_none())
            || (self.headless && self.port.is_some())
            || (self.result_format() != ResultFormat::Mcap && self.capture_directory.is_some())
            || self.additional_files.len() > 32
            || self.additional_files.iter().any(|name| {
                !simple_name(name)
                    || name == &self.result_file
                    || self.capture_directory.as_deref() == Some(name.as_str())
                    || self.shutdown_marker.as_deref() == Some(name.as_str())
                    || self
                        .recording_count
                        .as_ref()
                        .is_some_and(|count| count.count_file == *name)
            })
            || self.additional_files.iter().collect::<BTreeSet<_>>().len()
                != self.additional_files.len()
            || self.artifact_collections.len() > 8
            || self.artifact_collections.iter().any(|collection| {
                !collection.valid()
                    || collection.matches(&self.result_file)
                    || self
                        .capture_directory
                        .as_deref()
                        .is_some_and(|name| collection.matches(name))
                    || self
                        .shutdown_marker
                        .as_deref()
                        .is_some_and(|name| collection.matches(name))
                    || self
                        .recording_count
                        .as_ref()
                        .is_some_and(|check| collection.matches(&check.count_file))
                    || self
                        .additional_files
                        .iter()
                        .any(|name| collection.matches(name))
            })
            || self
                .artifact_collections
                .iter()
                .enumerate()
                .any(|(index, a)| {
                    self.artifact_collections[index + 1..].iter().any(|b| {
                        a.id == b.id
                            || ((a.prefix.starts_with(&b.prefix)
                                || b.prefix.starts_with(&a.prefix))
                                && (a.suffix.ends_with(&b.suffix) || b.suffix.ends_with(&a.suffix)))
                            || a.base_file.as_deref().is_some_and(|name| b.matches(name))
                            || b.base_file.as_deref().is_some_and(|name| a.matches(name))
                    })
                })
            || self.recording_count.as_ref().is_some_and(|check| {
                self.result_format() != ResultFormat::Mcap
                    || self.result_optional
                    || !simple_name(&check.count_file)
                    || check.count_file == self.result_file
                    || self.shutdown_marker.as_deref() == Some(check.count_file.as_str())
                    || self.capture_directory.as_deref() == Some(check.count_file.as_str())
                    || check.topic.is_empty()
                    || check.topic.len() > 256
            })
            || self.readiness_seconds == 0
            || self.stop_seconds == 0
            || self.private_directories.len() > 4
            || self.private_directories.iter().any(|name| {
                name.is_empty()
                    || name.len() > 32
                    || !name.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
                    || !name
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            })
            || self
                .private_directories
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != self.private_directories.len()
            || self.env_prefix.is_empty()
            || !self
                .env_prefix
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_uppercase())
            || !self
                .env_prefix
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            || self.exclusive_resources.iter().any(|v| !simple_name(v))
        {
            return Err("invalid port, timeout, environment prefix or resource".into());
        }
        Ok(())
    }
    /// Construct trusted Compose input for an absolute fresh result directory.
    /// The caller creates and owns that directory. The returned environment is
    /// derived solely from the registration and supplied UID/GID, not the client.
    pub fn project(
        &self,
        results: &Path,
        uid: u32,
        gid: u32,
    ) -> Result<ComposeProject, &'static str> {
        if !self.directory.is_absolute() || !results.is_absolute() {
            return Err("registration and result paths must be absolute");
        }
        let generation = results.parent().ok_or("result path lacks a parent")?;
        let results = results.to_str().ok_or("result path is not UTF-8")?;
        let mut environment = BTreeMap::from([
            (format!("{}_UID", self.env_prefix), uid.to_string()),
            (format!("{}_GID", self.env_prefix), gid.to_string()),
            (
                format!("{}_RESULTS_DIR", self.env_prefix),
                results.to_owned(),
            ),
        ]);
        for name in &self.private_directories {
            let path = generation.join("private").join(name);
            environment.insert(
                format!(
                    "{}_PRIVATE_{}_DIR",
                    self.env_prefix,
                    name.to_ascii_uppercase()
                ),
                path.to_str()
                    .ok_or("private directory is not UTF-8")?
                    .to_owned(),
            );
        }
        Ok(ComposeProject {
            key: self.id.clone(),
            directory: self
                .directory
                .to_str()
                .ok_or("registration directory is not UTF-8")?
                .to_owned(),
            files: vec![self.compose_file.clone()],
            env_files: vec![],
            environment,
            profiles: vec![],
            services: std::iter::once(self.service.clone())
                .chain(
                    self.system_apps.iter().flat_map(|config| {
                        config.dora_flows.iter().map(|flow| flow.service.clone())
                    }),
                )
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect(),
            completed_services: vec![],
            remove_volumes: false,
        })
    }
}

/// Registry loaded from trusted wrapper directories at server startup.
#[derive(Clone, Debug)]
pub struct Registry {
    apps: BTreeMap<String, Registration>,
}
impl Registry {
    /// Load default-enabled manifests from each app's deployment directory.
    pub fn new(workspace: &Path) -> Result<Self, String> {
        Self::load(workspace, &[])
    }
    /// Load registrations, explicitly enabling IDs listed in `enabled`.
    /// Rejects malformed manifests, duplicate IDs and unknown requested IDs.
    pub fn load(workspace: &Path, enabled: &[String]) -> Result<Self, String> {
        let mut registry = Self::load_all(workspace)?;
        for id in enabled {
            if !registry.apps.contains_key(id) {
                return Err(format!("unknown app requested for enablement: {id}"));
            }
        }
        let enabled: BTreeSet<_> = enabled.iter().map(String::as_str).collect();
        registry
            .apps
            .retain(|id, app| !app.requires_explicit_enable || enabled.contains(id.as_str()));
        Ok(registry)
    }
    /// Load all registrations, including apps requiring opt-in.
    pub fn load_all(workspace: &Path) -> Result<Self, String> {
        if !workspace.is_absolute() {
            return Err("workspace path must be absolute".into());
        }
        let root = workspace.join("apps");
        let entries = fs::read_dir(&root).map_err(|e| format!("read {}: {e}", root.display()))?;
        let mut apps = BTreeMap::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("read wrapper directory: {e}"))?;
            let file_type = entry.file_type().map_err(|e| e.to_string())?;
            if !file_type.is_dir() {
                continue;
            }
            let directory = entry.path().join("deploy");
            let path = directory.join("registration.pbtxt");
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(format!("{}: {error}", path.display())),
            };
            if !metadata.file_type().is_file() {
                return Err(format!(
                    "{}: registration must be a regular file",
                    path.display()
                ));
            }
            let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let mut app = Registration::parse(&text, directory)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            app.repository_root = fs::canonicalize(entry.path()).map_err(|e| e.to_string())?;
            app.validate()
                .map_err(|e| format!("{}: {e}", path.display()))?;
            let id = app.id.clone();
            if apps.insert(id.clone(), app).is_some() {
                return Err(format!("duplicate app id: {id}"));
            }
        }
        Ok(Self { apps })
    }
    /// Find a server-defined registration by its exact public ID.
    pub fn get(&self, id: &str) -> Option<&Registration> {
        self.apps.get(id)
    }
    /// Iterate registered applications in stable ID order.
    pub fn iter(&self) -> impl Iterator<Item = &Registration> {
        self.apps.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn workspace() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .unwrap()
            .to_owned()
    }
    #[test]
    fn gallery_apps_have_independent_canonical_repository_roots() {
        // Arrange
        let root =
            std::env::temp_dir().join(format!("gallery-app-roots-{:032x}", rand::random::<u128>()));
        for id in ["first", "second"] {
            let deploy = root.as_path().join("apps").join(id).join("deploy");
            fs::create_dir_all(&deploy).unwrap();
            fs::write(deploy.join("registration.pbtxt"), format!(
                "id: \"{id}\" compose_file: \"compose.yaml\" service: \"app\" network: \"default\" headless: true readiness_seconds: 1 stop_seconds: 1 result_file: \"result.mcap\" env_prefix: \"APP\""
            )).unwrap();
        }

        // Act
        let registry = Registry::load(root.as_path(), &[]).unwrap();

        // Assert
        let first = registry.get("first").unwrap();
        let second = registry.get("second").unwrap();
        assert_eq!(
            first.repository_root,
            fs::canonicalize(root.as_path().join("apps/first")).unwrap()
        );
        assert_eq!(
            second.repository_root,
            fs::canonicalize(root.as_path().join("apps/second")).unwrap()
        );
        assert_ne!(first.repository_root, second.repository_root);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn protobuf_registrations_preserve_defaults_and_reject_invalid_fields() {
        // Arrange
        let text =
            fs::read_to_string(workspace().join("apps/activity-demo/deploy/registration.pbtxt"))
                .unwrap();
        // Act
        let app = Registration::parse(&text, PathBuf::new()).unwrap();
        // Assert
        assert!(app.validate().is_ok());
        assert_eq!(app.result_format(), ResultFormat::Mcap);
        // Act / Assert: parsing rejects unknown fields and invalid integer widths.
        for invalid in [
            format!("{text} typo: true"),
            format!("{text} id: \"duplicate\""),
            text.replace("port: 8088", "port: 4294967296"),
        ] {
            assert!(Registration::parse(&invalid, PathBuf::new()).is_err());
        }
        // Act / Assert: validation rejects unknown enum numbers and invalid ports.
        for invalid in [
            format!("{text} result_format: 99"),
            text.replace("port: 8088", "port: 65536"),
        ] {
            assert!(
                Registration::parse(&invalid, PathBuf::new())
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
        // Act / Assert: every deployment profile, including pending apps, validates.
        for app in fs::read_dir(workspace().join("apps")).unwrap() {
            let directory = app.unwrap().path().join("deploy");
            let Ok(entries) = fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries {
                let path = entry.unwrap().path();
                if path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("registration")
                    && path.extension().and_then(|v| v.to_str()) == Some("pbtxt")
                {
                    let app =
                        Registration::parse(&fs::read_to_string(&path).unwrap(), directory.clone())
                            .unwrap();
                    assert!(app.validate().is_ok(), "{}", path.display());
                }
            }
        }
    }

    #[test]
    fn storage_requires_explicit_opt_in_and_owns_its_result_directory() {
        // Arrange
        let root = workspace();
        let disabled = Registry::new(&root).unwrap();
        let enabled = Registry::load(&root, &["storage-demo".into()]).unwrap();
        // Act
        let storage = enabled.get("storage-demo").unwrap();
        let project = storage
            .project(Path::new("/results/generation"), 1000, 1001)
            .unwrap();
        // Assert
        assert!(disabled.get("storage-demo").is_none());
        assert_eq!(
            project.environment["STORAGE_RESULTS_DIR"],
            "/results/generation"
        );
        assert_eq!(storage.capture_directory.as_deref(), Some("captures"));
        assert!(Registry::load(&root, &["missing".into()]).is_err());
    }
    #[test]
    fn registration_accepts_only_known_ids_and_owns_compose_settings() {
        // Arrange
        let registry = Registry::new(&workspace()).unwrap();
        // Act
        let app = registry.get("activity-demo").unwrap();
        let project = app
            .project(Path::new("/results/generation"), 1000, 1001)
            .unwrap();
        // Assert
        assert!(registry.get("unknown").is_none());
        assert!(registry.get("../activity-demo").is_none());
        assert_eq!(project.files, [PathBuf::from("compose.yaml")]);
        assert_eq!(
            project.environment["ACTIVITY_RESULTS_DIR"],
            "/results/generation"
        );
        assert_eq!(project.environment["ACTIVITY_UID"], "1000");
        assert_eq!(project.environment["ACTIVITY_GID"], "1001");
        assert!(!project.remove_volumes);
        assert!(app.project(Path::new("relative"), 1, 1).is_err());
    }
    #[test]
    fn private_directory_names_are_bounded_and_outside_results() {
        // Arrange
        let registry = Registry::new(&workspace()).unwrap();
        let mut app = registry.get("activity-demo").unwrap().clone();
        app.private_directories = vec!["socket".into(), "recovery".into()];

        // Act
        let project = app
            .project(Path::new("/data/generation/results"), 1000, 1000)
            .unwrap();
        let valid = app.validate();

        // Assert
        assert!(valid.is_ok());
        assert_eq!(
            project.environment["ACTIVITY_PRIVATE_SOCKET_DIR"],
            "/data/generation/private/socket"
        );
        assert_eq!(
            project.environment["ACTIVITY_PRIVATE_RECOVERY_DIR"],
            "/data/generation/private/recovery"
        );
        app.private_directories.push("../escape".into());
        assert!(app.validate().is_err());
        app.private_directories = vec!["socket".into(), "socket".into()];
        assert!(app.validate().is_err());
    }
    #[test]
    fn dora_workflow_services_are_included_with_the_http_endpoint() {
        // Arrange
        let registry = Registry::load(&workspace(), &["cpu-viewer".into()]).unwrap();
        let app = registry.get("cpu-viewer").unwrap();
        // Act
        let project = app
            .project(Path::new("/results/generation"), 1000, 1000)
            .unwrap();
        // Assert
        assert_eq!(project.services, ["coordinator", "viewer"]);
        assert!(app.exclusive_resources.is_empty());
        assert_eq!(
            app.system_apps.as_ref().unwrap().dora_flows[0].project,
            app.id
        );
    }
    #[test]
    fn host_monitor_apps_do_not_claim_an_exclusive_bpf_resource() {
        // Arrange
        let ids = [
            "cpu-viewer",
            "mcap-recorder",
            "storage-demo",
            "system-monitor",
        ];
        let enabled = ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();

        // Act
        let registry = Registry::load(&workspace(), &enabled).unwrap();

        // Assert
        for id in ids {
            assert!(
                !registry
                    .get(id)
                    .unwrap()
                    .exclusive_resources
                    .iter()
                    .any(|resource| resource == "host-bpf-monitor")
            );
        }
    }
    #[test]
    fn headless_registration_omits_http_port() {
        // Arrange
        let text =
            fs::read_to_string(workspace().join("apps/activity-demo/deploy/registration.pbtxt"))
                .unwrap();
        let mut app = Registration::parse(&text, PathBuf::new()).unwrap();
        // Act
        app.headless = true;
        app.port = None;
        // Assert
        assert!(app.validate().is_ok());
        // Act / Assert: mismatched presentation and port are rejected.
        app.port = Some(8080);
        assert!(app.validate().is_err());
        app.port = None;
        app.headless = false;
        assert!(app.validate().is_err());
        app.headless = true;
        // Act / Assert: shutdown evidence remains inside the results directory.
        let mut unsafe_marker = app.clone();
        unsafe_marker.shutdown_marker = Some("../escape".into());
        assert!(unsafe_marker.validate().is_err());
        // Act / Assert: additional names are bounded, distinct and flat.
        let mut extra = app.clone();
        extra.additional_files = vec!["diagnostics.log".into()];
        assert!(extra.validate().is_ok());
        extra.additional_files.push("diagnostics.log".into());
        assert!(extra.validate().is_err());
        extra.additional_files = vec!["../escape".into()];
        assert!(extra.validate().is_err());
        extra.additional_files = vec![extra.result_file.clone()];
        assert!(extra.validate().is_err());
    }
    #[test]
    fn collection_patterns_are_bounded_and_cannot_select_paths() {
        // Arrange
        let mut collection = ArtifactCollection {
            id: "recordings".into(),
            base_file: Some("faults.mcap".into()),
            prefix: "faults.mcap.".into(),
            suffix: "".into(),
            digits: 6,
            variable_digits: false,
            max_files: 128,
            required: false,
            directory: None,
            run_directories: false,
            run_prefix: None,
            generation_runs: false,
            format: ResultFormat::Mcap.into(),
        };
        // Act / Assert
        assert!(collection.valid());
        let checkpoint = serde_json::to_value(&collection).unwrap();
        assert_eq!(checkpoint["format"], "mcap");
        assert_eq!(
            serde_json::from_value::<ArtifactCollection>(checkpoint).unwrap(),
            collection
        );
        assert!(collection.matches("faults.mcap"));
        assert!(collection.matches("faults.mcap.000001"));
        assert!(!collection.matches("faults.mcap.1"));
        assert!(!collection.matches("faults.mcap.000001/escape"));
        collection.variable_digits = true;
        assert!(collection.matches("faults.mcap.1"));
        assert!(!collection.matches("faults.mcap.1234567"));
        assert!(!collection.matches("faults.mcap."));
        collection.prefix = "../".into();
        assert!(!collection.valid());
        collection.prefix = "faults.mcap.".into();
        collection.max_files = 513;
        assert!(!collection.valid());
        assert!(artifact_path(
            "runs/0123456789abcdef0123456789abcdef/producer.mcap"
        ));
        assert!(!artifact_path("runs/../producer.mcap"));
        assert!(!artifact_path("a/b/c/d"));
        assert!(run_directory("0123456789abcdef0123456789abcdef"));
        assert!(!run_directory("0123456789ABCDEF0123456789ABCDEF"));
        collection.max_files = 128;
        collection.run_directories = true;
        collection.run_prefix = Some("doramcap-live-".into());
        assert!(collection.valid());
        assert!(collection.matches_run("doramcap-live-123-456-1"));
        assert!(!collection.matches_run("doramcap-live-123-456-1-inputs"));
        collection.run_prefix = None;
        collection.generation_runs = true;
        assert!(collection.valid());
        assert!(collection.matches_run("gal-0123456789abcdef0123456789abcdef-999"));
        assert!(!collection.matches_run("gal-0123456789abcdef0123456789abcdef-1000"));
        assert!(!collection.matches_run("gal-0123456789abcdef0123456789abcdef-"));
        assert!(!collection.matches_run("gal-0123456789abcdef0123456789abcdef-1-inputs"));
    }

    #[test]
    fn cuda_registration_selects_pid_files_without_logs() {
        // Arrange
        let text = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../cuda-shm-demo/deploy/registration.pbtxt"
        ));
        let registration = Registration::parse(text, PathBuf::new()).unwrap();

        // Act
        let validated = registration.validate();
        let producer = registration
            .artifact_collections
            .iter()
            .find(|c| c.id == "producer-mcap")
            .unwrap();
        let activity = registration
            .artifact_collections
            .iter()
            .find(|c| c.id == "consumer-activity")
            .unwrap();

        // Assert
        assert!(validated.is_ok());
        assert!(producer.matches("producer-12.mcap"));
        assert!(producer.matches("producer-1234567890.mcap"));
        assert!(!producer.matches("producer-12345678901.mcap"));
        assert!(activity.matches("consumer-456.activity.pb"));
        assert!(
            !registration
                .artifact_collections
                .iter()
                .any(|c| c.matches("run.log"))
        );
    }
    #[test]
    fn loads_independent_manifest_and_rejects_duplicate_and_unsafe_entries() {
        // Arrange
        let root =
            std::env::temp_dir().join(format!("registry-test-{:032x}", rand::random::<u128>()));
        let wrappers = root.join("apps");
        let first = wrappers.join("first/deploy");
        let second = wrappers.join("second/deploy");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        let original =
            fs::read_to_string(workspace().join("apps/activity-demo/deploy/registration.pbtxt"))
                .unwrap();
        let custom = original.replace("activity-demo", "new-app");
        fs::write(first.join("registration.pbtxt"), &custom).unwrap();
        // Act
        let loaded = Registry::new(&root).unwrap();
        // Assert
        assert!(loaded.get("new-app").is_some());
        assert_eq!(loaded.get("new-app").unwrap().directory, first);
        // Act / Assert
        fs::write(second.join("registration.pbtxt"), &custom).unwrap();
        assert!(
            Registry::new(&root)
                .unwrap_err()
                .contains("duplicate app id")
        );
        fs::remove_file(second.join("registration.pbtxt")).unwrap();
        fs::write(
            first.join("registration.pbtxt"),
            custom.replace("activity.mcap", "../escape.mcap"),
        )
        .unwrap();
        assert!(Registry::new(&root).is_err());
        fs::write(
            first.join("registration.pbtxt"),
            custom.replace("new-app", "../escape"),
        )
        .unwrap();
        assert!(Registry::new(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
