use resource_policy::{CpuSet, Goal, config, proto, targets};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "resource-policy-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn put(&self, file: &str, data: &str) -> PathBuf {
        let path = self.0.join(file);
        fs::write(&path, data).unwrap();
        path
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn cpulist_normalization_and_bounds() {
    for (input, expected) in [
        ("0", "0"),
        ("1-3", "1-3"),
        ("4,5-7,9-11", "4-7,9-11"),
        ("9,1-3,2,1", "1-3,9"),
        ("4096", "4096"),
    ] {
        let cpus = CpuSet::parse(input).unwrap();
        assert_eq!(cpus.canonical(), expected);
        assert_eq!(CpuSet::parse(&cpus.canonical()).unwrap(), cpus);
    }
    for input in [
        "",
        " ",
        "0,",
        "-1",
        "1-0",
        "1--3",
        "1-2-3",
        "1-3:2",
        "1, 2",
        "4294967296",
        "1048576",
        "0-1048575,0-1048575",
    ] {
        assert!(CpuSet::parse(input).is_err(), "{input}");
    }
}
#[test]
fn overlays_preserve_zero_and_replace_selection() {
    let tmp = Temp::new();
    tmp.put(
        "base.pbtxt",
        "app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { placement { cpus: '4,5-7' } rt_priority: 30 } } } app { id: 'b' config { [type.googleapis.com/resource_policy.v1.Policy] { placement { cpus: '0' } } } }",
    );
    let lab = tmp.put(
        "lab.pbtxt",
        "include: 'base.pbtxt' app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { placement { cpus: '1-3' } rt_priority: 0 } } }",
    );
    let result = config::resolve(&lab, &["b", "a"]).unwrap();
    assert_eq!(
        result
            .config("a")
            .map(|p| Goal::from_proto(&p).unwrap())
            .unwrap()
            .cpus()
            .canonical(),
        "1-3"
    );
    assert_eq!(
        result
            .config("a")
            .map(|p| Goal::from_proto(&p).unwrap())
            .unwrap()
            .rt_priority(),
        0
    );
    assert_eq!(
        result
            .config("b")
            .map(|p| Goal::from_proto(&p).unwrap())
            .unwrap()
            .cpus()
            .canonical(),
        "0"
    );
    assert_eq!(result.provenance.len(), 5);
    assert_eq!(
        result.digest(),
        config::resolve(&lab, &["a", "b"]).unwrap().digest()
    );
    assert_eq!(result.deployment().app[0].id, "a");
    assert!(
        result
            .config("unknown")
            .map(|p| Goal::from_proto(&p).unwrap())
            .is_err()
    );
    let single = tmp.put(
        "single.pbtxt",
        "app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { placement { cpus: '3,1,2' } rt_priority: 0 } } } app { id: 'b' config { [type.googleapis.com/resource_policy.v1.Policy] { placement { cpus: '0' } } } }",
    );
    assert_eq!(
        result.canonical_bytes(),
        config::resolve(&single, &["a", "b"])
            .unwrap()
            .canonical_bytes()
    );
    let partial = tmp.put(
        "partial.pbtxt",
        "include: 'base.pbtxt' app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { rt_priority: 1 } } }",
    );
    let goal = config::resolve(&partial, &["a", "b"])
        .unwrap()
        .config("a")
        .map(|p| Goal::from_proto(&p).unwrap())
        .unwrap();
    assert_eq!(goal.cpus().canonical(), "4-7");
    assert_eq!(goal.rt_priority(), 1);
}
#[test]
fn fragments_are_checked_even_if_overridden() {
    let tmp = Temp::new();
    for invalid in [
        "app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { placement { cpus: '' } } } }",
        "app { id: 'missing' }",
        "app { id: 'a' } app { id: 'a' }",
        "app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { rt_priority: 100 } } }",
        "app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { inherit_cpus: false } } }",
        "unknown: 1",
        "app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { rt_priority: 1 rt_priority: 2 } } }",
    ] {
        tmp.put("bad.pbtxt", invalid);
        let main = tmp.put(
            "main.pbtxt",
            "include: 'bad.pbtxt' app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { placement { cpus: '0' } rt_priority: 0 } } }",
        );
        let error = config::resolve(&main, &["a"]).unwrap_err().to_string();
        assert!(error.contains("bad.pbtxt"), "{error}");
    }
}
#[test]
fn include_cycles_repetition_and_limits() {
    let tmp = Temp::new();
    let a = tmp.put("a", "include: 'b'");
    tmp.put("b", "include: 'a'");
    assert!(
        config::resolve(&a, &[])
            .unwrap_err()
            .to_string()
            .contains("cycle")
    );
    tmp.put("b", "include: 'missing'");
    assert!(config::resolve(&a, &[]).is_err());
    tmp.put(
        "b",
        "app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { placement { cpus: '0' } rt_priority: 2 } } }",
    );
    tmp.put("c", "app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { rt_priority: 3 } } }");
    tmp.put("a", "include: 'b' include: 'c' include: 'b'");
    assert_eq!(
        config::resolve(&a, &["a"])
            .unwrap()
            .config("a")
            .map(|p| Goal::from_proto(&p).unwrap())
            .unwrap()
            .rt_priority(),
        2
    );
    tmp.put("a", &"include: 'b'\n".repeat(256));
    assert!(config::resolve(&a, &["a"]).is_err());
    assert!(config::resolve(&a, &["a", "a"]).is_err());
}
#[test]
fn targets_keep_exact_placement_and_rt_semantics() {
    let fragment = config::parse_fragment(
        "app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { placement { cpus: '4,5-7,9' } rt_priority: 30 } } }",
    )
    .unwrap();
    let goal = Goal::from_proto(fragment.app[0].config.as_ref().unwrap()).unwrap();
    assert_eq!(
        targets::systemd_service_directives(&goal),
        "AllowedCPUs=4-7 9\nCPUAffinity=4-7 9\nCPUSchedulingPolicy=rr\nCPUSchedulingPriority=30\n"
    );
    assert!(targets::validate_standard_kubernetes(&goal).is_err());
    assert!(Goal::from_proto(&proto::Policy::default()).is_err());
    let ordinary = Goal::from_proto(&proto::Policy {
        placement: Some(proto::Placement { cpus: "0".into() }),
        rt_priority: None,
    })
    .unwrap();
    assert!(targets::systemd_service_directives(&ordinary).contains("CPUSchedulingPolicy=other"));
}
#[cfg(target_os = "linux")]
#[test]
fn current_thread_converges_and_children_inherit() {
    std::thread::spawn(|| {
        use resource_policy::linux::*;
        let initial = inspect_current_thread().unwrap();
        let goal = Goal::from_proto(&proto::Policy {
            placement: Some(proto::Placement {
                cpus: initial.cpus.ids()[0].to_string(),
            }),
            rt_priority: Some(0),
        })
        .unwrap();
        let first = apply_current_thread(&goal).unwrap();
        assert_eq!(first.after.cpus, *goal.cpus());
        assert!(apply_current_thread(&goal).unwrap().changed.is_empty());
        let inherited = std::thread::spawn(inspect_current_thread)
            .join()
            .unwrap()
            .unwrap();
        assert_eq!(inherited, first.after);
        let restore = Goal::from_proto(&proto::Policy {
            placement: Some(proto::Placement {
                cpus: initial.cpus.canonical(),
            }),
            rt_priority: Some(0),
        })
        .unwrap();
        apply_current_thread(&restore).unwrap();
        assert_eq!(inspect_current_thread().unwrap().cpus, initial.cpus);
        assert_eq!(
            plan_current_thread(&goal).unwrap().desired.cpus,
            *goal.cpus()
        );
        assert_eq!(
            apply_current_thread(&goal).unwrap().after.cpus,
            *goal.cpus()
        );
        apply_current_thread(&restore).unwrap();
    })
    .join()
    .unwrap();
}

#[test]
fn resolved_goals_require_explicit_placement() {
    let tmp = Temp::new();
    for input in [
        "",
        "app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { rt_priority: 0 } } }",
    ] {
        let path = tmp.put("missing", input);
        let error = config::resolve(&path, &["a"]).unwrap_err().to_string();
        assert!(
            error.contains("app a") && error.contains("placement.cpus"),
            "{error}"
        );
    }
    for input in [
        "app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { placement {} } } }",
        "app { id: 'a' config { [type.googleapis.com/resource_policy.v1.Policy] { inherit_cpus: true } } }",
    ] {
        let path = tmp.put("invalid", input);
        assert!(config::resolve(&path, &["a"]).is_err());
    }
}
