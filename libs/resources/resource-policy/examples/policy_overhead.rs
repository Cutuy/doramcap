//! Informational startup-path measurements; no latency guarantee or real-time threshold.
use resource_policy::config;
use std::{hint::black_box, path::Path, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("configs/test.pbtxt");
    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(config::resolve(&path, &["perception", "recorder"])?);
    }
    println!(
        "resolve_mean_us={:.2} iterations={iterations}",
        start.elapsed().as_secs_f64() * 1e6 / iterations as f64
    );
    #[cfg(target_os = "linux")]
    {
        // The benchmark supplies an explicit host-specific goal; the library never infers it.
        let goal = resource_policy::Goal::from_proto(&resource_policy::proto::Policy {
            placement: Some(resource_policy::proto::Placement {
                cpus: resource_policy::linux::inspect_current_thread()?
                    .cpus
                    .canonical(),
            }),
            rt_priority: Some(0),
        })?;
        resource_policy::linux::apply_current_thread(&goal)?;
        let start = Instant::now();
        for _ in 0..iterations {
            let applied = resource_policy::linux::apply_current_thread(&goal)?;
            assert!(applied.changed.is_empty());
            black_box(applied);
        }
        println!(
            "satisfied_apply_mean_us={:.2} iterations={iterations}",
            start.elapsed().as_secs_f64() * 1e6 / iterations as f64
        );
    }
    Ok(())
}
