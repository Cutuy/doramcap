use resource_policy::{CpuSet, Goal, config, proto, targets};
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
fn direct_textformat_preserves_presence_and_rejects_dialects() {
    // Arrange
    let text = "app { id: 'a' policy { placement { cpus: '1-3' } rt_priority: 0 } }";
    // Act
    let deployment = config::parse(text).unwrap();
    // Assert
    assert_eq!(
        deployment.app[0].policy.as_ref().unwrap().rt_priority,
        Some(0)
    );
    for invalid in [
        "include: 'base.pbtxt'",
        "app { id:'a' }",
        "app {id:'a' policy {rt_priority:0}}",
        "app {id:'a' policy {placement {cpus:'0'} rt_priority:100}}",
        &format!("{text} {text}"),
    ] {
        assert!(config::parse(invalid).is_err(), "{invalid}");
    }
    // Act
    let omitted = config::parse("app { id:'a' policy {placement {cpus:'0'}} }").unwrap();
    // Assert: parsing preserves protobuf presence.
    assert_eq!(omitted.app[0].policy.as_ref().unwrap().rt_priority, None);
}
#[test]
fn shipped_deployments_are_independent() {
    // Arrange
    for name in ["base", "lab", "production", "test"] {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("configs/{name}.pbtxt"));
        // Act
        let deployment = config::load(&path).unwrap();
        // Assert
        assert_eq!(deployment.app.len(), 2);
    }
}
#[test]
fn targets_keep_exact_placement_and_rt_semantics() {
    let fragment =
        config::parse("app { id: 'a' policy { placement { cpus: '4,5-7,9' } rt_priority: 30 } }")
            .unwrap();
    let goal = Goal::from_proto(fragment.app[0].policy.as_ref().unwrap()).unwrap();
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
