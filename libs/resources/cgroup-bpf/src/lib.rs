//! Native inclusive cgroup CPU accounting. No userspace ancestor summation.
//! The BPF backend requires Linux >= 6.8, BTF and privileged tracing access.
use std::fmt;

pub const VALID: u32 = 1;
/// Global CPU accounting for the hierarchy root.
pub const ROOT: u32 = 2;
pub const READ_ERROR: u32 = 4;
/// Native user/system counters have not undergone cpu.stat's cputime_adjust.
pub const RAW_USER_SYSTEM: u32 = 8;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct CgroupRecord {
    pub id: u64,
    pub parent_id: u64,
    pub cpu: u32,
    pub flags: u32,
    pub runtime_ns: u64,
    pub user_ns: u64,
    pub system_ns: u64,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CollectionStats {
    pub records: usize,
    pub overflow: u64,
}

#[derive(Debug)]
pub enum Error {
    FeatureDisabled,
    InvalidCapacity,
    OutputCapacity {
        required: usize,
        actual: usize,
    },
    Unsupported(&'static str),
    Io(std::io::Error),
    #[cfg(feature = "bpf")]
    Bpf(libbpf_rs::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FeatureDisabled => {
                f.write_str("native acquisition requires the cgroup-bpf/bpf feature")
            }
            Self::InvalidCapacity => f.write_str("record capacity must be between 1 and u32::MAX"),
            Self::OutputCapacity { required, actual } => write!(
                f,
                "output must be preallocated for {required} records (has {actual})"
            ),
            Self::Unsupported(s) => f.write_str(s),
            Self::Io(e) => write!(f, "cgroup acquisition I/O: {e}"),
            #[cfg(feature = "bpf")]
            Self::Bpf(e) => write!(
                f,
                "BPF load/attach/read failed (check BTF, cgroup iterator, rstat flush kfunc, per-CPU offsets and CAP_BPF/CAP_PERFMON/CAP_SYSLOG): {e}"
            ),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
#[cfg(feature = "bpf")]
impl From<libbpf_rs::Error> for Error {
    fn from(e: libbpf_rs::Error) -> Self {
        Self::Bpf(e)
    }
}
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(feature = "bpf")]
mod native;
#[cfg(feature = "bpf")]
pub use native::Backend;

#[cfg(not(feature = "bpf"))]
pub struct Backend;
#[cfg(not(feature = "bpf"))]
impl Backend {
    pub fn new(_capacity: usize) -> Result<Self> {
        Err(Error::FeatureDisabled)
    }
    pub fn collect_into(&mut self, _output: &mut Vec<CgroupRecord>) -> Result<CollectionStats> {
        Err(Error::FeatureDisabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn c_abi_layout() {
        assert_eq!(std::mem::size_of::<CgroupRecord>(), 48);
        assert_eq!(std::mem::offset_of!(CgroupRecord, runtime_ns), 24);
    }
    #[cfg(not(feature = "bpf"))]
    #[test]
    fn disabled_is_explicit() {
        assert!(matches!(Backend::new(10), Err(Error::FeatureDisabled)));
    }
}
