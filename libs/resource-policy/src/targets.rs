//! Translation boundaries: these values describe enforcement requirements.
use crate::{Error, Goal, Result};

/// Resource directives suitable for a systemd [Service] section. Callers own
/// unit lifecycle, executable quoting, deployment, and host capability checks.
pub fn systemd_service_directives(goal: &Goal) -> String {
    let mut result = String::new();
    {
        let cpus = goal.cpus();
        let list = cpus.canonical().replace(',', " ");
        result.push_str(&format!("AllowedCPUs={list}\nCPUAffinity={list}\n"));
    }
    if goal.rt_priority() == 0 {
        result.push_str("CPUSchedulingPolicy=other\nCPUSchedulingPriority=0\n");
    } else {
        result.push_str(&format!(
            "CPUSchedulingPolicy=rr\nCPUSchedulingPriority={}\n",
            goal.rt_priority()
        ));
    }
    result
}
/// Standard Kubernetes CPU quantities cannot encode exact host CPU IDs or RR.
/// A future target adapter must negotiate explicit runtime/node capabilities.
pub fn validate_standard_kubernetes(_goal: &Goal) -> Result<()> {
    Err(Error::Invalid(
        "exact cpulist enforcement requires a Kubernetes node/runtime adapter".into(),
    ))
}
