//! Termination of a caller-owned foreground helper scope, independent of Docker containers.
use super::{Result, error, host_path, verify_empty};
use futures_util::StreamExt;
use std::path::Path;
use zbus::proxy::CacheProperties;
use zbus_systemd::systemd1::{ManagerProxy, ScopeProxy, UnitProxy};

const NO_SUCH_UNIT: &str = "org.freedesktop.systemd1.NoSuchUnit";
fn absent(error: &zbus::Error) -> bool {
    matches!(error, zbus::Error::MethodError(name, _, _) if name.as_str() == NO_SUCH_UNIT)
}

/// Kill and unload exactly the scope matching the persisted slice, path and description.
/// A subscribed manager and an outer caller deadline are required; absence is accepted.
/// Ownership mismatches/read failures preserve the unit. Cancellation requires retry with the same identity.
pub async fn stop(
    manager: &ManagerProxy<'_>,
    name: &str,
    slice: &str,
    path: &Path,
    description: &str,
) -> Result<()> {
    if !name.ends_with(".scope") || !slice.ends_with(".slice") {
        return Err("helper scope and slice suffixes are required".into());
    }
    let mut removed = manager.receive_unit_removed().await.map_err(error)?;
    let mut jobs = manager.receive_job_removed().await.map_err(error)?;
    let object = match manager.get_unit(name.into()).await {
        Ok(object) => object,
        Err(e) if absent(&e) => return Ok(()),
        Err(e) => return Err(error(e)),
    };
    let scope = ScopeProxy::builder(manager.inner().connection())
        .path(object.clone())
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .map_err(error)?;
    let unit = UnitProxy::builder(manager.inner().connection())
        .path(object)
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .map_err(error)?;
    if scope.slice().await.map_err(error)? != slice
        || unit.description().await.map_err(error)? != description
    {
        return Err(format!("{name}: helper scope ownership differs"));
    }
    let group = scope.control_group().await.map_err(error)?;
    if !group.is_empty() && host_path(&group)? != path {
        return Err(format!("{name}: helper scope cgroup path differs"));
    }
    // Kill all descendants, including helpers that created another process group/session.
    if let Err(e) = manager
        .kill_unit(name.into(), "all".into(), libc::SIGKILL)
        .await
        && !absent(&e)
    {
        return Err(error(e));
    }
    let job = match manager.stop_unit(name.into(), "replace".into()).await {
        Ok(job) => job,
        Err(e) if absent(&e) => return Ok(()),
        Err(e) => return Err(error(e)),
    };
    loop {
        let signal = jobs
            .next()
            .await
            .ok_or_else(|| format!("{name}: job notifications ended"))?;
        let args = signal.args().map_err(error)?;
        if *args.job == *job {
            if args.result != "done" {
                return Err(format!("{name}: helper scope stop failed: {}", args.result));
            }
            break;
        }
    }
    while !manager
        .list_units_by_patterns(vec![], vec![name.into()])
        .await
        .map_err(error)?
        .is_empty()
    {
        removed
            .next()
            .await
            .ok_or_else(|| format!("{name}: unit removal notifications ended"))?;
    }
    if path.try_exists().map_err(error)? {
        verify_empty(path)?;
        return Err(format!(
            "{}: helper scope cgroup remains after removal",
            path.display()
        ));
    }
    Ok(())
}
