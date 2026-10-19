//! One-shot capability/load check. Requires native BPF privilege, never elevates.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let capacity = 65536;
    let mut backend = cgroup_bpf::Backend::new(capacity)?;
    let mut rows = Vec::with_capacity(capacity);
    let stats = backend.collect_into(&mut rows)?;
    println!("records={} overflow={}", stats.records, stats.overflow);
    assert!(
        rows.iter()
            .any(|row| row.flags & cgroup_bpf::VALID != 0 && row.runtime_ns > 0)
    );
    assert!(
        rows.iter()
            .all(|row| row.flags & cgroup_bpf::READ_ERROR == 0)
    );
    for row in rows
        .iter()
        .filter(|row| row.flags & cgroup_bpf::VALID != 0)
        .take(8)
    {
        println!("{row:?}");
    }
    Ok(())
}
