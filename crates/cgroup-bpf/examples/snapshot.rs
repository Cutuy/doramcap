//! One-shot capability/load check. Requires native BPF privilege, never elevates.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let capacity = 65536;
    let mut backend = cgroup_bpf::Backend::new(capacity)?;
    let mut rows = Vec::with_capacity(capacity);
    let stats = backend.collect_into(&mut rows)?;
    println!("records={} overflow={}", stats.records, stats.overflow);
    for row in rows.iter().take(8) {
        println!("{row:?}");
    }
    Ok(())
}
