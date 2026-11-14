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
    /// Decode Docker values with proto3 defaults and legacy marker semantics.
    /// Read presence from the native map when applying defaults for older resources.
    pub fn from_docker_labels(labels: &HashMap<String, String>) -> Self {
        Self {
            controller: labels.get(CONTROLLER_LABEL).cloned().unwrap_or_default(),
            run: labels.get(RUN_LABEL).cloned().unwrap_or_default(),
            artifacts: labels.get(ARTIFACTS_LABEL).cloned().unwrap_or_default(),
            experiment_owner: labels.contains_key(EXPERIMENT_OWNER_LABEL),
            remove_volume: labels
                .get(REMOVE_VOLUME_LABEL)
                .is_some_and(|value| value == "true"),
            journal_export: labels
                .get(JOURNAL_EXPORT_LABEL)
                .is_some_and(|value| value == "true"),
        }
    }

    /// Encode populated ownership strings and the experiment-owner presence marker.
    /// Policy labels are added explicitly for their resource type, including false values.
    pub fn identity_labels(&self) -> HashMap<String, String> {
        let mut labels = HashMap::new();
        for (key, value) in [
            (CONTROLLER_LABEL, &self.controller),
            (RUN_LABEL, &self.run),
            (ARTIFACTS_LABEL, &self.artifacts),
        ] {
            if !value.is_empty() {
                labels.insert(key.into(), value.clone());
            }
        }
        if self.experiment_owner {
            labels.insert(EXPERIMENT_OWNER_LABEL.into(), "true".into());
        }
        labels
    }

    /// Encode container identity and its explicit journal export/skip choice.
    pub fn journal_labels(&self) -> HashMap<String, String> {
        let mut labels = self.identity_labels();
        labels.insert(JOURNAL_EXPORT_LABEL.into(), self.journal_export.to_string());
        labels
    }

    /// Encode volume identity and its explicit deletion policy.
    pub fn volume_labels(&self) -> HashMap<String, String> {
        let mut labels = self.identity_labels();
        labels.insert(REMOVE_VOLUME_LABEL.into(), self.remove_volume.to_string());
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
        let mut encoded = decoded.journal_labels();
        encoded.extend(decoded.volume_labels());
        // Assert
        assert_eq!(decoded.controller, "/tmp/repo-scope");
        assert_eq!(decoded.run, "old-run");
        assert_eq!(decoded.artifacts, "/data/runs/old-run");
        assert!(decoded.experiment_owner);
        assert!(!decoded.remove_volume);
        assert!(decoded.journal_export);
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
        assert_eq!(old.run, "old-run");
        assert!(old.controller.is_empty());
        assert!(old.artifacts.is_empty());
        assert!(!old.journal_export);
        assert!(!old.remove_volume);
        assert!(!old.experiment_owner);
        assert!(!old.identity_labels().contains_key(JOURNAL_EXPORT_LABEL));
        assert!(!labels.contains_key(JOURNAL_EXPORT_LABEL));
        assert!(!labels.contains_key(REMOVE_VOLUME_LABEL));
        // Act
        labels.insert("io.doramcap.journal-export".into(), "false".into());
        labels.insert("io.doramcap.remove-volume".into(), "false".into());
        let explicit = ResourceLabels::from_docker_labels(&labels);
        // Assert
        assert!(!explicit.journal_export);
        assert!(!explicit.remove_volume);
        assert!(labels.contains_key(JOURNAL_EXPORT_LABEL));
        assert!(labels.contains_key(REMOVE_VOLUME_LABEL));
        assert_eq!(
            explicit.journal_labels().get(JOURNAL_EXPORT_LABEL).unwrap(),
            "false"
        );
        assert_eq!(
            explicit.volume_labels().get(REMOVE_VOLUME_LABEL).unwrap(),
            "false"
        );
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
        assert!(!decoded.remove_volume);
        assert!(!decoded.journal_export);
        assert!(decoded.controller.is_empty());
        assert!(decoded.run.is_empty());
    }
}
