//! Tracks filesystem work that must finish before the process exits.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};

#[derive(Clone, Default)]
struct Tracker(Arc<AtomicUsize>);

impl Tracker {
    fn acquire(&self) -> Arc<Lease> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Arc::new(Lease(self.0.clone()))
    }

    fn in_flight(&self) -> bool {
        self.0.load(Ordering::SeqCst) != 0
    }
}

pub(crate) struct Lease(Arc<AtomicUsize>);

impl Drop for Lease {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

static TRACKER: LazyLock<Tracker> = LazyLock::new(Tracker::default);

/// Acquire before spawning or deferring work, so shutdown sees it immediately.
pub(crate) fn acquire() -> Arc<Lease> {
    TRACKER.acquire()
}

pub(crate) fn in_flight() -> bool {
    TRACKER.in_flight()
}

/// Remove an exclusively owned temporary tree without blocking the UI.
pub(crate) fn cleanup(path: PathBuf) {
    if path.as_os_str().is_empty() {
        return;
    }
    let lease = acquire();
    let worker_lease = lease.clone();
    let worker_path = path.clone();
    if std::thread::Builder::new()
        .name("favnyr-cleanup".into())
        .spawn(move || {
            let _lease = worker_lease;
            remove_tree(&worker_path);
        })
        .is_err()
    {
        // The caller still owns a lease if the worker could not be started.
        remove_tree(&path);
    }
}

fn remove_tree(path: &std::path::Path) {
    if let Err(error) = std::fs::remove_dir_all(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(%error, path = %path.display(), "temporary directory cleanup failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_stays_pending_until_its_last_owner_finishes() {
        let tracker = Tracker::default();
        let first = tracker.acquire();
        let worker = first.clone();
        let second = tracker.acquire();
        drop(first);
        drop(second);
        assert!(tracker.in_flight());
        std::thread::spawn(move || drop(worker)).join().unwrap();
        assert!(!tracker.in_flight());
    }
}
