//! Bounded wake hints. Capture a generation before draining; wait only after checking work.
use crossbeam_channel::{Receiver, Sender, bounded};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

struct Inner {
    generation: AtomicU64,
    sender: Sender<()>,
    receiver: Receiver<()>,
}
/// Cloneable signal handle with one logical waiting consumer. No record payloads.
/// A stale token may wake early; callers always recheck their queues and deadlines.
#[derive(Clone)]
pub struct Notification(Arc<Inner>);
impl Default for Notification {
    fn default() -> Self {
        let (sender, receiver) = bounded(1);
        Self(Arc::new(Inner {
            generation: AtomicU64::new(0),
            sender,
            receiver,
        }))
    }
}
impl Notification {
    /// Read the notification generation before inspecting associated work.
    ///
    /// The counter wraps at `u64::MAX`; callers must not reuse a generation across
    /// an entire counter cycle.
    pub fn generation(&self) -> u64 {
        self.0.generation.load(Ordering::Acquire)
    }
    /// Advance the generation and enqueue a bounded wake hint without waiting.
    ///
    /// Concurrent notifications can share one pending token. Records and deadlines
    /// remain in their owning component; this handle carries no payload.
    pub fn notify(&self) {
        self.0.generation.fetch_add(1, Ordering::Release);
        let _ = self.0.sender.try_send(());
    }
    /// Block for a generation change, wake hint, or `timeout`.
    ///
    /// Capture `seen` before draining/checking work. If the generation already
    /// changed, returns immediately. A stale token may also return early; always
    /// recheck work afterward. Uses one logical waiting consumer across all clones.
    ///
    /// No records are consumed and no thread is created. Release host locks needed
    /// by producers before calling. OS scheduling may delay return beyond the timeout.
    pub fn wait_since(&self, seen: u64, timeout: Duration) {
        if self.generation() == seen {
            let _ = self.0.receiver.recv_timeout(timeout);
        }
    }
}
