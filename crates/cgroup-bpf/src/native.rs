use super::*;
use libbpf_rs::{
    CgroupIterOpts, CgroupIterOrder, Iter, IterOpts, Link, MapCore, Object, ObjectBuilder,
};
use std::fs::File;
use std::io::Read;
use std::os::fd::{AsFd, AsRawFd, RawFd};

#[repr(C)]
#[derive(Default)]
struct Control {
    capacity: u32,
    cpus: u32,
    count: u32,
    overflow: u32,
}

/// Owns preallocated kernel arrays. Successful collection performs no Rust heap
/// allocation. Iterator creation still allocates kernel fd/seq-file state.
/// Reads are a bounded traversal, NOT an atomic cross-CPU or cross-cgroup snapshot.
pub struct Backend {
    link: Link,
    _object: Object,
    records_fd: RawFd,
    control_fd: RawFd,
    capacity: usize,
    cpus: u32,
}
impl Backend {
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 || capacity > u32::MAX as usize {
            return Err(Error::InvalidCapacity);
        }
        if !std::path::Path::new("/sys/fs/cgroup/cgroup.controllers").exists() {
            return Err(Error::Unsupported(
                "cgroup v2 root is required at /sys/fs/cgroup",
            ));
        }
        if !std::path::Path::new("/sys/kernel/btf/vmlinux").exists() {
            return Err(Error::Unsupported(
                "kernel BTF is unavailable at /sys/kernel/btf/vmlinux",
            ));
        }
        let cpus = libbpf_rs::num_possible_cpus()?;
        if cpus == 0 || cpus > 4096 {
            return Err(Error::Unsupported(
                "supported possible CPU count is 1..=4096",
            ));
        }
        let mut builder = ObjectBuilder::default();
        let mut open =
            builder.open_memory(include_bytes!(concat!(env!("OUT_DIR"), "/monitor.bpf.o")))?;
        open.maps_mut()
            .find(|m| m.name() == "records")
            .ok_or(Error::Unsupported("BPF records map missing"))?
            .set_max_entries(capacity as u32)?;
        let object = load_bpf_object(open)?;
        let link = attach_cgroup_iterator(&object)?;
        let records_fd = object
            .maps()
            .find(|m| m.name() == "records")
            .ok_or(Error::Unsupported("BPF records map missing"))?
            .as_fd()
            .as_raw_fd();
        let control_fd = object
            .maps()
            .find(|m| m.name() == "control")
            .ok_or(Error::Unsupported("BPF control map missing"))?
            .as_fd()
            .as_raw_fd();
        Ok(Self {
            link,
            _object: object,
            records_fd,
            control_fd,
            capacity,
            cpus: cpus as u32,
        })
    }

    pub fn collect_into(&mut self, output: &mut Vec<CgroupRecord>) -> Result<CollectionStats> {
        if output.capacity() < self.capacity {
            return Err(Error::OutputCapacity {
                required: self.capacity,
                actual: output.capacity(),
            });
        }
        output.clear();
        let mut ctl = Control {
            capacity: self.capacity as u32,
            cpus: self.cpus,
            ..Default::default()
        };
        let key = 0u32;
        // SAFETY: fds are retained by _object, and key/value sizes match the C ABI.
        let rc = unsafe {
            libbpf_rs::libbpf_sys::bpf_map_update_elem(
                self.control_fd,
                (&key as *const u32).cast(),
                (&ctl as *const Control).cast(),
                0,
            )
        };
        if rc != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let mut iter = Iter::new(&self.link)?;
        let mut sink = [0u8; 64];
        loop {
            match iter.read(&mut sink) {
                Ok(0) => break,
                Ok(_) => return Err(Error::Unsupported("unexpected iterator stream bytes")),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            }
        }
        lookup(self.control_fd, 0, &mut ctl)?;
        if ctl.count as usize > self.capacity {
            return Err(Error::Unsupported("invalid BPF record count"));
        }
        for index in 0..ctl.count {
            let mut record = CgroupRecord::default();
            lookup(self.records_fd, index, &mut record)?;
            output.push(record);
        }
        Ok(CollectionStats {
            records: output.len(),
            overflow: ctl.overflow as u64,
        })
    }
}
fn lookup<T>(fd: RawFd, key: u32, value: &mut T) -> Result<()> {
    // SAFETY: callers exclusively use the matching array-map value ABI; neither
    // syscall retains the provided stack pointers.
    let rc = unsafe {
        libbpf_rs::libbpf_sys::bpf_map_lookup_elem(
            fd,
            (&key as *const u32).cast(),
            (value as *mut T).cast(),
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

// Declarative Linux capability requirements; no automatic grant or preprocessing.
// #[cap(CAP_BPF, CAP_PERFMON)]
fn load_bpf_object(open: libbpf_rs::OpenObject) -> Result<Object> {
    Ok(open.load()?)
}

// #[cap(CAP_BPF, CAP_PERFMON)]
fn attach_cgroup_iterator(object: &Object) -> Result<Link> {
    let root = File::open("/sys/fs/cgroup")?;
    let mut opts = CgroupIterOpts::from_fd(root.as_fd());
    opts.order = CgroupIterOrder::DescendantsPre;
    let link = object
        .progs_mut()
        .find(|p| p.name() == "collect")
        .ok_or(Error::Unsupported("BPF iterator missing"))?
        .attach_iter_with_opts(IterOpts::Cgroup(opts))?;
    Ok(link)
}
