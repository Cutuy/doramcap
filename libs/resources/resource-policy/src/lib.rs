//! Declarative CPU placement and scheduling with explicit caller-side enforcement.
//! Resolve configuration before app initialization. Linux updates are non-atomic;
//! inspect `ApplyError` for partial progress. This crate starts no background work.
pub mod config;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod targets;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/resource_policy.v1.rs"));
}
pub const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/policy.bin"));

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
}
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn io(context: impl Into<String>, source: std::io::Error) -> Error {
    Error::Io {
        context: context.into(),
        source,
    }
}

/// Bounded logical CPU set. Supports IDs through 1,048,575, independently of cpu_set_t.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuSet(Vec<u32>);
impl CpuSet {
    pub const MAX_ID: u32 = 1_048_575;
    pub fn parse(text: &str) -> Result<Self> {
        if text.is_empty() || text.len() > 65536 {
            return Err(Error::Invalid("cpulist must contain 1..65536 bytes".into()));
        }
        // Bound expansion before invoking the dependency. Kernel stride/group extensions
        // are intentionally excluded from the initial portable authoring contract.
        let mut expansion = 0_u64;
        for token in text.split(',') {
            let mut parts = token.split('-');
            let parse = |s: &str| -> Result<u32> {
                if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(Error::Invalid(format!("invalid cpulist token {token:?}")));
                }
                s.parse::<u32>()
                    .ok()
                    .filter(|n| *n <= Self::MAX_ID)
                    .ok_or_else(|| Error::Invalid(format!("CPU ID exceeds {}", Self::MAX_ID)))
            };
            let start = parse(parts.next().unwrap())?;
            let end = parts.next().map(parse).transpose()?.unwrap_or(start);
            if parts.next().is_some() || end < start {
                return Err(Error::Invalid(format!("invalid cpulist range {token:?}")));
            }
            expansion += u64::from(end - start) + 1;
            if expansion > u64::from(Self::MAX_ID) + 1 {
                return Err(Error::Invalid("cpulist expansion exceeds bound".into()));
            }
        }
        let ids = cpulist::parse(text).map_err(|e| Error::Invalid(e.to_string()))?;
        Ok(Self(ids))
    }
    pub fn ids(&self) -> &[u32] {
        &self.0
    }
    pub fn canonical(&self) -> String {
        cpulist::emit(self.0.iter().copied())
    }
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.0.iter().all(|id| other.0.binary_search(id).is_ok())
    }
}

/// Validated goal state, independent of the caller's current placement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Goal {
    cpus: CpuSet,
    rt_priority: u32,
}
impl Goal {
    pub fn from_proto(policy: &proto::Policy) -> Result<Self> {
        validate_partial(policy)?;
        let placement = policy
            .placement
            .as_ref()
            .ok_or_else(|| Error::Invalid("resolved policy requires placement.cpus".into()))?;
        Ok(Self {
            cpus: CpuSet::parse(&placement.cpus)?,
            rt_priority: policy.rt_priority.unwrap_or(0),
        })
    }
    pub fn cpus(&self) -> &CpuSet {
        &self.cpus
    }
    pub fn rt_priority(&self) -> u32 {
        self.rt_priority
    }
    pub fn to_proto(&self) -> proto::Policy {
        proto::Policy {
            placement: Some(proto::Placement {
                cpus: self.cpus.canonical(),
            }),
            rt_priority: Some(self.rt_priority),
        }
    }
}

pub(crate) fn validate_partial(policy: &proto::Policy) -> Result<()> {
    if let Some(placement) = &policy.placement {
        CpuSet::parse(&placement.cpus)?;
    }
    if policy.rt_priority.unwrap_or(0) > 99 {
        return Err(Error::Invalid("rt_priority must be 0..99".into()));
    }
    Ok(())
}
