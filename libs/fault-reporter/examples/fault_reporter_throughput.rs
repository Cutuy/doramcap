//! Local smoke benchmark, not a real-time guarantee. Run with --release.
use fault_reporter::{Config, Definition, Observation, Reporter, Text};
use std::time::{Duration, Instant};
fn main() {
    const N: usize = 100_000;
    println!("observation_bytes={}", std::mem::size_of::<Observation>());
    for mode in ["drained", "burst", "overload", "limited-success"] {
        let (reporter, mut publisher) = Reporter::new(1, Config::default()).unwrap();
        let mut definition = Definition::new("benchmark", "", "sample").unwrap();
        if mode == "limited-success" {
            definition.policy.success_interval = Some(Duration::from_secs(1));
        }
        let check = reporter.register(definition).unwrap();
        let mut samples = Vec::with_capacity(N);
        for i in 0..N {
            let start = Instant::now();
            if mode == "limited-success" || i % 2 == 0 {
                check.success().unwrap();
            } else {
                check.failure(Text::EMPTY).unwrap();
            }
            samples.push(start.elapsed().as_nanos());
            if mode == "drained" || (mode == "burst" && i % 128 == 127) {
                while publisher.try_recv().is_some() {}
            }
        }
        samples.sort_unstable();
        println!(
            "{mode}: p50={}ns p99={}ns losses={:?}",
            samples[N / 2],
            samples[N * 99 / 100],
            reporter.losses()
        );
    }
}
