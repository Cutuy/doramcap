//! Compile-time Console policy; deployment paths and images come from `ConsoleConfig`.
//! Retention and byte limits also govern terminal checkpoint validation.
use std::time::Duration;

/// Ordinary commands queued before dispatch reports Busy; control has a separate lane.
pub const COMMAND_QUEUE_CAPACITY: usize = 32;
pub const CONTROL_QUEUE_CAPACITY: usize = 8;
/// Pending Start subscribers, including the first request; closed receivers are pruned.
pub const MAX_START_WAITERS: usize = 32;
/// Workflow events queued before the observer increments its dropped-event counter.
pub const EVENT_QUEUE_CAPACITY: usize = 512;
pub const MAX_EVENT_BYTES: usize = 16 * 1024;
/// Invocation rows retained; completed periodic rows are evicted before lifecycle rows.
pub const MAX_PROGRESS_ROWS: usize = 512;
pub const MAX_REQUEST_KEY_BYTES: usize = 128;
pub const READINESS_SECONDS: u32 = 60;
pub const STOP_SECONDS: u32 = 15;

pub const DEFAULT_SAMPLE_INTERVAL_MS: u32 = 100;
pub const MAX_METRIC_SAMPLES: usize = 1200;
pub const MAX_CPU_FRAME_BYTES: u64 = 8 * 1024 * 1024;
/// Header scan budget per tick; remaining frames are visited on later ticks.
pub const MAX_CPU_HEADERS_PER_TICK: usize = 4096;
pub const MAX_CPU_SNAPSHOTS: usize = 2;
pub const MAX_CPU_SNAPSHOT_BYTES: usize = 1024 * 1024;
pub const MAX_CPU_GROUP_NAMES: usize = 512;
pub const MAX_CPU_GROUP_NAME_CHARS: usize = 128;
pub const MAX_CGROUP_ANCESTORS: usize = 64;
pub const MIN_RATE_INTERVAL: Duration = Duration::from_millis(1);
pub const MAX_ACTIVITY_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_MARKER_BYTES: u64 = 64 * 1024;

/// Persisted identifiers use 128 random bits encoded as lowercase hexadecimal.
pub const RANDOM_ID_BYTES: usize = 16;
pub const RUN_ID_HEX_CHARS: usize = RANDOM_ID_BYTES * 2;
pub const MARKER_VERSION: u32 = 1;
