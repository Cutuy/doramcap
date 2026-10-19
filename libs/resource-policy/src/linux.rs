//! Linux caller enforcement. Call bootstrap while single-threaded and before app loading.
//! The owner must serialize process-wide membership changes and CPU hotplug handling.
use crate::{CpuSet, Error, Goal, Result, io};
use std::{
    fs, mem,
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadResources {
    pub cpus: CpuSet,
    pub scheduler: i32,
    pub priority: i32,
}

fn syscall(result: i32, context: &str) -> Result<()> {
    if result < 0 {
        Err(io(context, std::io::Error::last_os_error()))
    } else {
        Ok(())
    }
}
fn affinity() -> Result<CpuSet> {
    let mut words = vec![0 as libc::c_ulong; 16];
    loop {
        // SAFETY: storage is aligned to c_ulong and writable for the supplied byte count.
        let result = unsafe {
            libc::sched_getaffinity(
                0,
                words.len() * mem::size_of::<libc::c_ulong>(),
                words.as_mut_ptr().cast(),
            )
        };
        if result == 0 {
            let ids = words
                .iter()
                .enumerate()
                .flat_map(|(word, mask)| {
                    (0..libc::c_ulong::BITS).filter_map(move |bit| {
                        (mask & (1 << bit) != 0).then_some(word as u32 * libc::c_ulong::BITS + bit)
                    })
                })
                .collect::<Vec<_>>();
            return CpuSet::parse(&cpulist::emit(ids));
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINVAL)
            || words.len() * libc::c_ulong::BITS as usize > CpuSet::MAX_ID as usize
        {
            return Err(io("sched_getaffinity", error));
        }
        words.resize(words.len() * 2, 0);
    }
}
fn set_affinity(cpus: &CpuSet) -> Result<()> {
    let bits = libc::c_ulong::BITS as usize;
    let mut words = vec![
        0 as libc::c_ulong;
        (*cpus.ids().last().expect("nonempty") as usize + 1).div_ceil(bits)
    ];
    for &id in cpus.ids() {
        words[id as usize / bits] |= 1 << (id as usize % bits);
    }
    // SAFETY: aligned storage contains a bitmask of the specified length, for current thread.
    syscall(
        unsafe {
            libc::sched_setaffinity(
                0,
                words.len() * mem::size_of::<libc::c_ulong>(),
                words.as_ptr().cast(),
            )
        },
        "sched_setaffinity",
    )
}
fn set_scheduler(scheduler: i32, priority: i32) -> Result<()> {
    let parameter = libc::sched_param {
        sched_priority: priority,
    };
    // SAFETY: sched_param has the required layout and lives through the syscall.
    syscall(
        unsafe { libc::sched_setscheduler(0, scheduler, &parameter) },
        "sched_setscheduler",
    )
}
pub fn inspect_current_thread() -> Result<ThreadResources> {
    // SAFETY: pid 0 addresses the calling thread; output parameter is initialized and writable.
    let scheduler = unsafe { libc::sched_getscheduler(0) };
    syscall(scheduler, "sched_getscheduler")?;
    let mut parameter = libc::sched_param { sched_priority: 0 };
    // SAFETY: output parameter has the expected layout and lifetime.
    syscall(
        unsafe { libc::sched_getparam(0, &mut parameter) },
        "sched_getparam",
    )?;
    Ok(ThreadResources {
        cpus: affinity()?,
        scheduler,
        priority: parameter.sched_priority,
    })
}
#[derive(Clone, Debug)]
pub struct ThreadPlan {
    pub before: ThreadResources,
    pub desired: ThreadResources,
}
pub fn plan_current_thread(goal: &Goal) -> Result<ThreadPlan> {
    let before = inspect_current_thread()?;
    let cpus = goal.cpus().clone();
    let online = read_set(Path::new("/sys/devices/system/cpu/online"))?;
    if !cpus.is_subset_of(&online) {
        return Err(Error::Invalid(
            "requested CPUs include offline/unavailable IDs".into(),
        ));
    }
    let scheduler = if goal.rt_priority() == 0 {
        libc::SCHED_OTHER
    } else {
        libc::SCHED_RR
    };
    if scheduler == libc::SCHED_RR {
        // SAFETY: querying a constant, valid Linux scheduler class.
        let min = unsafe { libc::sched_get_priority_min(scheduler) };
        // SAFETY: same valid scheduler class.
        let max = unsafe { libc::sched_get_priority_max(scheduler) };
        if !(min..=max).contains(&(goal.rt_priority() as i32)) {
            return Err(Error::Invalid("RT priority outside host limits".into()));
        }
    }
    Ok(ThreadPlan {
        before,
        desired: ThreadResources {
            cpus,
            scheduler,
            priority: goal.rt_priority() as i32,
        },
    })
}
#[derive(Clone, Debug)]
pub struct Applied {
    pub before: ThreadResources,
    pub after: ThreadResources,
    pub changed: Vec<&'static str>,
}
#[derive(Debug, thiserror::Error)]
#[error("policy application failed after {completed:?}: {cause}; observed={observed:?}")]
pub struct ApplyError {
    pub cause: Error,
    pub completed: Vec<&'static str>,
    pub observed: Option<ThreadResources>,
}
impl ApplyError {
    fn new(cause: Error, completed: Vec<&'static str>) -> Self {
        Self {
            cause,
            completed,
            observed: inspect_current_thread().ok(),
        }
    }
}
/// Applies only differing fields, then verifies exact convergence. Failure can leave
/// changes in place; caller must inspect the report before choosing a recovery policy.
pub fn apply_current_thread(goal: &Goal) -> std::result::Result<Applied, ApplyError> {
    let plan = plan_current_thread(goal).map_err(|e| ApplyError::new(e, vec![]))?;
    let mut changed = vec![];
    let result = (|| -> Result<ThreadResources> {
        if plan.before.cpus != plan.desired.cpus {
            set_affinity(&plan.desired.cpus)?;
            changed.push("affinity");
            if affinity()? != plan.desired.cpus {
                return Err(Error::Invalid("effective affinity differs from requested CPUs (parent restriction or hotplug)".into()));
            }
        }
        if (plan.before.scheduler, plan.before.priority)
            != (plan.desired.scheduler, plan.desired.priority)
        {
            set_scheduler(plan.desired.scheduler, plan.desired.priority)?;
            changed.push("scheduler");
        }
        let after = inspect_current_thread()?;
        if after != plan.desired {
            return Err(Error::Invalid(
                "effective thread policy differs from goal".into(),
            ));
        }
        Ok(after)
    })();
    match result {
        Ok(after) => Ok(Applied {
            before: plan.before,
            after,
            changed,
        }),
        Err(cause) => Err(ApplyError::new(cause, changed)),
    }
}
fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| io(path.display().to_string(), e))
}
fn write(path: &Path, text: &str) -> Result<()> {
    fs::write(path, text).map_err(|e| io(path.display().to_string(), e))
}
fn read_set(path: &Path) -> Result<CpuSet> {
    CpuSet::parse(read(path)?.trim())
}

