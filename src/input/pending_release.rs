use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub(crate) struct PendingRelease {
    pending: Arc<AtomicUsize>,
    released: AtomicBool,
}

impl PendingRelease {
    pub(crate) fn new(pending: Arc<AtomicUsize>) -> Self {
        Self {
            pending,
            released: AtomicBool::new(false),
        }
    }

    pub(crate) fn release(&self) {
        if !self.released.swap(true, Ordering::AcqRel) {
            self.pending.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

impl Drop for PendingRelease {
    fn drop(&mut self) {
        // A queued callback can be discarded when its owner window is torn
        // down. Keep the accounting balanced even when the callback body is
        // never invoked; release is idempotent for the normal invoke path.
        self.release();
    }
}
