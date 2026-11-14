//! Typed metadata at Docker's flat string-label boundary; shipped keys stay stable.
use crate::docker_proto::ResourceLabels;
use std::collections::HashMap;

pub const RUN_LABEL: &str = "io.doramcap.run";
pub const ARTIFACTS_LABEL: &str = "io.doramcap.artifacts";
pub const CONTROLLER_LABEL: &str = "io.doramcap.controller";
pub const EXPERIMENT_OWNER_LABEL: &str = "io.doramcap.experiment-owner";
pub const REMOVE_VOLUME_LABEL: &str = "io.doramcap.remove-volume";
pub const JOURNAL_EXPORT_LABEL: &str = "io.doramcap.journal-export";

impl ResourceLabels {
    /// Decode existing Docker labels, preserving missing values and legacy marker semantics.
    /// Boolean values enable a policy only when exactly "true"; unrelated labels are ignored.
    pub fn from_docker_labels(labels: &HashMap<String, String>) -> Self {
        Self {
            controller: labels.get(CONTROLLER_LABEL).cloned(),
            run: labels.get(RUN_LABEL).cloned(),
            artifacts: labels.get(ARTIFACTS_LABEL).cloned(),
            experiment_owner: labels.contains_key(EXPERIMENT_OWNER_LABEL),
            remove_volume: labels.get(REMOVE_VOLUME_LABEL).map(|value| value == "true"),
            journal_export: labels
                .get(JOURNAL_EXPORT_LABEL)
                .map(|value| value == "true"),
        }
    }

    /// Encode supported fields with the existing keys and string values, omitting absent fields.
    /// Merge the returned labels into the object's native map to retain unrelated metadata.
    pub fn to_docker_labels(&self) -> HashMap<String, String> {
        let mut labels = HashMap::new();
        for (key, value) in [
            (CONTROLLER_LABEL, self.controller.as_ref()),
            (RUN_LABEL, self.run.as_ref()),
            (ARTIFACTS_LABEL, self.artifacts.as_ref()),
        ] {
            if let Some(value) = value {
                labels.insert(key.into(), value.clone());
            }
        }
        if self.experiment_owner {
            labels.insert(EXPERIMENT_OWNER_LABEL.into(), "true".into());
        }
        for (key, value) in [
            (REMOVE_VOLUME_LABEL, self.remove_volume),
            (JOURNAL_EXPORT_LABEL, self.journal_export),
        ] {
            if let Some(value) = value {
                labels.insert(key.into(), value.to_string());
            }
        }
        labels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_label_maps_keep_their_encodings_and_policy_values() {
        // Arrange: literal keys are the persisted contract from installed versions.
        let labels = HashMap::from([
            ("io.doramcap.controller".into(), "/tmp/repo-scope".into()),
            ("io.doramcap.run".into(), "old-run".into()),
            ("io.doramcap.artifacts".into(), "/data/runs/old-run".into()),
            ("io.doramcap.experiment-owner".into(), "true".into()),
            ("io.doramcap.remove-volume".into(), "false".into()),
            ("io.doramcap.journal-export".into(), "true".into()),
        ]);
        // Act
        let decoded = ResourceLabels::from_docker_labels(&labels);
        let encoded = decoded.to_docker_labels();
        // Assert
        assert_eq!(decoded.controller.as_deref(), Some("/tmp/repo-scope"));
        assert_eq!(decoded.run.as_deref(), Some("old-run"));
        assert_eq!(decoded.artifacts.as_deref(), Some("/data/runs/old-run"));
        assert!(decoded.experiment_owner);
        assert_eq!(decoded.remove_volume, Some(false));
        assert_eq!(decoded.journal_export, Some(true));
        assert_eq!(encoded, labels);
    }

    #[test]
    fn older_resources_distinguish_absent_policy_from_explicit_false() {
        // Arrange: older containers have ownership but no journal-export label.
        let mut labels = HashMap::from([
            ("io.doramcap.run".into(), "old-run".into()),
            ("com.docker.compose.service".into(), "camera".into()),
        ]);
        // Act
        let old = ResourceLabels::from_docker_labels(&labels);
        // Assert
        assert_eq!(old.run.as_deref(), Some("old-run"));
        assert_eq!(old.controller, None);
        assert_eq!(old.artifacts, None);
        assert_eq!(old.journal_export, None);
        assert_eq!(old.remove_volume, None);
        assert!(!old.experiment_owner);
        assert!(!old.to_docker_labels().contains_key(JOURNAL_EXPORT_LABEL));
        // Act
        labels.insert("io.doramcap.journal-export".into(), "false".into());
        labels.insert("io.doramcap.remove-volume".into(), "false".into());
        let explicit = ResourceLabels::from_docker_labels(&labels);
        // Assert
        assert_eq!(explicit.journal_export, Some(false));
        assert_eq!(explicit.remove_volume, Some(false));
        assert_eq!(labels.get("com.docker.compose.service").unwrap(), "camera");
    }

    #[test]
    fn noncanonical_values_keep_legacy_safe_boolean_and_presence_rules() {
        // Arrange: historical readers checked literal true and marker presence.
        let labels = HashMap::from([
            ("io.doramcap.experiment-owner".into(), "false".into()),
            ("io.doramcap.remove-volume".into(), "TRUE".into()),
            ("io.doramcap.journal-export".into(), "unknown".into()),
        ]);
        // Act
        let decoded = ResourceLabels::from_docker_labels(&labels);
        // Assert
        assert!(decoded.experiment_owner);
        assert_eq!(decoded.remove_volume, Some(false));
        assert_eq!(decoded.journal_export, Some(false));
        assert_eq!(decoded.controller, None);
        assert_eq!(decoded.run, None);
    }
}
