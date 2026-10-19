use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};
#[test]
fn recording_rotation_and_bounded_overload() {
    for slow in [false, true] {
        let dir =
            std::env::temp_dir().join(format!("doramcap-embedded-{}-{slow}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("capture.mcap");
        let mut command = Command::new(env!("CARGO_BIN_EXE_system-monitor"));
        command
            .args([
                "--backend",
                "synthetic",
                "--samples",
                "24",
                "--records",
                "32",
                "--record",
            ])
            .arg(&path)
            .args(["--record-rotate-bytes", "1"])
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        if slow {
            command.args(["--queue-slots", "1", "--publisher-delay-ms", "250"]);
        }
        let start = Instant::now();
        let mut child = command.spawn().unwrap();
        while child.try_wait().unwrap().is_none() {
            if start.elapsed() > Duration::from_secs(30) {
                child.kill().unwrap();
                panic!("recording test timed out");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        let log = String::from_utf8(output.stderr).unwrap();
        assert!(output.status.success(), "{log}");
        let mut sequences = Vec::new();
        let mut losses = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let data = std::fs::read(entry.unwrap().path()).unwrap();
            let mut dictionary_seen = false;
            for msg in mcap::MessageStream::new(&data).unwrap() {
                let msg = msg.unwrap();
                if msg.channel.topic == "doramcap/dictionary" {
                    telemetry_protocol::decode_dictionary(&msg.data).unwrap();
                    dictionary_seen = true;
                } else {
                    assert!(
                        dictionary_seen,
                        "rotated file needs its dictionary checkpoint"
                    );
                    let frame = telemetry_protocol::decode(&msg.data).unwrap();
                    assert_eq!(frame.snapshot.capture_unix_ns, msg.log_time);
                    sequences.push(frame.snapshot.sequence);
                    losses = losses.max(frame.snapshot.dropped_batches);
                }
            }
        }
        sequences.sort_unstable();
        if slow {
            assert!(sequences.len() < 24);
            assert!(losses > 0 || !log.contains("dropped=0"));
        } else {
            assert_eq!(sequences, (0..24).collect::<Vec<_>>());
            assert!(log.contains("dropped=0"));
        }
        eprintln!(
            "embedded slow={slow} elapsed_ms={} snapshots={} {log}",
            start.elapsed().as_millis(),
            sequences.len()
        );
        let collision = Command::new(env!("CARGO_BIN_EXE_system-monitor"))
            .args(["--backend", "synthetic", "--samples", "1", "--record"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(!collision.status.success());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
