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
        Err(e) if absent(&e) => return missing(path),
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
    let observed: std::result::Result<_, zbus::Error> = async {
        Ok((
            scope.slice().await?,
            unit.description().await?,
            scope.control_group().await?,
        ))
    }
    .await;
    let (observed_slice, observed_description, group) = match observed {
        Ok(observed) => observed,
        Err(e) if disappeared(&e) && gone(manager, name, path).await? => return Ok(()),
        Err(e) => return Err(error(e)),
    };
    if observed_slice != slice || observed_description != description {
        if gone(manager, name, path).await? {
            return Ok(());
        }
        return Err(format!("{name}: helper scope ownership differs"));
    }
    if !group.is_empty() && host_path(&group)? != path {
        if gone(manager, name, path).await? {
            return Ok(());
        }
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
        Err(e) if absent(&e) => return missing(path),
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
    // Failed scope starts remain loaded after StopUnit until their failure state is reset.
    if let Err(e) = manager.reset_failed_unit(name.into()).await
        && !absent(&e)
    {
        return Err(error(e));
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
        let empty = verify_empty(path);
        if !path.try_exists().map_err(error)? {
            return Ok(());
        }
        empty?;
        return Err(format!(
            "{}: helper scope cgroup remains after removal",
            path.display()
        ));
    }
    Ok(())
}

// A property can disappear while GetUnit's object path is being inspected.
fn disappeared(error: &zbus::Error) -> bool {
    matches!(error, zbus::Error::MethodError(name, _, _) if matches!(name.as_str(),
        NO_SUCH_UNIT | "org.freedesktop.DBus.Error.UnknownObject" | "org.freedesktop.DBus.Error.UnknownInterface"))
        || matches!(error, zbus::Error::FDO(cause) if matches!(cause.as_ref(),
            zbus::fdo::Error::UnknownObject(_) | zbus::fdo::Error::UnknownInterface(_)))
}
fn missing(path: &Path) -> Result<()> {
    if path.try_exists().map_err(error)? {
        return Err(format!(
            "{}: helper cgroup remains after unit disappearance",
            path.display()
        ));
    }
    Ok(())
}
async fn gone(manager: &ManagerProxy<'_>, name: &str, path: &Path) -> Result<bool> {
    match manager.get_unit(name.into()).await {
        Ok(_) => Ok(false),
        Err(e) if absent(&e) => {
            missing(path)?;
            Ok(true)
        }
        Err(e) => Err(error(e)),
    }
}

#[cfg(test)]
mod tests;
