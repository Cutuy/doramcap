//! Stop only Docker's verified-empty per-run slices through the local service manager.
use super::{cgroup, verify_empty_if_present};
use futures_util::StreamExt;
use std::path::PathBuf;
use zbus::proxy::CacheProperties;
use zbus_systemd::systemd1::{ManagerProxy, SliceProxy};

pub(super) async fn remove(paths: &[PathBuf]) -> Result<(), String> {
    let connection = zbus::Connection::system().await.map_err(error)?;
    let manager = ManagerProxy::new(&connection).await.map_err(error)?;
    manager.subscribe().await.map_err(error)?;
    for path in paths {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Compose slice name is not UTF-8")?;
        // Subscribe before inspecting state so an already-completed stop cannot lose its signal.
        let mut removed = manager.receive_unit_removed().await.map_err(error)?;
        let object = match manager.get_unit(name.into()).await {
            Ok(object) => object,
            Err(zbus::Error::MethodError(name, _, _))
                if name.as_str() == "org.freedesktop.systemd1.NoSuchUnit" =>
            {
                if path.try_exists().map_err(error)? {
                    return Err(format!(
                        "{}: cgroup exists without its systemd unit",
                        path.display()
                    ));
                }
                continue;
            }
            Err(err) => return Err(error(err)),
        };
        let slice = SliceProxy::builder(&connection)
            .path(object)
            .cache_properties(CacheProperties::No)
            .build()
            .await
            .map_err(error)?;
        let group = slice.control_group().await.map_err(error)?;
        if !group.is_empty() && cgroup::host_path(&group)? != *path {
            return Err(format!("{name}: systemd accounting path changed"));
        }
        let identity = path
            .try_exists()
            .map_err(error)?
            .then(|| cgroup::inspect(path))
            .transpose()?;
        let mut jobs = manager.receive_job_removed().await.map_err(error)?;
        if let Some(identity) = &identity {
            cgroup::verify_identity(identity)?;
        }
        verify_empty_if_present(path)?;
        let job = manager
            .stop_unit(name.into(), "replace".into())
            .await
            .map_err(error)?;
        loop {
            let signal = jobs
                .next()
                .await
                .ok_or("Compose slice stop notifications ended")?;
            let args = signal.args().map_err(error)?;
            if *args.job == *job {
                if args.result != "done" {
                    return Err(format!("{name}: slice stop failed: {}", args.result));
                }
                break;
            }
        }
        // Stopped units must unload as well as release their cgroups.
        while !manager
            .list_units_by_names(vec![name.into()])
            .await
            .map_err(error)?
            .is_empty()
        {
            let signal = removed
                .next()
                .await
                .ok_or("Compose slice removal notifications ended")?;
            if signal.args().map_err(error)?.id != name {
                continue;
            }
        }
    }
    Ok(())
}

fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
