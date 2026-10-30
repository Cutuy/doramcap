//! Host-approved system apps and per-review selection.
use crate::protocol::{ConsoleConfig, SystemApp, SystemAppSelection};
use workload_docker::workflows::system_apps::SystemAppsConfig;

pub(crate) fn select(
    config: &ConsoleConfig,
    requested: Option<&SystemAppSelection>,
) -> Result<SystemAppsConfig, String> {
    let available = config.system_apps.clone().unwrap_or_default();
    let Some(requested) = requested else {
        return Ok(available);
    };
    if requested.ids.len() > 1 || requested.ids.iter().any(|id| id != "cpu-monitor") {
        return Err("unknown or duplicate system app".into());
    }
    if requested.ids.is_empty() {
        return Ok(SystemAppsConfig::default());
    }
    if available.cpu_monitor.is_none() {
        return Err("CPU monitor is not configured".into());
    }
    Ok(available)
}

pub(crate) fn options(config: &ConsoleConfig, selected: &SystemAppsConfig) -> Vec<SystemApp> {
    if config
        .system_apps
        .as_ref()
        .and_then(|apps| apps.cpu_monitor.as_ref())
        .is_none()
    {
        return Vec::new();
    }
    vec![SystemApp {
        id: "cpu-monitor".into(),
        label: "CPU recording".into(),
        selected: selected.cpu_monitor.is_some(),
    }]
}

pub(crate) fn selection(apps: &SystemAppsConfig) -> SystemAppSelection {
    SystemAppSelection {
        ids: if apps.cpu_monitor.is_some() {
            vec!["cpu-monitor".into()]
        } else {
            Vec::new()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use workload_docker::workflows::system_apps::CpuMonitor;

    #[test]
    fn selection_uses_host_defaults_and_validates_explicit_ids() {
        // Arrange
        let config = ConsoleConfig {
            system_apps: Some(SystemAppsConfig {
                cpu_monitor: Some(CpuMonitor {
                    image: "local:monitor".into(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        // Act / Assert
        assert!(select(&config, None).unwrap().cpu_monitor.is_some());
        let empty = SystemAppSelection::default();
        let disabled = select(&config, Some(&empty)).unwrap();
        assert!(disabled.cpu_monitor.is_none());
        assert!(!options(&config, &disabled)[0].selected);
        assert!(selection(&disabled).ids.is_empty());
        for ids in [vec!["cpu-monitor", "cpu-monitor"], vec!["unknown"]] {
            assert!(
                select(
                    &config,
                    Some(&SystemAppSelection {
                        ids: ids.into_iter().map(str::to_owned).collect()
                    })
                )
                .is_err()
            );
        }
        assert!(
            select(
                &ConsoleConfig::default(),
                Some(&SystemAppSelection {
                    ids: vec!["cpu-monitor".into()]
                })
            )
            .is_err()
        );
        assert!(
            select(&ConsoleConfig::default(), None)
                .unwrap()
                .cpu_monitor
                .is_none()
        );
    }
}
