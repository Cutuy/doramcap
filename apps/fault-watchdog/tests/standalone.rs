use fault_aggregation::{Freshness, Health};
use fault_protocol::{AggregationPayload, ReporterFrame, ReporterPayload};
use fault_reconciliation::Engine;
use fault_reporter::{Definition, Reporter, Text};
use fault_watchdog::config;
use std::{path::Path, time::Duration};

fn source(engine: &Engine, now: Duration) -> fault_aggregation::SourceView {
    match engine.snapshot(now).payload {
        AggregationPayload::Snapshot(view) => view
            .sources
            .into_iter()
            .find(|source| source.source == 42)
            .unwrap(),
        _ => unreachable!(),
    }
}

#[test]
fn standalone_config_accepts_external_reporter_and_retains_disconnected_faults() {
    // Arrange
    let config = config::load(Path::new("deploy/standalone/config/watchdog.pbtxt")).unwrap();
    let timeout = Duration::from_millis(config.source_timeout_ms.unwrap().into());
    let input = "workload_faults".to_string();
    let mut engine = Engine::new(config.settings(vec![input.clone()]).unwrap(), 43, 44).unwrap();
    let (reporter, publisher) = Reporter::new(42, Default::default()).unwrap();
    let condition = reporter
        .register(Definition::new("workload", "disk", "write").unwrap())
        .unwrap();
    condition.failure(Text::new("disk full").unwrap()).unwrap();
    let bytes = fault_protocol::encode_reporter(
        &ReporterFrame {
            published_unix_ns: Some(1234),
            payload: ReporterPayload::Snapshot(publisher.snapshot()),
        },
        Default::default(),
    )
    .unwrap();

    // Act
    engine.receive(&input, &bytes, Duration::ZERO).unwrap();

    // Assert
    let connected = source(&engine, Duration::ZERO);
    assert!(connected.connected);
    assert_eq!(connected.conditions[0].health, Health::Active);

    // Act
    engine.advance(timeout).unwrap();

    // Assert
    let timed_out = source(&engine, timeout);
    assert!(!timed_out.connected);
    assert_eq!(timed_out.conditions[0].health, Health::Active);
    assert_eq!(timed_out.conditions[0].freshness, Freshness::Stale);

    // Act
    engine.receive(&input, &bytes, timeout).unwrap();
    engine.disconnect(&input, timeout).unwrap();

    // Assert
    let disconnected = source(&engine, timeout);
    assert!(!disconnected.connected);
    assert_eq!(disconnected.conditions[0].health, Health::Active);
    assert_eq!(disconnected.conditions[0].freshness, Freshness::Stale);
}

#[test]
fn standalone_sink_configs_use_existing_feature_gates() {
    // Arrange: Console mounts one profile file, with no siblings beside it.
    let directory = std::env::temp_dir().join(format!("watchdog-profiles-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    for (name, enabled) in [
        ("watchdog.pbtxt", true),
        ("watchdog-mcap.pbtxt", cfg!(feature = "mcap")),
        ("watchdog-otel.pbtxt", cfg!(feature = "otel")),
    ] {
        let path = directory.join(name);
        std::fs::copy(Path::new("deploy/standalone/config").join(name), &path).unwrap();

        // Act
        let config = config::load(&path);
        std::fs::remove_file(&path).unwrap();

        // Assert
        assert_eq!(config.is_ok(), enabled, "{name}: {config:?}");
    }
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn zero_inputs_publish_local_diagnostics_and_excess_inputs_fail() {
    // Arrange
    let config = config::load(Path::new("deploy/standalone/config/watchdog.pbtxt")).unwrap();
    // Act
    let mut engine = Engine::new(config.settings(vec![]).unwrap(), 43, 44).unwrap();
    engine.advance(Duration::ZERO).unwrap();
    // Assert
    assert!(matches!(
        engine.snapshot(Duration::ZERO).payload,
        AggregationPayload::Snapshot(_)
    ));
    // Act / Assert
    let inputs = (0..config.max_sources.unwrap())
        .map(|index| format!("health_{index}"))
        .collect();
    assert!(
        config
            .settings(inputs)
            .unwrap_err()
            .to_string()
            .contains("max_sources")
    );
}

#[test]
fn standalone_profiles_fit_the_declared_health_fan_in() {
    // Arrange
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("deploy/standalone/config");
    for (name, enabled) in [
        ("watchdog", true),
        ("watchdog-mcap", cfg!(feature = "mcap")),
        ("watchdog-otel", cfg!(feature = "otel")),
    ] {
        // Act
        let config = config::load(&root.join(format!("{name}.pbtxt")));
        // Assert
        assert_eq!(config.is_ok(), enabled, "{name}");
        if let Ok(config) = config {
            // Act / Assert: the reserved local source plus every configured external source fit.
            let inputs = (0..config.max_sources.unwrap() - 1)
                .map(|id| format!("health_{id}"))
                .collect();
            assert!(config.settings(inputs).is_ok(), "{name}");
        }
    }
}
