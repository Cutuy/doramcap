use super::*;
use libbpf_rs::{Link, MapCore, Object, ObjectBuilder};
use std::os::fd::{AsFd, AsRawFd, RawFd};

/// Experimental, explicitly opted-in task diagnostics. Successful construction
/// establishes attachability, not complete runtime/IRQ/BSP qualification.
/// Entries are retained for this attachment's lifetime; capacity exhaustion is
/// observable. Snapshots are non-atomic bounded traversals of cumulative counters.
pub struct Backend {
    _link: Link,
    _object: Object,
    tasks_fd: RawFd,
    losses_fd: RawFd,
    capacity: usize,
}
impl Backend {
    pub fn new_experimental(capacity: usize) -> Result<Self> {
        if capacity == 0 || capacity > u32::MAX as usize {
            return Err(Error::InvalidCapacity);
        }
        if !std::path::Path::new("/sys/fs/cgroup/cgroup.controllers").exists() {
            return Err(Error::Unsupported(
                "task diagnostics require cgroup v2 at /sys/fs/cgroup",
            ));
        }
        if !std::path::Path::new("/sys/kernel/btf/vmlinux").exists() {
            return Err(Error::Unsupported("task diagnostics require kernel BTF"));
        }
        let mut builder = ObjectBuilder::default();
        let mut open =
            builder.open_memory(include_bytes!(concat!(env!("OUT_DIR"), "/tasks.bpf.o")))?;
        open.maps_mut()
            .find(|map| map.name() == "tasks")
            .ok_or(Error::Unsupported("task map missing"))?
            .set_max_entries(capacity as u32)?;
        let object = load_bpf_object(open)?;
        let tasks_fd = object
            .maps()
            .find(|map| map.name() == "tasks")
            .ok_or(Error::Unsupported("task map missing"))?
            .as_fd()
            .as_raw_fd();
        let losses_fd = object
            .maps()
            .find(|map| map.name() == "losses")
            .ok_or(Error::Unsupported("task losses map missing"))?
            .as_fd()
            .as_raw_fd();
        let link = attach_runtime_probe(&object)?;
        Ok(Self {
            _link: link,
            _object: object,
            tasks_fd,
            losses_fd,
            capacity,
        })
    }

    /// Pre-reserve `capacity` records before calling. No per-record heap buffers.
    /// No deletion occurs while attached, so traversal cannot restart on a
    /// deleted key. Concurrent insertions may first appear in the next snapshot.
    pub fn collect_into(&mut self, output: &mut Vec<TaskRecord>) -> Result<Losses> {
        if output.capacity() < self.capacity {
            return Err(Error::OutputCapacity {
                required: self.capacity,
                actual: output.capacity(),
            });
        }
        output.clear();
        let mut previous: Option<TaskIdentity> = None;
        for _ in 0..self.capacity {
            let mut key = TaskIdentity::default();
            let ptr = previous
                .as_ref()
                .map_or(std::ptr::null(), |p| (p as *const TaskIdentity).cast());
            // SAFETY: FDs owned by object; C map key/value sizes are fixed and
            // checked by ABI tests. Syscalls do not retain stack pointers.
            let rc = unsafe {
                libbpf_rs::libbpf_sys::bpf_map_get_next_key(
                    self.tasks_fd,
                    ptr,
                    (&mut key as *mut TaskIdentity).cast(),
                )
            };
            if rc != 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::NotFound {
                    break;
                }
                return Err(error.into());
            }
            let mut runtime = 0u64;
            let rc = unsafe {
                libbpf_rs::libbpf_sys::bpf_map_lookup_elem(
                    self.tasks_fd,
                    (&key as *const TaskIdentity).cast(),
                    (&mut runtime as *mut u64).cast(),
                )
            };
            if rc != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            if key.flags & KERNEL_THREAD == 0 {
                return Err(Error::Unsupported(
                    "kernel-only BPF map contained a user task",
                ));
            }
            output.push(TaskRecord {
                identity: key,
                charged_runtime_ns: runtime,
            });
            previous = Some(key);
        }
        let zero = 0u32;
        let mut losses = Losses::default();
        let rc = unsafe {
            libbpf_rs::libbpf_sys::bpf_map_lookup_elem(
                self.losses_fd,
                (&zero as *const u32).cast(),
                (&mut losses as *mut Losses).cast(),
            )
        };
        if rc != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(losses)
    }
}

// Declarative Linux capability requirements; no automatic grant or preprocessing.
// #[cap(CAP_BPF, CAP_PERFMON)]
fn load_bpf_object(open: libbpf_rs::OpenObject) -> Result<Object> {
    Ok(open.load()?)
}

// #[cap(CAP_BPF, CAP_PERFMON)]
fn attach_runtime_probe(object: &Object) -> Result<Link> {
    let link = object
        .progs_mut()
        .find(|prog| prog.name() == "charge")
        .ok_or(Error::Unsupported("task runtime probe missing"))?
        .attach()?;
    Ok(link)
}
