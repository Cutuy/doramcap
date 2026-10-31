//! Registry-backed app coordination; experiment admission and reset belong to the runner.
use crate::{
    lifecycle::{AppLifecycle, Attempt, Launch, Status},
    registry::{Registration, Registry},
};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

/// A registered application and its independent lifecycle coordinator.
#[derive(Clone)]
pub struct AppEntry {
    pub registration: Registration,
    pub lifecycle: AppLifecycle,
}

/// Errors accepting a request from the gallery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    UnknownApp,
    ShuttingDown,
    Blocked(String),
}

/// The manager exposes server-owned registrations and per-app coordination.
/// The recorded runner resets prior resources before starting a generation.
#[derive(Clone)]
pub struct Manager {
    apps: BTreeMap<String, AppEntry>,
}
impl Manager {
    /// Construct app coordinators; the recorded workflow resets prior runs before launch.
    pub fn new(registry: Registry, idle_timeout: Duration) -> Self {
        let apps = registry
            .iter()
            .map(|app| {
                (
                    app.id.clone(),
                    AppEntry {
                        registration: app.clone(),
                        lifecycle: AppLifecycle::new(idle_timeout),
                    },
                )
            })
            .collect();
        Self { apps }
    }
    /// Get a registration and coordinator by public ID.
    pub fn get(&self, id: &str) -> Option<&AppEntry> {
        self.apps.get(id)
    }
    /// Iterate registered entries in stable ID order.
    pub fn iter(&self) -> impl Iterator<Item = &AppEntry> {
        self.apps.values()
    }
    /// Accept a launch for a known app, or return a visible blocked/unknown error.
    /// Acceptance returns without yielding again; cancellation before return
    /// creates no session. The caller must queue startup before its next await.
    pub async fn launch(&self, id: &str, now: Instant) -> Result<Launch, Error> {
        let app = self.get(id).ok_or(Error::UnknownApp)?;
        let mut accepted_at = now;
        loop {
            match app.lifecycle.try_launch(accepted_at).await {
                Attempt::Accepted(launch) => return Ok(launch),
                Attempt::Stopping => {
                    app.lifecycle.wait_until_not_stopping().await;
                    accepted_at = Instant::now();
                }
            }
        }
    }
    /// Return the current status of a known app.
    pub async fn status(&self, id: &str) -> Result<Status, Error> {
        Ok(self
            .get(id)
            .ok_or(Error::UnknownApp)?
            .lifecycle
            .status()
            .await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn workspace() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .unwrap()
            .to_owned()
    }

    #[tokio::test]
    async fn unfinished_cleanup_allows_the_next_workflow_to_reset() {
        // Arrange
        let manager = Manager::new(
            Registry::new(&workspace()).unwrap(),
            Duration::from_secs(120),
        );
        let first = manager
            .launch("activity-demo", Instant::now())
            .await
            .unwrap();
        let app = manager.get("activity-demo").unwrap();
        app.lifecycle.fail(first.generation).await;
        // Act
        app.lifecycle
            .completed(first.generation, Some("cleanup unverified".into()))
            .await;
        // Assert: retain the error while allowing a fresh reset attempt.
        assert_eq!(
            app.lifecycle.status().await,
            Status::Stopped {
                error: Some("cleanup unverified".into())
            }
        );
        let next = manager
            .launch("activity-demo", Instant::now())
            .await
            .unwrap();
        assert!(next.start);
        assert_ne!(first.generation, next.generation);
    }

    #[tokio::test]
    async fn different_apps_launch_concurrently_and_viewers_join_their_app() {
        // Arrange
        let registry = Registry::load(&workspace(), &["storage-demo".into()]).unwrap();
        let manager = Manager::new(registry, Duration::from_secs(120));

        // Act
        let (activity, storage) = tokio::join!(
            manager.launch("activity-demo", Instant::now()),
            manager.launch("storage-demo", Instant::now())
        );

        // Assert
        let activity = activity.unwrap();
        let storage = storage.unwrap();
        assert!(activity.start && storage.start);
        assert_ne!(activity.generation, storage.generation);

        // Act / Assert: each app's additional viewer shares only its own generation.
        for (id, first) in [("activity-demo", activity), ("storage-demo", storage)] {
            let joined = manager.launch(id, Instant::now()).await.unwrap();
            assert!(!joined.start);
            assert_eq!(joined.generation, first.generation);
        }
    }

    #[tokio::test]
    async fn waiting_for_one_apps_cleanup_does_not_block_another_app() {
        // Arrange
        let registry = Registry::load(&workspace(), &["storage-demo".into()]).unwrap();
        let manager = Manager::new(registry, Duration::from_secs(120));
        let first = manager
            .launch("activity-demo", Instant::now())
            .await
            .unwrap();
        let lifecycle = &manager.get("activity-demo").unwrap().lifecycle;
        lifecycle.fail(first.generation).await;
        let waiting_manager = manager.clone();
        let waiter = tokio::spawn(async move {
            waiting_manager
                .launch("activity-demo", Instant::now())
                .await
        });
        tokio::task::yield_now().await;

        // Act
        let unrelated = tokio::time::timeout(
            Duration::from_secs(1),
            manager.launch("storage-demo", Instant::now()),
        )
        .await
        .unwrap()
        .unwrap();

        // Assert: a separate app starts while the same-app replacement waits.
        assert!(unrelated.start);
        assert!(!waiter.is_finished());

        // Act
        lifecycle.completed(first.generation, None).await;
        let resumed = waiter.await.unwrap().unwrap();

        // Assert
        assert!(resumed.start);
        assert_ne!(resumed.generation, first.generation);
    }
}
