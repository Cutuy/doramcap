//! Privileged native probe, driven with a timeout by tests/validate_native.py.
#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use resource_policy::{Goal, linux::*, proto};
    use std::{fs, path::Path};
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--descriptor") {
        fs::write(
            &args[2],
            pbtxt_config::descriptor_pool::<proto::Policy>()?.encode_to_vec(),
        )?;
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--wire") {
        use prost::Message;
        let fragment = resource_policy::config::parse_fragment(&fs::read_to_string(&args[2])?)?;
        fs::write(&args[3], fragment.to_proto().encode_to_vec())?;
        return Ok(());
    }
    let before = inspect_current_thread()?;
    let cpu = args.get(3).map(String::as_str).unwrap_or("0");
    let policy = |priority| {
        Goal::from_proto(&proto::Policy {
            placement: Some(proto::Placement { cpus: cpu.into() }),
            rt_priority: Some(priority),
        })
    };
    if args.get(1).map(String::as_str) == Some("--contend") {
        let priority = args[2].parse()?;
        apply_current_thread(&policy(priority)?)?;
        let start = std::time::Instant::now();
        let mut work = 0_u64;
        while start.elapsed() < std::time::Duration::from_millis(200) {
            work = std::hint::black_box(work.wrapping_add(1));
        }
        assert!(work > 0);
        assert_eq!(inspect_current_thread()?.priority, priority as i32);
        apply_current_thread(&policy(0)?)?;
        println!("PASS bounded contention RR priority {priority}: {work} iterations");
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--denied") {
        // Drop privilege after exec so private CI build directories remain usable.
        // SAFETY: this dedicated single-threaded probe changes only its own credentials.
        unsafe {
            let limit = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            assert_eq!(libc::setrlimit(libc::RLIMIT_RTPRIO, &limit), 0);
            assert_eq!(libc::setgroups(0, std::ptr::null()), 0);
            assert_eq!(libc::setgid(65534), 0);
            assert_eq!(libc::setuid(65534), 0);
        }
        let error = apply_current_thread(&policy(1)?).expect_err("unprivileged RT must fail");
        assert!(
            matches!(error.cause, resource_policy::Error::Io { source, .. } if source.raw_os_error() == Some(libc::EPERM))
        );
        assert_eq!(error.observed.unwrap().priority, 0);
        println!("PASS permission denial with observed partial state");
        return Ok(());
    }
    let parent = Path::new(args.get(1).ok_or("expected parent and instance")?);
    let name = args.get(2).ok_or("expected instance")?;
    if args.get(4).map(String::as_str) == Some("reject") {
        assert!(Instance::prepare(parent, name, &policy(0)?).is_err());
        assert!(!parent.join(name).exists());
        println!("PASS parent restriction rejected before creating leaf");
        return Ok(());
    }
    let priority = args.get(4).ok_or("expected priority")?.parse()?;
    let goal = policy(priority)?;
    let saved_group = fs::read_to_string("/proc/self/cgroup")?
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .ok_or("cgroup v2 required")?
        .to_owned();
    let saved_group = Path::new("/sys/fs/cgroup").join(saved_group.trim_start_matches('/'));
    assert!(Instance::prepare(parent, "../escape", &goal).is_err());
    let instance = Instance::prepare(parent, name, &goal)?;
    assert!(
        Instance::prepare(parent, name, &goal).is_err(),
        "must never adopt an existing instance"
    );
    let outcome = (|| -> Result<(), Box<dyn std::error::Error>> {
        let applied = instance.bootstrap_current_process(&goal)?;
        assert_eq!(applied.after.cpus.canonical(), cpu);
        assert_eq!(applied.after.priority, priority as i32);
        assert!(
            instance
                .bootstrap_current_process(&goal)?
                .changed
                .is_empty()
        );
        let parent_cpus = fs::read_to_string(parent.join("cpuset.cpus.effective"))?
            .trim()
            .to_owned();
        let expanded = Goal::from_proto(&proto::Policy {
            placement: Some(proto::Placement {
                cpus: parent_cpus.clone(),
            }),
            rt_priority: Some(priority),
        })?;
        assert_eq!(
            instance.bootstrap_current_process(&expanded)?.after.cpus,
            *expanded.cpus()
        );
        assert_eq!(
            instance.bootstrap_current_process(&goal)?.after.cpus,
            *goal.cpus()
        );
        fs::write(instance.path().join("cpuset.cpus"), &parent_cpus)?;
        let repaired = instance.bootstrap_current_process(&goal)?;
        assert!(repaired.changed.contains(&"cpuset"));
        assert_eq!(
            fs::read_to_string(instance.path().join("cpuset.cpus.effective"))?.trim(),
            cpu
        );
        println!("PASS explicit goal expansion, narrowing, and cgroup drift repair");
        assert!(
            instance.remove().is_err(),
            "populated group must remain owned"
        );
        let expected = applied.after.clone();
        std::thread::spawn(move || {
            assert_eq!(inspect_current_thread().unwrap(), expected);
        })
        .join()
        .unwrap();
        let child = std::process::Command::new("python3").args([
            "-c", "import os,sys; assert os.sched_getaffinity(0)=={int(sys.argv[1])}; assert os.sched_getparam(0).sched_priority==int(sys.argv[2]); assert open('/proc/self/cgroup').read().strip().endswith(sys.argv[3])",
            cpu, &priority.to_string(), name,
        ]).status()?;
        assert!(child.success(), "exec child must inherit policy and cgroup");
        // Deliberate thread policy drift followed by explicit reconciliation.
        apply_current_thread(&policy(0)?)?;
        assert_eq!(apply_current_thread(&goal)?.after.priority, priority as i32);
        // Existing workers make process-bootstrap invalid even if already attached.
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || rx.recv().unwrap());
        assert!(instance.bootstrap_current_process(&goal).is_err());
        tx.send(())?;
        worker.join().unwrap();
        println!(
            "PASS bootstrap, idempotence, drift, inheritance, ownership: {}",
            instance.path().display()
        );
        Ok(())
    })();
    // Demote before restoring membership, including when validation failed.
    let demote = apply_current_thread(&policy(0)?);
    let detach = fs::write(
        saved_group.join("cgroup.procs"),
        std::process::id().to_string(),
    );
    let restore = Goal::from_proto(&proto::Policy {
        placement: Some(proto::Placement {
            cpus: before.cpus.canonical(),
        }),
        rt_priority: Some(0),
    })?;
    demote?;
    detach?;
    apply_current_thread(&restore)?;
    instance.remove()?;
    outcome
}
#[cfg(not(target_os = "linux"))]
fn main() {
    panic!("Linux required")
}