/// Host facts for a caller-authorized cgroup parent. Read-only; permissions are
/// finally established by operations, which can race delegation changes.
#[derive(Debug)]
pub struct Capabilities {
    pub online: CpuSet,
    pub parent_cpus: CpuSet,
    pub parent_memory_nodes: String,
    pub controllers: Vec<String>,
    pub rt_runtime_us: String,
}
pub fn probe_parent(parent: &Path) -> Result<Capabilities> {
    let directory = fs::File::open(parent).map_err(|e| io(parent.display().to_string(), e))?;
    let mut stat = mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: valid open descriptor and output storage; read only after success.
    syscall(
        unsafe { libc::fstatfs(directory.as_raw_fd(), stat.as_mut_ptr()) },
        "fstatfs delegated parent",
    )?;
    // SAFETY: successful fstatfs initialized the complete structure.
    if unsafe { stat.assume_init() }.f_type != libc::CGROUP2_SUPER_MAGIC {
        return Err(Error::Invalid(
            "delegated parent must be on cgroup v2".into(),
        ));
    }

    let controllers = read(&parent.join("cgroup.subtree_control"))?
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if !controllers.iter().any(|c| c == "cpuset") {
        return Err(Error::Invalid(
            "delegated parent must already enable cpuset for children".into(),
        ));
    }
    Ok(Capabilities {
        online: read_set(Path::new("/sys/devices/system/cpu/online"))?,
        parent_cpus: read_set(&parent.join("cpuset.cpus.effective"))?,
        parent_memory_nodes: read(&parent.join("cpuset.mems.effective"))?
            .trim()
            .to_owned(),
        controllers,
        rt_runtime_us: read(Path::new("/proc/sys/kernel/sched_rt_runtime_us"))?
            .trim()
            .to_owned(),
    })
}
/// Owns only the newly created leaf. No automatic cleanup or process termination.
/// The authorized parent and leaf must remain under the caller's exclusive control.
#[derive(Debug)]
pub struct Instance {
    path: PathBuf,
    device: u64,
    inode: u64,
}
impl Instance {
    pub fn prepare(parent: &Path, name: &str, goal: &Goal) -> Result<Self> {
        if name.is_empty()
            || name.len() > 128
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        {
            return Err(Error::Invalid(
                "instance name must be 1..128 ASCII letters/digits/_/-".into(),
            ));
        }
        let parent = fs::canonicalize(parent).map_err(|e| io(parent.display().to_string(), e))?;
        let caps = probe_parent(&parent)?;
        if !goal.cpus().is_subset_of(&caps.parent_cpus) || !goal.cpus().is_subset_of(&caps.online) {
            return Err(Error::Invalid(
                "cpulist exceeds parent's effective/online CPUs".into(),
            ));
        }
        // The kernel is authoritative for RT admission. CONFIG_RT_GROUP_SCHED with
        // the v2 CPU controller can reject RT tasks; never disable that controller.
        let path = parent.join(name);
        fs::create_dir(&path).map_err(|e| io(path.display().to_string(), e))?;
        let setup = (|| -> Result<Self> {
            let metadata =
                fs::symlink_metadata(&path).map_err(|e| io(path.display().to_string(), e))?;
            write(&path.join("cpuset.mems"), &caps.parent_memory_nodes)?;
            {
                let cpus = goal.cpus();
                write(&path.join("cpuset.cpus"), &cpus.canonical())?;
                if read_set(&path.join("cpuset.cpus.effective"))? != *cpus {
                    return Err(Error::Invalid(
                        "new cgroup effective CPUs differ from goal".into(),
                    ));
                }
            }
            Ok(Self {
                path: path.clone(),
                device: metadata.dev(),
                inode: metadata.ino(),
            })
        })();
        if let Err(error) = setup {
            if let Err(cleanup) = fs::remove_dir(&path) {
                return Err(Error::Invalid(format!(
                    "{error}; cleanup {} failed: {cleanup}",
                    path.display()
                )));
            }
            return Err(error);
        }
        setup
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn id(&self) -> u64 {
        self.inode
    }
    fn check_identity(&self) -> Result<()> {
        let meta =
            fs::symlink_metadata(&self.path).map_err(|e| io(self.path.display().to_string(), e))?;
        if meta.file_type().is_symlink() || (meta.dev(), meta.ino()) != (self.device, self.inode) {
            return Err(Error::Invalid("owned cgroup identity changed".into()));
        }
        Ok(())
    }
    /// Must be invoked before creating app threads. The task count checks catch
    /// existing threads; the caller must prevent concurrent thread creation.
    pub fn bootstrap_current_process(
        &self,
        goal: &Goal,
    ) -> std::result::Result<Applied, ApplyError> {
        let original = inspect_current_thread().map_err(|e| ApplyError::new(e, vec![]))?;
        let mut completed = vec![];
        let setup = (|| -> Result<()> {
            self.check_identity()?;
            if fs::read_dir("/proc/self/task")
                .map_err(|e| io("thread inventory", e))?
                .count()
                != 1
            {
                return Err(Error::Invalid(
                    "process bootstrap requires a single-threaded entrypoint".into(),
                ));
            }
            let parent = self.path.parent().expect("owned leaf parent");
            let caps = probe_parent(parent)?;
            if !goal.cpus().is_subset_of(&caps.parent_cpus)
                || !goal.cpus().is_subset_of(&caps.online)
            {
                return Err(Error::Invalid(
                    "goal CPUs exceed parent's effective/online CPUs".into(),
                ));
            }
            let configured = read(&self.path.join("cpuset.cpus"))?;
            if configured.trim().is_empty()
                || read_set(&self.path.join("cpuset.cpus"))? != *goal.cpus()
            {
                write(&self.path.join("cpuset.cpus"), &goal.cpus().canonical())?;
                completed.push("cpuset");
            }
            if read_set(&self.path.join("cpuset.cpus.effective"))? != *goal.cpus() {
                return Err(Error::Invalid(
                    "instance cpuset did not converge to goal".into(),
                ));
            }
            let pid = std::process::id().to_string();
            if !read(&self.path.join("cgroup.procs"))?
                .lines()
                .any(|line| line == pid)
            {
                write(&self.path.join("cgroup.procs"), &pid)?;
                completed.push("cgroup");
            }
            if !read(&self.path.join("cgroup.procs"))?
                .lines()
                .any(|line| line == pid)
            {
                return Err(Error::Invalid(
                    "process cgroup membership did not converge".into(),
                ));
            }
            Ok(())
        })();
        if let Err(error) = setup {
            return Err(ApplyError::new(error, completed));
        }
        match apply_current_thread(goal) {
            Ok(mut applied) => {
                applied.before = original;
                completed.extend(applied.changed);
                applied.changed = completed;
                Ok(applied)
            }
            Err(mut error) => {
                completed.extend(error.completed);
                error.completed = completed;
                Err(error)
            }
        }
    }
    /// Removes an empty owned leaf. Active processes/descendants make this fail.
    /// On failure the handle remains available for a later retry.
    pub fn remove(&self) -> Result<()> {
        self.check_identity()?;
        fs::remove_dir(&self.path).map_err(|e| io(self.path.display().to_string(), e))
    }
}
