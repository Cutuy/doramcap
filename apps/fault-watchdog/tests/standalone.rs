use fault_aggregation::{Freshness, Health};
use fault_protocol::{AggregationPayload, ReporterFrame, ReporterPayload};
use fault_reconciliation::Engine;
use fault_reporter::{Definition, Reporter, Text};
use fault_watchdog::config;
use std::{path::Path, time::Duration};

fn resolved_text(path: &Path) -> pbtxt_config::Result<String> {
    pbtxt_config::resolve::<config::WatchdogConfig>(path, &[config::APP])?.fragment_text()
}

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
    let input = config.inputs[0].clone();
    let mut engine = Engine::new(config.settings().unwrap(), 43, 44).unwrap();
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
    for (name, enabled) in [
        ("watchdog.pbtxt", true),
        ("watchdog-mcap.pbtxt", cfg!(feature = "mcap")),
        ("watchdog-otel.pbtxt", cfg!(feature = "otel")),
    ] {
        // Arrange
        let path = Path::new("deploy/standalone/config").join(name);

        // Act
        let config = config::load(&path);

        // Assert
        assert_eq!(config.is_ok(), enabled, "{name}: {config:?}");
    }
}

#[test]
fn resolved_profile_loads_alone_with_the_config_of_its_includes() {
    // Arrange
    let source = Path::new("deploy/standalone/config/watchdog-otel.pbtxt");
    if !cfg!(feature = "otel") {
        assert!(resolved_text(source).is_err());
        return;
    }
    let output = std::env::temp_dir().join(format!("watchdog-resolved-{}", std::process::id()));

    // Act
    let text = resolved_text(source).unwrap();
    std::fs::write(&output, &text).unwrap();
    let alone = config::load(&output);
    let again = resolved_text(&output);
    std::fs::remove_file(&output).unwrap();

    // Assert
    assert_eq!(alone.unwrap(), config::load(source).unwrap());
    assert_eq!(again.unwrap(), text);
}
