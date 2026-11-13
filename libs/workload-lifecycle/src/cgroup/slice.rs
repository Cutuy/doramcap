//! Stop empty systemd slices through the host service manager and confirm their removal.
use super::{Result, error, host_path, verify_empty};
use futures_util::StreamExt;
use zbus::proxy::CacheProperties;
use zbus_systemd::systemd1::{ManagerProxy, SliceProxy};

const NO_SUCH_UNIT: &str = "org.freedesktop.systemd1.NoSuchUnit";

/// Connect to the local system bus and subscribe to unit signals.
pub async fn connect() -> Result<ManagerProxy<'static>> {
    let connection = zbus::Connection::system().await.map_err(error)?;
    let manager = ManagerProxy::new(&connection).await.map_err(error)?;
    manager.subscribe().await.map_err(error)?;
    Ok(manager)
}

/// Stop the loaded slice `name` once its whole cgroup tree has no processes, then wait
/// until the unit unloads and its tree is gone. An unloaded slice is accepted unchanged.
/// - `manager` must be subscribed; callers bound the wait and release their own unit references.
pub async fn stop(manager: &ManagerProxy<'_>, name: &str) -> Result<()> {
    // Subscribe before reading state so a completed stop or unload cannot be missed.
    let mut removed = manager.receive_unit_removed().await.map_err(error)?;
    let mut jobs = manager.receive_job_removed().await.map_err(error)?;
    let object = match manager.get_unit(name.into()).await {
        Ok(object) => object,
        Err(zbus::Error::MethodError(kind, _, _)) if kind.as_str() == NO_SUCH_UNIT => {
            return Ok(());
        }
        Err(err) => return Err(error(err)),
    };
    let group = SliceProxy::builder(manager.inner().connection())
        .path(object)
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .map_err(error)?
        .control_group()
        .await
        .map_err(error)?;
    let path = (!group.is_empty()).then(|| host_path(&group)).transpose()?;
    // Stopping a slice stops every unit below it; refuse while any descendant runs.
    if let Some(path) = &path
        && path.try_exists().map_err(error)?
    {
        verify_empty(path)?;
    }
    let job = manager
        .stop_unit(name.into(), "replace".into())
        .await
        .map_err(error)?;
    loop {
        let signal = jobs
            .next()
            .await
            .ok_or_else(|| format!("{name}: job notifications ended"))?;
        let args = signal.args().map_err(error)?;
        if *args.job == *job {
            if args.result != "done" {
                return Err(format!("{name}: slice stop failed: {}", args.result));
            }
            break;
        }
    }
    // A stopped slice must also unload, or its name keeps the cgroup reserved.
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
    if let Some(path) = &path
        && path.try_exists().map_err(error)?
    {
        return Err(format!("{}: cgroup remains after removal", path.display()));
    }
    Ok(())
}
