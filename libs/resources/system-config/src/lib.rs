//! Typed, read-only observation of Linux host state and shared-memory metadata.
mod shm;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Read,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

include!(concat!(env!("OUT_DIR"), "/doramcap.system_config.v1.rs"));

/// Largest file read for one fact; `/proc/cpuinfo` on large hosts stays well below this.
const MAX_FACT_BYTES: u64 = 4 * 1024 * 1024;
const MILLIDEGREES_PER_DEGREE: f64 = 1000.0;
const BYTES_PER_KIB: u64 = 1024;
const CPU_DIRECTORY: &str = "sys/devices/system/cpu";

/// Read `path` beneath `root` as trimmed text; unreadable or non-UTF-8 content is `None`.
fn text(root: &Path, path: &str) -> Option<String> {
    let mut content = String::new();
    File::open(root.join(path))
        .ok()?
        .take(MAX_FACT_BYTES)
        .read_to_string(&mut content)
        .ok()?;
    Some(content.trim().to_owned())
}
fn parsed<T: std::str::FromStr>(root: &Path, path: &str) -> Option<T> {
    text(root, path)?.parse().ok()
}
/// The value of the first `label: value` line in `content`.
fn labeled<'a>(content: &'a str, label: &str) -> Option<&'a str> {
    content.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        (name.trim() == label).then(|| value.trim())
    })
}
/// One meminfo size in bytes.
fn meminfo_bytes(meminfo: &str, label: &str) -> Option<u64> {
    let kib = labeled(meminfo, label)?.split_whitespace().next()?;
    kib.parse::<u64>().ok()?.checked_mul(BYTES_PER_KIB)
}
/// Directory names beneath `root/path` starting with `prefix` and followed by digits.
fn numbered(root: &Path, path: &str, prefix: &str) -> Vec<u32> {
    let Ok(entries) = fs::read_dir(root.join(path)) else {
        return Vec::new();
    };
    let mut ids: Vec<u32> = entries
        .flatten()
        .filter_map(|entry| {
            entry
                .file_name()
                .to_str()?
                .strip_prefix(prefix)?
                .parse()
                .ok()
        })
        .collect();
    ids.sort_unstable();
    ids
}
/// Group one per-CPU setting by value, so a mixed machine reports each value and its CPUs.
fn per_cpu(root: &Path, online: &[u32], setting: &str) -> Vec<CpuValue> {
    let mut values: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for cpu in online {
        if let Some(value) = text(root, &format!("{CPU_DIRECTORY}/cpu{cpu}/cpufreq/{setting}")) {
            values.entry(value).or_default().push(*cpu);
        }
    }
    values
        .into_iter()
        .map(|(value, cpus)| CpuValue {
            value,
            cpus: cpulist::emit(cpus),
        })
        .collect()
}
/// Intel reports disabled turbo and other drivers report enabled boost.
fn turbo_enabled(root: &Path) -> Option<bool> {
    parsed::<u8>(root, &format!("{CPU_DIRECTORY}/intel_pstate/no_turbo"))
        .map(|disabled| disabled == 0)
        .or_else(|| {
            parsed::<u8>(root, &format!("{CPU_DIRECTORY}/cpufreq/boost"))
                .map(|enabled| enabled != 0)
        })
}
/// The bracketed choice in a sysfs selection such as `always [madvise] never`.
fn selected(setting: &str) -> &str {
    setting
        .split_once('[')
        .and_then(|(_, rest)| rest.split_once(']'))
        .map_or(setting, |(choice, _)| choice)
}
fn gpu_models(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root.join("proc/driver/nvidia/gpus")) else {
        return Vec::new();
    };
    let mut models: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let information = fs::read_to_string(entry.path().join("information")).ok()?;
            labeled(&information, "Model").map(str::to_owned)
        })
        .collect();
    models.sort();
    models
}
fn max_temperature(root: &Path) -> Option<f64> {
    numbered(root, "sys/class/thermal", "thermal_zone")
        .into_iter()
        .filter_map(|zone| {
            parsed::<f64>(root, &format!("sys/class/thermal/thermal_zone{zone}/temp"))
        })
        .map(|millidegrees| millidegrees / MILLIDEGREES_PER_DEGREE)
        .max_by(f64::total_cmp)
}
fn configuration(root: &Path) -> MachineConfiguration {
    let online_cpus = text(root, &format!("{CPU_DIRECTORY}/online")).unwrap_or_default();
    let online: Vec<u32> = cpulist::parse(&online_cpus).unwrap_or_default();
    let setting = |path: &str| text(root, path).unwrap_or_default();
    MachineConfiguration {
        machine_fingerprint: text(root, "etc/machine-id")
            .filter(|id| !id.is_empty())
            .map(|id| hex::encode(Sha256::digest(id.as_bytes())))
            .unwrap_or_default(),
        architecture: std::env::consts::ARCH.to_owned(),
        cpu_model: text(root, "proc/cpuinfo")
            .and_then(|cpuinfo| labeled(&cpuinfo, "model name").map(str::to_owned))
            .unwrap_or_default(),
        online_cpus,
        numa_nodes: numbered(root, "sys/devices/system/node", "node").len() as u32,
        memory_total_bytes: text(root, "proc/meminfo")
            .and_then(|meminfo| meminfo_bytes(&meminfo, "MemTotal"))
            .unwrap_or_default(),
        kernel_release: setting("proc/sys/kernel/osrelease"),
        kernel_build: setting("proc/sys/kernel/version"),
        boot_flags: setting("proc/cmdline"),
        isolated_cpus: setting(&format!("{CPU_DIRECTORY}/isolated")),
        nohz_full_cpus: setting(&format!("{CPU_DIRECTORY}/nohz_full")),
        governor: per_cpu(root, &online, "scaling_governor"),
        frequency_driver: per_cpu(root, &online, "scaling_driver"),
        energy_preference: per_cpu(root, &online, "energy_performance_preference"),
        turbo_enabled: turbo_enabled(root).unwrap_or_default(),
        smt_active: parsed::<u8>(root, &format!("{CPU_DIRECTORY}/smt/active"))
            .is_some_and(|active| active != 0),
        transparent_hugepages: text(root, "sys/kernel/mm/transparent_hugepage/enabled")
            .map(|setting| selected(&setting).to_owned())
            .unwrap_or_default(),
        swappiness: parsed(root, "proc/sys/vm/swappiness").unwrap_or_default(),
        rt_runtime_us: parsed(root, "proc/sys/kernel/sched_rt_runtime_us").unwrap_or_default(),
        cgroup_controllers: setting("sys/fs/cgroup/cgroup.controllers"),
        gpu_driver: setting("sys/module/nvidia/version"),
        gpu_models: gpu_models(root),
        docker: None,
    }
}
fn condition(root: &Path, data_directory: &Path) -> MachineCondition {
    let first_field = |path: &str| -> f64 {
        text(root, path)
            .and_then(|content| content.split_whitespace().next()?.parse().ok())
            .unwrap_or_default()
    };
    MachineCondition {
        boot_id: text(root, "proc/sys/kernel/random/boot_id").unwrap_or_default(),
        uptime_seconds: first_field("proc/uptime"),
        load_1m: first_field("proc/loadavg"),
        memory_available_bytes: text(root, "proc/meminfo")
            .and_then(|meminfo| meminfo_bytes(&meminfo, "MemAvailable"))
            .unwrap_or_default(),
        disk_free_bytes: rustix::fs::statvfs(data_directory)
            .map_or(0, |stats| stats.f_bavail.saturating_mul(stats.f_frsize)),
        max_temperature_celsius: max_temperature(root).unwrap_or_default(),
        other_containers: Vec::new(),
    }
}

