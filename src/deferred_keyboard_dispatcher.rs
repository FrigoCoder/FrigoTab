use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use crate::keyboard_input::KeyboardInput;

/// Callback accepted by the UI message-queue adapter.
pub type PostedCallback = Box<dyn FnOnce() + Send + 'static>;
type Post = Box<dyn Fn(PostedCallback) -> bool + Send + Sync + 'static>;
type Handler = Arc<dyn Fn(KeyboardInput) + Send + Sync + 'static>;

/// Bounds and defers keyboard delivery.  The supplied post operation must
/// enqueue the callback rather than execute application work inline.
pub struct DeferredKeyboardDispatcher {
    post: Post,
    handler: Handler,
    capacity: usize,
    pending: Arc<AtomicUsize>,
    generation: Arc<AtomicU64>,
}

struct PendingRelease {
    pending: Arc<AtomicUsize>,
    released: AtomicBool,
}

impl PendingRelease {
    fn new(pending: Arc<AtomicUsize>) -> Self {
        Self {
            pending,
            released: AtomicBool::new(false),
        }
    }

    fn release(&self) {
        if !self.released.swap(true, Ordering::AcqRel) {
            self.pending.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

impl Drop for PendingRelease {
    fn drop(&mut self) {
        // A queued callback can be discarded when its owner window is torn
        // down.  Keep the accounting balanced even when the callback body is
        // never invoked; `release` is idempotent for the normal invoke path.
        self.release();
    }
}

impl DeferredKeyboardDispatcher {
    pub fn new<P, H>(post: P, handler: H, capacity: usize) -> Result<Self, &'static str>
    where
        P: Fn(PostedCallback) -> bool + Send + Sync + 'static,
        H: Fn(KeyboardInput) + Send + Sync + 'static,
    {
        if capacity == 0 {
            return Err("keyboard dispatcher capacity must be positive");
        }
        Ok(Self {
            post: Box::new(post),
            handler: Arc::new(handler),
            capacity,
            pending: Arc::new(AtomicUsize::new(0)),
            generation: Arc::new(AtomicU64::new(0)),
        })
    }

    pub fn pending_count(&self) -> usize {
        self.pending.load(Ordering::Acquire)
    }

    /// Prevents callbacks posted before a session/desktop interruption from
    /// reaching the handler after input state has been reset. Already-posted
    /// callbacks still release their bounded queue slots.
    pub fn invalidate_pending(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
    }

    pub fn try_dispatch(&self, input: KeyboardInput) -> bool {
        self.try_dispatch_with_limit(input, self.capacity)
    }

    /// Uses one reserved slot beyond the ordinary queue capacity. Session
    /// termination must survive a burst of repeat events while the UI thread
    /// is constructing or rendering the overlay.
    pub fn try_dispatch_critical(&self, input: KeyboardInput) -> bool {
        self.try_dispatch_with_limit(input, self.capacity.saturating_add(1))
    }

    fn try_dispatch_with_limit(&self, input: KeyboardInput, limit: usize) -> bool {
        let pending = self.pending.fetch_add(1, Ordering::AcqRel) + 1;
        if pending > limit {
            self.pending.fetch_sub(1, Ordering::AcqRel);
            return false;
        }

        let dispatch_generation = self.generation.load(Ordering::Acquire);
        let release = Arc::new(PendingRelease::new(Arc::clone(&self.pending)));
        let handler = Arc::clone(&self.handler);
        let generation = Arc::clone(&self.generation);
        let callback_release = Arc::clone(&release);
        let callback: PostedCallback = Box::new(move || {
            callback_release.release();
            if dispatch_generation != generation.load(Ordering::Acquire) {
                return;
            }
            handler(input);
        });

        // Posting can fail while the UI handle is being destroyed.  Keep the
        // queue accounting balanced even if an adapter panics while posting.
        let posted = catch_unwind(AssertUnwindSafe(|| (self.post)(callback))).unwrap_or(false);
        if !posted {
            release.release();
        }
        posted
    }
}
