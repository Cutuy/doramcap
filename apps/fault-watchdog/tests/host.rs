use fault_aggregation::{Freshness, Health};
use fault_protocol::{AggregationFrame, AggregationPayload, ReporterFrame, ReporterPayload};
use fault_reporter::{Definition, Reporter, Text};
use fault_watchdog::{
    config::{self, WatchdogConfig},
    engine::Engine,
    sinks::{Output, Packet, Sink},
};
use pbtxt_config::Payload;
use std::time::{Duration, Instant};
fn cfg() -> WatchdogConfig {
    WatchdogConfig {
        inputs: vec!["a".into(), "b".into()],
        ..Default::default()
    }
    .normalize()
    .unwrap()
}
fn frame(payload: ReporterPayload) -> Vec<u8> {
    fault_protocol::encode_reporter(
        &ReporterFrame {
            published_unix_ns: None,
            payload,
        },
        Default::default(),
    )
    .unwrap()
}
fn view(engine: &Engine) -> fault_aggregation::View {
    match engine.snapshot(Duration::ZERO).payload {
        AggregationPayload::Snapshot(v) => v,
        _ => unreachable!(),
    }
}
#[test]
fn configuration_presence_features_and_budgets() {
    let c = config::load(std::path::Path::new("config/default.pbtxt")).unwrap();
    assert_eq!(c.source_timeout_ms, Some(3000));
    let mut base = cfg();
    WatchdogConfig {
        retry_count: Some(0),
        transport_allowance_ms: Some(0),
        ..Default::default()
    }
    .overlay(&mut base);
    assert_eq!(base.normalize().unwrap().retry_count, Some(0));
    for invalid in [
        WatchdogConfig {
            queue_capacity: Some(1000),
            ..cfg()
        },
        WatchdogConfig {
            max_frame_bytes: Some(1024),
            ..cfg()
        },
        WatchdogConfig {
            inputs: vec!["a".into(), "a".into()],
            ..cfg()
        },
    ] {
        assert!(invalid.normalize().is_err());
    }
    let c = WatchdogConfig {
        mcap: Some(config::pb::McapConfig {
            enabled: true,
            path: "x.mcap".into(),
            flush_ms: 100,
            ..Default::default()
        }),
        ..cfg()
    };
    assert_eq!(c.normalize().is_ok(), cfg!(feature = "mcap"));
    let c = WatchdogConfig {
        otel: Some(config::pb::OtelConfig {
            enabled: true,
            endpoint: "http://localhost:4318/v1/logs".into(),
            timeout_ms: 100,
            service_name: "test".into(),
        }),
        ..cfg()
    };
    assert_eq!(c.normalize().is_ok(), cfg!(feature = "otel"));
}
#[test]
fn registration_timeout_restart_and_collision() {
    let mut engine = Engine::new(cfg(), 10, 11).unwrap();
    let (r, mut p) = Reporter::new(1, Default::default()).unwrap();
    let mut def = Definition::new("test", "disk", "write").unwrap();
    def.policy.deadline = Some(Duration::from_millis(100));
    let c = r.register(def).unwrap();
    c.failure(Text::new("io").unwrap()).unwrap();
    assert!(
        engine
            .receive(
                "a",
                &frame(ReporterPayload::Batch(p.batch(10).unwrap())),
                Duration::ZERO
            )
            .is_err()
    );
    let bytes = frame(ReporterPayload::Snapshot(p.snapshot()));
    engine.receive("a", &bytes, Duration::ZERO).unwrap();
    assert!(engine.receive("b", &bytes, Duration::ZERO).is_err());
    assert!(engine.receive("a", b"bad", Duration::ZERO).is_err());
    engine.advance(Duration::from_secs(1)).unwrap();
    let v = view(&engine);
    let source = v.sources.iter().find(|s| s.source == 1).unwrap();
    assert_eq!(source.conditions[0].health, Health::Active);
    assert_eq!(source.conditions[0].freshness, Freshness::Stale);
    engine.advance(Duration::from_secs(6)).unwrap();
    assert!(
        !view(&engine)
            .sources
            .iter()
            .find(|s| s.source == 1)
            .unwrap()
            .connected
    );
    let (_r, p) = Reporter::new(2, Default::default()).unwrap();
    engine
        .receive(
            "a",
            &frame(ReporterPayload::Snapshot(p.snapshot())),
            Duration::from_secs(6),
        )
        .unwrap();
    assert!(!view(&engine).sources.iter().any(|s| s.source == 1));
    assert!(view(&engine).sources.iter().any(|s| s.source == 2));
}
#[test]
fn immediate_failure_publication_and_idle_snapshot() {
    let mut e = Engine::new(cfg(), 10, 11).unwrap();
    let (r, p) = Reporter::new(1, Default::default()).unwrap();
    let c = r
        .register(Definition::new("test", "disk", "write").unwrap())
        .unwrap();
    e.receive(
        "a",
        &frame(ReporterPayload::Snapshot(p.snapshot())),
        Duration::ZERO,
    )
    .unwrap();
    e.advance(Duration::ZERO).unwrap();
    e.publications(Duration::ZERO);
    c.failure(Text::new("io").unwrap()).unwrap();
    e.receive(
        "a",
        &frame(ReporterPayload::Snapshot(p.snapshot())),
        Duration::from_millis(1),
    )
    .unwrap();
    let frames = e.publications(Duration::from_millis(1));
    assert!(frames.iter().any(|f|matches!(&f.payload,AggregationPayload::Updates(b) if b.updates.iter().any(|u|u.state.is_some_and(|s|s.health==Health::Active)))));
    assert!(
        e.publications(Duration::from_secs(2))
            .iter()
            .any(|f| matches!(f.payload, AggregationPayload::Snapshot(_)))
    );
}
struct Gate {
    started: crossbeam_channel::Sender<()>,
    release: crossbeam_channel::Receiver<()>,
}
impl Sink for Gate {
    fn write(&mut self, _: &Packet) -> eyre::Result<()> {
        let _ = self.started.send(());
        let _ = self.release.recv();
        Ok(())
    }
}
#[test]
fn blocked_sink_bounds_loss_repair_and_shutdown() {
    let mut config = cfg();
    config.queue_capacity = Some(1);
    let e = Engine::new(config.clone(), 10, 11).unwrap();
    let (started, ready) = crossbeam_channel::unbounded();
    let (release, gate) = crossbeam_channel::unbounded();
    let out = Output::spawn(
        "test",
        &config,
        e.limits(),
        e.sink_check("test").unwrap(),
        move || {
            Ok(Box::new(Gate {
                started,
                release: gate,
            }))
        },
    )
    .unwrap();
    let f = e.snapshot(Duration::ZERO);
    assert!(out.offer(&f).unwrap());
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(out.offer(&f).unwrap());
    assert!(!out.offer(&f).unwrap());
    assert_eq!(out.observe().unwrap().dropped, 1);
    assert!(out.take_repair());
    assert!(
        out.finish(Instant::now() + Duration::from_millis(20))
            .is_err()
    );
    drop(release);
}
struct Capture(crossbeam_channel::Sender<AggregationFrame>);
impl Sink for Capture {
    fn write(&mut self, p: &Packet) -> eyre::Result<()> {
        self.0.send(fault_protocol::decode_aggregation(
            &p.bytes,
            Default::default(),
        )?)?;
        Ok(())
    }
}
#[test]
fn own_fault_is_filtered_other_sink_faults_survive() {
    let config = cfg();
    let mut e = Engine::new(config.clone(), 10, 11).unwrap();
    let own = e.sink_check("live").unwrap();
    let key = own.key();
    let other = e.sink_check("mcap").unwrap();
    own.failure(Text::new("live broken").unwrap()).unwrap();
    other.failure(Text::new("disk broken").unwrap()).unwrap();
    e.advance(Duration::ZERO).unwrap();
    let (tx, rx) = crossbeam_channel::unbounded();
    let out = Output::spawn("live", &config, e.limits(), own, move || {
        Ok(Box::new(Capture(tx)))
    })
    .unwrap();
    out.offer(&e.snapshot(Duration::ZERO)).unwrap();
    let f = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    if let AggregationPayload::Snapshot(v) = f.payload {
        let keys: Vec<_> = v
            .sources
            .iter()
            .flat_map(|s| s.conditions.iter().map(|s| s.key))
            .collect();
        assert!(!keys.contains(&key));
        assert!(keys.contains(&other.key()));
    } else {
        panic!()
    }
    out.finish(Instant::now() + Duration::from_secs(2)).unwrap();
}