/// Observe this host now. `data_directory` selects the filesystem reported as free disk.
///
/// Returns: readable facts; host-shm capture failure leaves `host_shm` absent with `host_shm_error`.
/// Other unreadable facts keep their zero values. The caller adds Docker facts.
pub fn observe(data_directory: &Path) -> MachineState {
    observe_root(Path::new("/"), data_directory)
}

/// Observe the Linux file layout beneath `root`.
pub fn observe_root(root: &Path, data_directory: &Path) -> MachineState {
    let (host_shm, host_shm_error) = match shm::observe(root) {
        Ok(snapshot) => (Some(snapshot), String::new()),
        Err(error) => (None, error),
    };
    MachineState {
        captured_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis() as u64),
        configuration: Some(configuration(root)),
        condition: Some(condition(root, data_directory)),
        host_shm,
        host_shm_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, path: &str, content: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn an_empty_host_reports_zero_values() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        // Act
        let state = observe_root(root.path(), root.path());
        // Assert
        let configuration = state.configuration.unwrap();
        assert_eq!(
            configuration,
            MachineConfiguration {
                architecture: configuration.architecture.clone(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn mixed_cpu_settings_report_each_value_with_its_cpus() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "sys/devices/system/cpu/online", "0-3\n");
        for (cpu, governor) in [
            (0, "performance"),
            (1, "powersave"),
            (2, "powersave"),
            (3, "powersave"),
        ] {
            write(
                root.path(),
                &format!("sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_governor"),
                governor,
            );
        }
        // Act
        let governor = observe_root(root.path(), root.path())
            .configuration
            .unwrap()
            .governor;
        // Assert
        assert_eq!(
            governor,
            [
                CpuValue {
                    value: "performance".into(),
                    cpus: "0".into()
                },
                CpuValue {
                    value: "powersave".into(),
                    cpus: "1-3".into()
                },
            ]
        );
    }

    #[test]
    fn missing_and_blank_machine_ids_have_no_fingerprint() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        // Act / Assert
        assert!(configuration(root.path()).machine_fingerprint.is_empty());
        for id in ["", " \n\t"] {
            // Act
            write(root.path(), "etc/machine-id", id);
            // Assert
            assert!(configuration(root.path()).machine_fingerprint.is_empty());
        }
    }

    #[test]
    fn host_files_become_typed_facts() {
        // Arrange
        let root = tempfile::tempdir().unwrap();
        let host = root.path();
        write(host, "etc/machine-id", "secret-machine-id\n");
        write(
            host,
            "proc/meminfo",
            "MemTotal: 2048 kB\nMemAvailable: 1024 kB\n",
        );
        write(
            host,
            "proc/cpuinfo",
            "processor : 0\nmodel name : Test CPU\n",
        );
        write(host, "proc/loadavg", "1.5 1.0 0.5 1/100 42\n");
        write(host, "sys/devices/system/cpu/intel_pstate/no_turbo", "1\n");
        write(
            host,
            "sys/kernel/mm/transparent_hugepage/enabled",
            "always [madvise] never\n",
        );
        write(host, "sys/class/thermal/thermal_zone0/temp", "30000\n");
        write(host, "sys/class/thermal/thermal_zone1/temp", "61500\n");
        write(host, "sys/devices/system/node/node0/x", "");
        write(
            host,
            "proc/driver/nvidia/gpus/0/information",
            "Model: \t Test GPU\nIRQ: 16\n",
        );
        write(host, "proc/sys/vm/swappiness", "not a number\n");
        // Act
        let state = observe_root(host, host);
        let (configuration, condition) = (state.configuration.unwrap(), state.condition.unwrap());
        // Assert
        assert_eq!(configuration.machine_fingerprint.len(), 64);
        assert!(
            !configuration
                .machine_fingerprint
                .contains("secret-machine-id")
        );
        assert_eq!(configuration.cpu_model, "Test CPU");
        assert_eq!(configuration.memory_total_bytes, 2048 * 1024);
        assert!(!configuration.turbo_enabled);
        assert_eq!(configuration.transparent_hugepages, "madvise");
        assert_eq!(configuration.numa_nodes, 1);
        assert_eq!(configuration.gpu_models, ["Test GPU"]);
        assert_eq!(configuration.swappiness, 0);
        assert_eq!(condition.memory_available_bytes, 1024 * 1024);
        assert_eq!(condition.load_1m, 1.5);
        assert_eq!(condition.max_temperature_celsius, 61.5);
        assert!(condition.disk_free_bytes > 0);
    }
}
