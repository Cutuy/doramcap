//! Registry-backed app coordination; experiment admission and reset belong to the runner.
use crate::{
    lifecycle::{AppLifecycle, Attempt, Launch, Status},
    registry::{Registration, Registry},
};
use std::{
    collections::BTreeMap,
    sync::Arc,
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
    Busy { owner: String },
}

/// The manager exposes server-owned registrations and per-app coordination.
/// The recorded runner resets prior resources before starting a generation.
#[derive(Clone)]
pub struct Manager {
    apps: BTreeMap<String, AppEntry>,
    admission: Arc<tokio::sync::Mutex<()>>,
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
        Self {
            apps,
            admission: Arc::new(tokio::sync::Mutex::new(())),
        }
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
            let admission = self.admission.lock().await;
            for other in self.iter().filter(|other| other.registration.id != id) {
                if !matches!(other.lifecycle.status().await, Status::Stopped { .. }) {
                    return Err(Error::Busy {
                        owner: other.registration.id.clone(),
                    });
                }
            }
            match app.lifecycle.try_launch(accepted_at).await {
                Attempt::Accepted(launch) => return Ok(launch),
                Attempt::Stopping => {
                    drop(admission);
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
    async fn conflicting_concurrent_launches_are_serialized_until_cleanup() {
        // Arrange
        let registry = Registry::load(&workspace(), &["storage-demo".into()]).unwrap();
        let manager = Manager::new(registry, Duration::from_secs(120));

        // Act
        let (activity, storage) = tokio::join!(
            manager.launch("activity-demo", Instant::now()),
            manager.launch("storage-demo", Instant::now())
        );

        // Assert
        let (owner, other, accepted) = match (activity, storage) {
            (Ok(launch), Err(Error::Busy { owner })) => {
                assert_eq!(owner, "activity-demo");
                ("activity-demo", "storage-demo", launch)
            }
            (Err(Error::Busy { owner }), Ok(launch)) => {
                assert_eq!(owner, "storage-demo");
                ("storage-demo", "activity-demo", launch)
            }
            result => panic!("unexpected admission results: {result:?}"),
        };

        // Act
        let lifecycle = &manager.get(owner).unwrap().lifecycle;
        lifecycle.fail(accepted.generation).await;
        let during_cleanup = manager.launch(other, Instant::now()).await;
        lifecycle.completed(accepted.generation, None).await;
        let after_cleanup = manager.launch(other, Instant::now()).await;

        // Assert
        assert!(matches!(during_cleanup, Err(Error::Busy { .. })));
        assert!(after_cleanup.unwrap().start);
    }

    #[tokio::test]
    async fn waiting_for_cleanup_keeps_other_experiments_busy() {
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
        .await;
        lifecycle.completed(first.generation, None).await;
        let resumed = waiter.await.unwrap();

        // Assert
        assert!(matches!(unrelated.unwrap(), Err(Error::Busy { .. })));
        assert!(resumed.unwrap().start);
    }
}
