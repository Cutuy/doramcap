use fault_protocol::{AggregationFrame, AggregationPayload};
use fault_reconciliation::Engine;
use fault_reporter::Text;
use fault_watchdog::{
    config::{self, WatchdogConfig},
    sinks::{Output, Packet, Sink},
};
use std::time::{Duration, Instant};
fn cfg() -> WatchdogConfig {
    config::load(std::path::Path::new("config/default.pbtxt")).unwrap()
}
#[test]
fn configuration_presence_features_and_budgets() {
    let c = config::load(std::path::Path::new("config/default.pbtxt")).unwrap();
    assert_eq!(c.source_timeout_ms, Some(3000));
    // Arrange
    let base = WatchdogConfig {
        retry_count: Some(0),
        transport_allowance_ms: Some(0),
        ..cfg()
    };
    // Act / Assert
    assert_eq!(base.settings(vec![]).unwrap().inputs.len(), 0);
    assert_eq!(base.retry_count, Some(0));
    assert!(base.settings(vec!["a".into(), "a".into()]).is_err());
    assert!(WatchdogConfig::default().validate().is_err());
    for invalid in [
        WatchdogConfig {
            queue_capacity: Some(1000),
            ..cfg()
        },
        WatchdogConfig {
            max_frame_bytes: Some(1024),
            ..cfg()
        },
    ] {
        assert!(invalid.settings(vec!["a".into(), "b".into()]).is_err());
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
    assert_eq!(
        c.settings(vec!["a".into(), "b".into()]).is_ok(),
        cfg!(feature = "mcap")
    );
    let c = WatchdogConfig {
        otel: Some(config::pb::OtelConfig {
            enabled: true,
            endpoint: "http://localhost:4318/v1/logs".into(),
            timeout_ms: 100,
            service_name: "test".into(),
        }),
        ..cfg()
    };
    assert_eq!(
        c.settings(vec!["a".into(), "b".into()]).is_ok(),
        cfg!(feature = "otel")
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
    let e = Engine::new(
        config.settings(vec!["a".into(), "b".into()]).unwrap(),
        10,
        11,
    )
    .unwrap();
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
    let mut e = Engine::new(
        config.settings(vec!["a".into(), "b".into()]).unwrap(),
        10,
        11,
    )
    .unwrap();
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
    let e = Engine::new(
        config.settings(vec!["a".into(), "b".into()]).unwrap(),
        10,
        11,
    )
    .unwrap();
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
    let engine = Engine::new(
        config.settings(vec!["a".into(), "b".into()]).unwrap(),
        10,
        11,
    )
    .unwrap();
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
