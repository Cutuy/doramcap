//! Shared filesystem observations and recording/process readiness notifications.
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{io, path::Path};
use tokio::sync::watch;

/// Subscribe before inspecting paths. Keep the watcher alive while waiting.
/// Notifications coalesce; the first watcher error remains available to every wait.
/// Read-only access does not wake waiters.
pub fn watch_changes(
    paths: impl IntoIterator<Item = impl AsRef<Path>>,
) -> io::Result<(RecommendedWatcher, watch::Receiver<notify::Result<()>>)> {
    let (changed, receiver) = watch::channel(Ok(()));
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.as_ref().is_ok_and(|event| event.kind.is_access()) {
            return;
        }
        changed.send_modify(|status| {
            if status.is_ok() {
                *status = event.map(|_| ());
            }
        });
    })
    .map_err(io::Error::other)?;
    for path in paths {
        watcher
            .watch(path.as_ref(), RecursiveMode::NonRecursive)
            .map_err(io::Error::other)?;
    }
    Ok((watcher, receiver))
}

/// Await a change; return watcher failures and unexpected channel closure.
pub async fn wait_for_change(changes: &mut watch::Receiver<notify::Result<()>>) -> io::Result<()> {
    changes.changed().await.map_err(io::Error::other)?;
    changes
        .borrow_and_update()
        .as_ref()
        .map(|_| ())
        .map_err(|error| io::Error::other(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    const EVENT_DEADLINE: Duration = Duration::from_secs(2);
    const READ_QUIET_INTERVAL: Duration = Duration::from_millis(50);

    #[tokio::test]
    async fn creation_wakes_waiter_and_inspection_does_not() {
        // Arrange
        let root = std::env::temp_dir().join(format!(
            "file-changes-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let file = root.join("recording");
        let (_directory_watcher, mut created) = watch_changes([&root]).unwrap();

        // Act
        std::fs::write(&file, b"record").unwrap();
        tokio::time::timeout(EVENT_DEADLINE, wait_for_change(&mut created))
            .await
            .unwrap()
            .unwrap();

        // Assert
        assert!(file.exists());

        // Act: inspections must not feed another readiness wake.
        let (_file_watcher, mut changes) = watch_changes([&file]).unwrap();
        assert_eq!(std::fs::read(&file).unwrap(), b"record");

        // Assert
        assert!(
            tokio::time::timeout(READ_QUIET_INTERVAL, wait_for_change(&mut changes))
                .await
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