struct Flaky {
    attempts: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    accepted: crossbeam_channel::Sender<()>,
}
impl Sink for Flaky {
    fn write(&mut self, _: &Packet) -> eyre::Result<()> {
        if self
            .attempts
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            < 2
        {
            eyre::bail!("temporary failure")
        }
        self.accepted.send(())?;
        Ok(())
    }
}
#[test]
fn retries_are_bounded_and_success_clears_failure() {
    let mut config = cfg();
    config.retry_delay_ms = Some(1);
    let e = Engine::new(config.clone(), 10, 11).unwrap();
    let attempts = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let worker = attempts.clone();
    let (tx, rx) = crossbeam_channel::unbounded();
    let out = Output::spawn(
        "retry",
        &config,
        e.limits(),
        e.sink_check("retry").unwrap(),
        move || {
            Ok(Box::new(Flaky {
                attempts: worker,
                accepted: tx,
            }))
        },
    )
    .unwrap();
    out.offer(&e.snapshot(Duration::ZERO)).unwrap();
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    out.finish(Instant::now() + Duration::from_secs(2)).unwrap();
    assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[test]
fn shutdown_closes_healthy_sink_while_another_is_blocked() {
    let config = cfg();
    let engine = Engine::new(config.clone(), 10, 11).unwrap();
    let (started, ready) = crossbeam_channel::unbounded();
    let (release, gate) = crossbeam_channel::unbounded();
    let blocked = Output::spawn(
        "blocked",
        &config,
        engine.limits(),
        engine.sink_check("blocked").unwrap(),
        move || {
            Ok(Box::new(Gate {
                started,
                release: gate,
            }))
        },
    )
    .unwrap();
    let (tx, _rx) = crossbeam_channel::unbounded();
    let healthy = Output::spawn(
        "healthy",
        &config,
        engine.limits(),
        engine.sink_check("healthy").unwrap(),
        move || Ok(Box::new(Capture(tx))),
    )
    .unwrap();
    blocked.offer(&engine.snapshot(Duration::ZERO)).unwrap();
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    let blocked = blocked.close();
    let healthy = healthy.close();
    healthy
        .finish(Instant::now() + Duration::from_secs(2))
        .unwrap();
    assert!(
        blocked
            .finish(Instant::now() + Duration::from_millis(10))
            .is_err()
    );
    drop(release);
}
