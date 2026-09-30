//! The Win32 keyboard-hook adapter.
//!
//! This is the platform half of the original `KeyHook` boundary. The low-level
//! callback only normalizes an event, makes the bounded suppression decision,
//! and posts a small queue-slot number to the UI window.  Window enumeration,
//! session construction, and the user-supplied handler run later on the UI
//! thread.  The hook thread owns the Win32 hook and has its own message loop;
//! it is never used as a worker for application work.
//!
//! Integration contract: the UI window procedure must call
//! [`KeyHook::dispatch_ui_message`] for `WM_KEY_HOOK_INPUT`, and must call
//! [`KeyHook::drain_ui_messages`] before destroying that HWND.  The session
//! layer must call `set_session_visible(false)` for an ordinary close (to
//! invalidate stale UI callbacks while retaining matching key-up suppression)
//! and `reset_input_state()` only for an interruption or shutdown.

use std::mem::size_of;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, SendInput,
    VK_1, VK_9, VK_CONTROL, VK_ESCAPE, VK_F4, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_MENU,
    VK_NUMPAD1, VK_NUMPAD9, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_SHIFT, VK_TAB,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, LLKHF_EXTENDED,
    LLKHF_INJECTED, LLKHF_LOWER_IL_INJECTED, MSG, PM_NOREMOVE, PeekMessageW, PostMessageW,
    PostThreadMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL,
    WM_APP, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

pub use super::key_hook_error::KeyHookError;

use super::callback_queue::{CallbackQueue, UiCallback};
use super::completion_signal::CompletionSignal;
use super::delivered_inputs::DeliveredInputs;
use super::hook_shared::HookShared;
use super::input_state::InputState;
use crate::input::{
    AltTabRecoveryPlan, DeferredKeyboardDispatcher, KeyHandling, KeyTransition, KeyboardInput,
    KeyboardModifierKey, KeyboardModifierState, KeyboardSuppressionState, SwitcherKey,
};
use crate::window::{WindowHandle, window_handle::FOREGROUND_NUDGE_EXTRA_INFO};

/// Message posted to the UI window for an admitted keyboard callback.
///
/// `wParam` is a one-based slot number owned by this [`KeyHook`].  A slot is
/// taken exactly once by [`KeyHook::dispatch_ui_message`], and slots that are
/// still queued are dropped by `drain_ui_messages` during shutdown.
pub const WM_KEY_HOOK_INPUT: u32 = WM_APP + 2;

/// Explicit command-line opt-in used by black-box acceptance tests.
/// Normal application launches leave arbitrary injected key events untouched.
pub const ACCEPT_MARKED_TEST_INPUT_ARGUMENT: &str = "--accept-marked-test-input";

/// `dwExtraInfo` tag carried only by the acceptance harness's `SendInput`
/// events. This is provenance filtering, not a security boundary.
pub const MARKED_TEST_INPUT_EXTRA_INFO: usize = 0x4652_4947;

const DISPATCH_CAPACITY: usize = 64;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

// WH_KEYBOARD_LL callbacks do not receive a user-data pointer.  The process
// has one production hook, so an atomic pointer is the smallest bridge to the
// Arc-owned state.  It remains valid until the hook thread has unhooked and
// been joined.
static ACTIVE_HOOK: AtomicPtr<HookShared> = AtomicPtr::new(null_mut());

/// A running global low-level keyboard hook.
pub struct KeyHook {
    shared: Arc<HookShared>,
    thread: Option<JoinHandle<()>>,
    completion: Option<mpsc::Receiver<()>>,
}

impl KeyHook {
    /// Start the dedicated hook thread and wait for the `WH_KEYBOARD_LL` hook
    /// to be installed.  The supplied owner must be the UI-thread HWND which
    /// handles [`WM_KEY_HOOK_INPUT`].
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is opaque, never dereferenced as Rust memory.
    pub fn start(owner: HWND) -> Result<Self, KeyHookError> {
        Self::start_inner(owner, false)
    }

    /// Start the hook with narrowly marked injected events enabled.
    ///
    /// This exists for black-box acceptance tests. FrigoTab's own `SendInput`
    /// recovery remains unmarked and therefore cannot recurse through the
    /// hook. Lower-integrity injected input is always rejected.
    #[allow(clippy::not_unsafe_ptr_arg_deref)] // HWND is opaque, never dereferenced as Rust memory.
    pub fn start_accepting_marked_test_input(owner: HWND) -> Result<Self, KeyHookError> {
        Self::start_inner(owner, true)
    }

    fn start_inner(owner: HWND, accept_marked_test_input: bool) -> Result<Self, KeyHookError> {
        if !WindowHandle::new(owner).is_valid() {
            return Err(KeyHookError::InvalidOwner);
        }

        let callbacks = Arc::new(CallbackQueue::new(DISPATCH_CAPACITY));
        let delivered = Arc::new(DeliveredInputs::new(DISPATCH_CAPACITY));
        let post_callbacks = Arc::clone(&callbacks);
        let delivered_by_handler = Arc::clone(&delivered);
        let owner_value = owner as usize;

        let dispatcher = Arc::new(
            DeferredKeyboardDispatcher::new(
                move |callback: UiCallback| post_callback(&post_callbacks, owner_value, callback),
                move |input| delivered_by_handler.push(input, DISPATCH_CAPACITY),
                DISPATCH_CAPACITY,
            )
            .map_err(|_| KeyHookError::ThreadStart)?,
        );

        let shared = Arc::new(HookShared {
            input: Mutex::new(InputState {
                modifiers: KeyboardModifierState::new(),
                suppression: KeyboardSuppressionState::new(),
                deferred_alt_vk: 0,
                deferred_alt_scan: 0,
                deferred_alt_extended: false,
                deferred_shift_vk: 0,
                deferred_shift_scan: 0,
                deferred_shift_extended: false,
                alt_forwarded: false,
                blocked_native_releases: [0; 4],
            }),
            dispatcher,
            callbacks,
            delivered,
            disposed: AtomicBool::new(false),
            accept_marked_test_input,
            thread_id: AtomicU32::new(0),
            hook_id: AtomicPtr::new(null_mut()),
        });

        let thread_shared = Arc::clone(&shared);
        let (startup_sender, startup_receiver) = mpsc::sync_channel(1);
        let (completion_sender, completion_receiver) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("FrigoTab keyboard hook".to_owned())
            .spawn(move || hook_thread(thread_shared, startup_sender, completion_sender))
            .map_err(|_| KeyHookError::ThreadStart)?;

        match startup_receiver.recv_timeout(STARTUP_TIMEOUT) {
            Ok(Ok(())) => Ok(Self {
                shared,
                thread: Some(thread),
                completion: Some(completion_receiver),
            }),
            Ok(Err(error)) => {
                shared.disposed.store(true, Ordering::Release);
                shutdown_thread(&shared, thread, completion_receiver);
                Err(error)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                shared.disposed.store(true, Ordering::Release);
                shutdown_thread(&shared, thread, completion_receiver);
                Err(KeyHookError::StartupTimeout)
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                shared.disposed.store(true, Ordering::Release);
                shutdown_thread(&shared, thread, completion_receiver);
                Err(KeyHookError::ThreadExited)
            }
        }
    }

    /// Notify the suppression policy that the switcher is visible (or has
    /// closed).  Closing invalidates queued UI callbacks, but deliberately
    /// leaves the consumed-key ledger intact until the matching key-up.
    pub fn set_session_visible(&self, visible: bool) {
        if self.shared.disposed.load(Ordering::Acquire) {
            return;
        }
        let mut input = self
            .shared
            .input
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if self.shared.disposed.load(Ordering::Acquire) {
            return;
        }
        if !visible {
            self.shared.dispatcher.invalidate_pending();
        }
        input.suppression.set_session_visible(visible);
    }

    /// Invalidate queued callbacks and reset session/modifier state after a
    /// desktop interruption. Releases for already-suppressed downs remain in
    /// the ledger; only hook shutdown clears them completely.
    pub fn reset_input_state(&self) {
        let mut input = self
            .shared
            .input
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.shared.dispatcher.invalidate_pending();
        if input.deferred_alt_vk != 0 {
            input.suppression.consume_matching_release(SwitcherKey::Alt);
            clear_deferred_alt(&mut input);
        }
        if input.deferred_shift_vk != 0 {
            input
                .suppression
                .consume_matching_release(SwitcherKey::Shift);
            clear_deferred_shift(&mut input);
        }
        // `alt_forwarded` and the raw release ledger deliberately survive an
        // interruption: Windows has already seen those synthetic downs, so
        // their later physical ups must still pass or be swallowed as paired.
        input.modifiers.reset();
        input.suppression.reset_session_state();
    }

    /// Dispatch one `WM_KEY_HOOK_INPUT` slot on the UI thread.
    ///
    /// The supplied handler is called only for a current-generation event and
    /// returns the same `KeyHandling` decision as the original event subscriber. If
    /// the initial Alt+Tab was admitted but the handler did not consume it,
    /// the native gesture is recovered with `SendInput` exactly as before.
    /// Returning `false` means that `wParam` was not a live slot.
    pub fn dispatch_ui_message<F>(&self, wparam: WPARAM, handler: F) -> bool
    where
        F: FnOnce(KeyboardInput) -> KeyHandling,
    {
        let Some(callback) = self.shared.callbacks.take(wparam) else {
            return false;
        };
        callback();

        let Some(input) = self.shared.delivered.pop() else {
            // A generation-invalidated callback still owns and releases its
            // bounded dispatcher slot, but has no user event to deliver.
            return true;
        };

        let handling = handler(input);
        if handling == KeyHandling::PassThrough
            && input.is_down()
            && input.key == SwitcherKey::Tab
            && input.alt
        {
            let (alt_reached_target, shift_reached_target) = {
                let mut state = self
                    .shared
                    .input
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                self.shared.dispatcher.invalidate_pending();
                let alt_was_suppressed = state
                    .suppression
                    .consumes_matching_release(SwitcherKey::Alt);
                let shift_was_suppressed = state
                    .suppression
                    .consumes_matching_release(SwitcherKey::Shift);
                state.suppression.set_session_visible(false);
                (
                    state.modifiers.alt_down() && !alt_was_suppressed,
                    state.modifiers.shift_down() && !shift_was_suppressed,
                )
            };
            let _ = replay_native_alt_tab(alt_reached_target, input.shift, shift_reached_target);
        }
        true
    }

    /// Drop any callbacks which were admitted but whose owner HWND is being
    /// destroyed.  Call this from the UI teardown path before destroying the
    /// session HWND; `Drop` repeats it as a final safety net.
    pub fn drain_ui_messages(&self) {
        self.shared.callbacks.drain();
    }
}

impl Drop for KeyHook {
    fn drop(&mut self) {
        if self.shared.disposed.swap(true, Ordering::AcqRel) {
            return;
        }

        {
            let mut input = self
                .shared
                .input
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            self.shared.dispatcher.invalidate_pending();
            clear_deferred_alt(&mut input);
            clear_deferred_shift(&mut input);
            input.alt_forwarded = false;
            input.blocked_native_releases = [0; 4];
            input.modifiers.reset();
            input.suppression.reset();
        }
        self.shared.callbacks.drain();
        post_quit(&self.shared);

        if let Some(thread) = self.thread.take() {
            let completion = self
                .completion
                .take()
                .expect("hook thread completion channel missing");
            shutdown_thread(&self.shared, thread, completion);
        }
    }
}

fn post_callback(queue: &CallbackQueue, owner: usize, callback: UiCallback) -> bool {
    let Some(slot) = queue.insert(callback) else {
        return false;
    };
    let posted = unsafe { PostMessageW(owner as HWND, WM_KEY_HOOK_INPUT, slot, 0) != 0 };
    if !posted {
        // No UI callback can race a failed PostMessageW.  Taking the slot here
        // drops the exact Box once and leaves the dispatcher counter balanced.
        queue.take(slot);
    }
    posted
}

fn post_quit(shared: &HookShared) {
    let thread_id = shared.thread_id.load(Ordering::Acquire);
    if thread_id != 0 {
        unsafe {
            let _ = PostThreadMessageW(thread_id, WM_QUIT, 0, 0);
        }
    }
}

/// Stop the hook thread without making UI teardown unbounded.  The first
/// interval gives the message loop a chance to process WM_QUIT; the second
/// follows an explicit UnhookWindowsHookEx, matching the native lifetime
/// contract.  If a misbehaving Win32 call still prevents completion, dropping
/// the JoinHandle detaches the thread while its Arc-owned state remains valid.
fn shutdown_thread(shared: &HookShared, thread: JoinHandle<()>, completion: mpsc::Receiver<()>) {
    post_quit(shared);

    if thread.thread().id() == thread::current().id() {
        // Joining ourselves would deadlock.  The hook thread retains its Arc
        // and will finish cleanup independently.
        return;
    }

    match completion.recv_timeout(SHUTDOWN_TIMEOUT) {
        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
            let _ = thread.join();
            return;
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {}
    }

    uninstall_hook(shared);
    post_quit(shared);
    match completion.recv_timeout(SHUTDOWN_TIMEOUT) {
        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
            let _ = thread.join();
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            // Deliberately detach after the bounded grace periods.
            drop(thread);
        }
    }
}

fn hook_thread(
    shared: Arc<HookShared>,
    startup: mpsc::SyncSender<Result<(), KeyHookError>>,
    completion: mpsc::SyncSender<()>,
) {
    let _completion = CompletionSignal(Some(completion));
    let thread_id = unsafe { GetCurrentThreadId() };
    shared.thread_id.store(thread_id, Ordering::Release);

    unsafe {
        let mut message = MSG::default();
        let _ = PeekMessageW(&mut message, null_mut(), 0, 0, PM_NOREMOVE);
    }

    let pointer = Arc::as_ptr(&shared) as *mut HookShared;
    if ACTIVE_HOOK
        .compare_exchange(null_mut(), pointer, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        shared.thread_id.store(0, Ordering::Release);
        let _ = startup.send(Err(KeyHookError::AlreadyInstalled));
        return;
    }

    let module = unsafe { GetModuleHandleW(null()) };
    if module.is_null() {
        clear_active(pointer, &shared);
        let _ = startup.send(Err(win32_error("GetModuleHandleW")));
        return;
    }
    let hook =
        unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), module, 0) };
    if hook.is_null() {
        clear_active(pointer, &shared);
        let _ = startup.send(Err(win32_error("SetWindowsHookExW")));
        return;
    }
    shared.hook_id.store(hook, Ordering::Release);
    if shared.disposed.load(Ordering::Acquire) {
        uninstall_hook(&shared);
        clear_active(pointer, &shared);
        let _ = startup.send(Err(KeyHookError::ThreadExited));
        return;
    }
    let _ = startup.send(Ok(()));

    loop {
        if shared.disposed.load(Ordering::Acquire) {
            break;
        }
        let mut message = MSG::default();
        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if result == -1 {
            eprintln!("FrigoTab keyboard hook GetMessageW failed: {}", unsafe {
                GetLastError()
            });
            break;
        }
        if result == 0 {
            break;
        }
        unsafe {
            let _ = TranslateMessage(&message);
            let _ = DispatchMessageW(&message);
        }
    }

    uninstall_hook(&shared);
    clear_active(pointer, &shared);
}

fn clear_active(pointer: *mut HookShared, shared: &HookShared) {
    let _ = ACTIVE_HOOK.compare_exchange(pointer, null_mut(), Ordering::AcqRel, Ordering::Acquire);
    shared.thread_id.store(0, Ordering::Release);
}

fn uninstall_hook(shared: &HookShared) {
    let hook = shared.hook_id.swap(null_mut(), Ordering::AcqRel);
    if !hook.is_null() {
        unsafe {
            let _ = UnhookWindowsHookEx(hook);
        }
    }
}

unsafe extern "system" fn low_level_keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let consumed = catch_unwind(AssertUnwindSafe(|| {
        // Low-level hooks must forward negative nCode values unchanged.  For
        // every nonnegative code the hook examines in the event payload.
        if code < 0 {
            return false;
        }
        let pointer = ACTIVE_HOOK.load(Ordering::Acquire);
        if pointer.is_null() {
            return false;
        }
        let shared = unsafe { &*pointer };
        if shared.disposed.load(Ordering::Acquire) || lparam == 0 {
            return false;
        }
        hook_proc_inner(shared, code, wparam, lparam)
    }))
    .unwrap_or(false);

    if consumed {
        return 1;
    }
    catch_unwind(AssertUnwindSafe(|| unsafe {
        let active = ACTIVE_HOOK.load(Ordering::Acquire);
        let hook = if active.is_null() {
            null_mut()
        } else {
            (&*active).hook_id.load(Ordering::Acquire)
        };
        CallNextHookEx(hook, code, wparam, lparam)
    }))
    .unwrap_or(0)
}

fn hook_proc_inner(shared: &HookShared, _code: i32, wparam: WPARAM, lparam: LPARAM) -> bool {
    let transition = match wparam as u32 {
        WM_KEYDOWN | WM_SYSKEYDOWN => KeyTransition::Down,
        WM_KEYUP | WM_SYSKEYUP => KeyTransition::Up,
        _ => return false,
    };
    let event = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
    let injected = event.flags & LLKHF_INJECTED != 0;
    // Lower-integrity input is never admitted, including a forged copy of
    // FrigoTab's private foreground nudge marker. Keep this check ahead of
    // the marker fast path so the marker cannot weaken the UIPI boundary.
    if event.flags & LLKHF_LOWER_IL_INJECTED != 0 {
        return false;
    }
    if injected
        && transition == KeyTransition::Down
        && event.vkCode == 0
        && event.scanCode == 0
        && event.dwExtraInfo == FOREGROUND_NUDGE_EXTRA_INFO
    {
        // The zero-key nudge is an activation aid, not a keyboard gesture.
        // Consume it before normal input normalization so it cannot leak to
        // the old foreground HWND or affect any modifier ledger.
        return true;
    }
    if injected
        && (!shared.accept_marked_test_input || event.dwExtraInfo != MARKED_TEST_INPUT_EXTRA_INFO)
    {
        return false;
    }
    // An accepted marked event intentionally continues with physical
    // semantics: the acceptance path must exercise the same modifier and
    // release ledgers as a hardware transition. Unmarked replay never reaches
    // this point, so it cannot feed back into FrigoTab.
    let virtual_key = event.vkCode as u16;
    let (key, modifier) = map_virtual_key(virtual_key);

    let input;
    let mut consume = false;
    let mut recovery = None;
    let mut replay = [INPUT::default(); 3];
    let mut replay_count = 0usize;
    let mut replay_and_consume = false;
    let mut replay_leaves_alt_down = false;
    let mut discard_gesture = false;
    let mut pending_releases = [(0u16, 0usize); 3];
    let mut pending_release_count = 0usize;
    let mut current_release = None;
    {
        let mut state = shared
            .input
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if shared.disposed.load(Ordering::Acquire) {
            return false;
        }
        let shift_was_down = state.modifiers.shift_down();
        input = state.modifiers.create_input_with_modifier(
            key,
            transition,
            event.flags & LLKHF_ALTDOWN != 0,
            false,
            modifier,
        );

        if native_release_is_blocked(&state, virtual_key) {
            if input.is_up() {
                unblock_native_release(&mut state, virtual_key);
                state.suppression.forget_matching_release(key);
            }
            return true;
        }

        let session_active = state.suppression.session_active_or_pending();
        if !session_active
            && key != SwitcherKey::Unknown
            && state.suppression.consumes_matching_release(key)
        {
            if input.is_up() {
                state.suppression.forget_matching_release(key);
            }
            // After a close or interrupted admission, neither an auto-repeat
            // nor its eventual release may escape merely because a new
            // ordinary Alt chord has entered native-forwarding mode.
            return true;
        }

        // A zero-result SendInput replay is failed closed until physical Alt
        // is released. Do not let later keys become an unintended unmodified
        // gesture merely because Windows rejected the synthetic shortcut.
        if blocked_alt_is_down(&state) {
            if input.is_down() {
                block_native_release(&mut state, virtual_key);
                return true;
            }
            return false;
        }

        // Ctrl+Alt includes AltGr and Windows' own Ctrl+Alt+Tab variant. Keep
        // that whole chord native instead of taking ownership halfway through.
        if state.alt_forwarded {
            if key == SwitcherKey::Alt && input.is_up() && !state.modifiers.alt_down() {
                state.alt_forwarded = false;
            }
            return false;
        }

        let tracked_alt = state.deferred_alt_vk != 0;
        let same_alt = tracked_alt && same_alt_key(state.deferred_alt_vk, virtual_key);
        let tracked_shift = state.deferred_shift_vk != 0;
        let same_shift = tracked_shift && same_shift_key(state.deferred_shift_vk, virtual_key);

        // A held Alt with no deferred down can start another cycle only when
        // the release ledger proves that FrigoTab intercepted its down in an
        // earlier cycle. Otherwise Alt was already held when the hook began or
        // reset, so its down reached the foreground application and the whole
        // gesture must remain native.
        if input.is_down()
            && key == SwitcherKey::Tab
            && input.alt
            && !tracked_alt
            && !session_active
            && !state
                .suppression
                .consumes_matching_release(SwitcherKey::Alt)
        {
            return false;
        }

        if key == SwitcherKey::Alt && input.is_down() {
            if session_active {
                // Modifier-down is useful only to the hook's release ledger;
                // avoid queueing work which the controller deliberately ignores.
                return state.suppression.should_consume_with_token(input).0;
            }
            if !tracked_alt && state.modifiers.control_down() {
                state.alt_forwarded = true;
                return false;
            }
            if !tracked_alt {
                state.deferred_alt_vk = virtual_key;
                state.deferred_alt_scan = event.scanCode as u16;
                state.deferred_alt_extended = event.flags & LLKHF_EXTENDED != 0;
                return true;
            }
            if same_alt {
                return true;
            }

            // Two different Alt keys identify an ordinary modifier chord.
            // Replay both downs in their original order and suppress this
            // physical down so the target never sees a partial sequence.
            replay[0] = native_keyboard_input(
                state.deferred_alt_vk,
                state.deferred_alt_scan,
                state.deferred_alt_extended,
                false,
            );
            if tracked_shift {
                replay[1] = native_keyboard_input(
                    state.deferred_shift_vk,
                    state.deferred_shift_scan,
                    state.deferred_shift_extended,
                    false,
                );
                replay[2] = native_keyboard_input(
                    virtual_key,
                    event.scanCode as u16,
                    event.flags & LLKHF_EXTENDED != 0,
                    false,
                );
                replay_count = 3;
                pending_releases[1] = (state.deferred_shift_vk, 1);
                pending_releases[2] = (virtual_key, 2);
                pending_release_count = 3;
            } else {
                replay[1] = native_keyboard_input(
                    virtual_key,
                    event.scanCode as u16,
                    event.flags & LLKHF_EXTENDED != 0,
                    false,
                );
                replay_count = 2;
                pending_releases[1] = (virtual_key, 1);
                pending_release_count = 2;
            }
            replay_and_consume = true;
            replay_leaves_alt_down = true;
            pending_releases[0] = (state.deferred_alt_vk, 0);
            clear_deferred_alt(&mut state);
            clear_deferred_shift(&mut state);
        } else if key == SwitcherKey::Alt && input.is_up() && same_alt {
            // No Tab followed. Preserve the exact modifier order, including a
            // deferred Shift if this was an ordinary Alt+Shift chord.
            replay[0] = native_keyboard_input(
                state.deferred_alt_vk,
                state.deferred_alt_scan,
                state.deferred_alt_extended,
                false,
            );
            if tracked_shift {
                replay[1] = native_keyboard_input(
                    state.deferred_shift_vk,
                    state.deferred_shift_scan,
                    state.deferred_shift_extended,
                    false,
                );
                replay[2] = native_keyboard_input(
                    state.deferred_alt_vk,
                    state.deferred_alt_scan,
                    state.deferred_alt_extended,
                    true,
                );
                replay_count = 3;
                pending_releases[0] = (state.deferred_shift_vk, 1);
                pending_release_count = 1;
                current_release = Some((0, 2));
            } else {
                replay[1] = native_keyboard_input(
                    state.deferred_alt_vk,
                    state.deferred_alt_scan,
                    state.deferred_alt_extended,
                    true,
                );
                replay_count = 2;
                current_release = Some((0, 1));
            }
            replay_and_consume = true;
            state.suppression.forget_matching_release(SwitcherKey::Alt);
            clear_deferred_alt(&mut state);
            clear_deferred_shift(&mut state);
        } else if key == SwitcherKey::Shift && input.is_down() && tracked_alt {
            if !tracked_shift && shift_was_down {
                // This Shift was already native before Alt (or is its repeat),
                // so it must stay on the pre-existing balancing path.
                return false;
            }
            if !tracked_shift {
                state.deferred_shift_vk = virtual_key;
                state.deferred_shift_scan = event.scanCode as u16;
                state.deferred_shift_extended = event.flags & LLKHF_EXTENDED != 0;
                return true;
            }
            if same_shift {
                return true;
            }

            // Two distinct Shift keys make this an ordinary modifier chord.
            replay[0] = native_keyboard_input(
                state.deferred_alt_vk,
                state.deferred_alt_scan,
                state.deferred_alt_extended,
                false,
            );
            replay[1] = native_keyboard_input(
                state.deferred_shift_vk,
                state.deferred_shift_scan,
                state.deferred_shift_extended,
                false,
            );
            replay[2] = native_keyboard_input(
                virtual_key,
                event.scanCode as u16,
                event.flags & LLKHF_EXTENDED != 0,
                false,
            );
            replay_count = 3;
            replay_and_consume = true;
            replay_leaves_alt_down = true;
            pending_releases[0] = (state.deferred_alt_vk, 0);
            pending_releases[1] = (state.deferred_shift_vk, 1);
            pending_releases[2] = (virtual_key, 2);
            pending_release_count = 3;
            clear_deferred_alt(&mut state);
            clear_deferred_shift(&mut state);
        } else if key == SwitcherKey::Shift && input.is_up() && tracked_alt && same_shift {
            // Alt+Shift without Tab is a normal Windows/layout gesture. Replay
            // it in order while leaving Alt held for the physical Alt-up.
            replay[0] = native_keyboard_input(
                state.deferred_alt_vk,
                state.deferred_alt_scan,
                state.deferred_alt_extended,
                false,
            );
            replay[1] = native_keyboard_input(
                state.deferred_shift_vk,
                state.deferred_shift_scan,
                state.deferred_shift_extended,
                false,
            );
            replay[2] = native_keyboard_input(
                state.deferred_shift_vk,
                state.deferred_shift_scan,
                state.deferred_shift_extended,
                true,
            );
            replay_count = 3;
            replay_and_consume = true;
            replay_leaves_alt_down = true;
            pending_releases[0] = (state.deferred_alt_vk, 0);
            pending_release_count = 1;
            current_release = Some((1, 2));
            state
                .suppression
                .forget_matching_release(SwitcherKey::Shift);
            clear_deferred_alt(&mut state);
            clear_deferred_shift(&mut state);
        } else if input.is_down() && key == SwitcherKey::Tab && input.alt && tracked_alt {
            // Promote every gated modifier into the switcher gesture. Gated
            // transitions are never published to the previous application.
            clear_deferred_alt(&mut state);
            state.suppression.consume_matching_release(SwitcherKey::Alt);
            if tracked_shift {
                clear_deferred_shift(&mut state);
                state
                    .suppression
                    .consume_matching_release(SwitcherKey::Shift);
            } else if input.shift {
                // Shift was already down before Alt, so its down event cannot
                // be retroactively hidden. Balance it while the old window is
                // still foreground, then swallow the later physical release.
                let mut shift_ups = [INPUT::default(); 3];
                let mut shift_keys = [0u16; 3];
                let mut shift_count = 0usize;
                for modifier in state.modifiers.pressed_shift_keys().into_iter().flatten() {
                    let virtual_key = shift_virtual_key(modifier);
                    shift_ups[shift_count] = native_keyboard_input(virtual_key, 0, false, true);
                    shift_keys[shift_count] = virtual_key;
                    shift_count += 1;
                }
                let sent = send_native_keyboard_inputs(
                    &shift_ups[..shift_count],
                    "pre-existing Shift balance",
                );
                for &virtual_key in &shift_keys[..sent] {
                    block_native_release(&mut state, virtual_key);
                }
                if sent == shift_count {
                    state
                        .suppression
                        .consume_matching_release(SwitcherKey::Shift);
                } else {
                    // Do not move foreground while the old target still owns
                    // an unbalanced Shift. Discard this switch request; every
                    // unsent physical Shift release stays with the old target.
                    state.suppression.consume_matching_release(SwitcherKey::Tab);
                    discard_gesture = true;
                }
            }
        } else if input.is_down() && tracked_alt {
            // This is an ordinary Alt shortcut. Replay every deferred modifier
            // and this exact key-down atomically. Physical releases will then
            // balance only the downs Windows actually accepted.
            replay[0] = native_keyboard_input(
                state.deferred_alt_vk,
                state.deferred_alt_scan,
                state.deferred_alt_extended,
                false,
            );
            pending_releases[0] = (state.deferred_alt_vk, 0);
            if tracked_shift {
                replay[1] = native_keyboard_input(
                    state.deferred_shift_vk,
                    state.deferred_shift_scan,
                    state.deferred_shift_extended,
                    false,
                );
                replay[2] = native_keyboard_input(
                    virtual_key,
                    event.scanCode as u16,
                    event.flags & LLKHF_EXTENDED != 0,
                    false,
                );
                pending_releases[1] = (state.deferred_shift_vk, 1);
                pending_releases[2] = (virtual_key, 2);
                pending_release_count = 3;
                replay_count = 3;
            } else {
                replay[1] = native_keyboard_input(
                    virtual_key,
                    event.scanCode as u16,
                    event.flags & LLKHF_EXTENDED != 0,
                    false,
                );
                pending_releases[1] = (virtual_key, 1);
                pending_release_count = 2;
                replay_count = 2;
            }
            replay_and_consume = true;
            replay_leaves_alt_down = true;
            clear_deferred_alt(&mut state);
            clear_deferred_shift(&mut state);
        }

        if !replay_and_consume && !discard_gesture && recovery.is_none() {
            if key == SwitcherKey::Unknown {
                if session_active && input.is_down() {
                    block_native_release(&mut state, virtual_key);
                    return true;
                }
                return false;
            }
            let (should_consume, admission_token) =
                state.suppression.should_consume_with_token(input);
            consume = should_consume;

            let ending = is_session_ending_input(input);
            let delivered = if ending {
                shared.dispatcher.try_dispatch_critical(input)
            } else {
                shared.dispatcher.try_dispatch(input)
            };
            if !delivered {
                if consume && state.suppression.abort_pending_admission(admission_token) {
                    let alt_reached_target = !state
                        .suppression
                        .consumes_matching_release(SwitcherKey::Alt)
                        && state.modifiers.alt_down();
                    let shift_reached_target = !state
                        .suppression
                        .consumes_matching_release(SwitcherKey::Shift)
                        && state.modifiers.shift_down();
                    recovery = Some((alt_reached_target, input.shift, shift_reached_target));
                } else {
                    return consume;
                }
            }
        }
    }

    if replay_and_consume {
        let sent = send_native_keyboard_inputs(&replay[..replay_count], "Alt key replay");
        let mut state = shared
            .input
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if replay_leaves_alt_down && sent > 0 {
            state.alt_forwarded = true;
        }
        if sent != replay_count {
            for &(key, down_index) in &pending_releases[..pending_release_count] {
                if down_index >= sent {
                    block_native_release(&mut state, key);
                }
            }
        }
        let pass_current_release = current_release
            .is_some_and(|(down_index, up_index)| down_index < sent && up_index >= sent);
        return !pass_current_release;
    }

    if discard_gesture {
        return true;
    }

    if let Some((alt_reached_target, reverse, shift_reached_target)) = recovery {
        // This is the only native input recovery path.  The fixed gesture is
        // bounded and is executed after releasing the input-state lock. The
        // original physical Tab-down was already admitted for suppression;
        // keep it suppressed even if SendInput itself fails, and retain its
        // ledger entry so the later physical Tab-up is suppressed as well.
        let _ = replay_native_alt_tab(alt_reached_target, reverse, shift_reached_target);
        return true;
    }
    consume
}

fn is_session_ending_input(input: KeyboardInput) -> bool {
    (input.is_up() && input.key == SwitcherKey::Alt)
        || (input.is_down() && input.key == SwitcherKey::Escape)
        || (input.is_down() && input.key == SwitcherKey::F4 && input.alt)
}

fn map_virtual_key(vk: u16) -> (SwitcherKey, KeyboardModifierKey) {
    let key = match vk {
        VK_TAB => SwitcherKey::Tab,
        VK_ESCAPE => SwitcherKey::Escape,
        VK_F4 => SwitcherKey::F4,
        VK_LMENU | VK_RMENU | VK_MENU => SwitcherKey::Alt,
        VK_LSHIFT | VK_RSHIFT | VK_SHIFT => SwitcherKey::Shift,
        VK_1..=VK_9 => match vk - VK_1 {
            0 => SwitcherKey::D1,
            1 => SwitcherKey::D2,
            2 => SwitcherKey::D3,
            3 => SwitcherKey::D4,
            4 => SwitcherKey::D5,
            5 => SwitcherKey::D6,
            6 => SwitcherKey::D7,
            7 => SwitcherKey::D8,
            _ => SwitcherKey::D9,
        },
        VK_NUMPAD1..=VK_NUMPAD9 => match vk - VK_NUMPAD1 {
            0 => SwitcherKey::NumPad1,
            1 => SwitcherKey::NumPad2,
            2 => SwitcherKey::NumPad3,
            3 => SwitcherKey::NumPad4,
            4 => SwitcherKey::NumPad5,
            5 => SwitcherKey::NumPad6,
            6 => SwitcherKey::NumPad7,
            7 => SwitcherKey::NumPad8,
            _ => SwitcherKey::NumPad9,
        },
        _ => SwitcherKey::Unknown,
    };
    let modifier = match vk {
        VK_MENU => KeyboardModifierKey::Alt,
        VK_LMENU => KeyboardModifierKey::LeftAlt,
        VK_RMENU => KeyboardModifierKey::RightAlt,
        VK_SHIFT => KeyboardModifierKey::Shift,
        VK_LSHIFT => KeyboardModifierKey::LeftShift,
        VK_RSHIFT => KeyboardModifierKey::RightShift,
        VK_CONTROL => KeyboardModifierKey::Control,
        VK_LCONTROL => KeyboardModifierKey::LeftControl,
        VK_RCONTROL => KeyboardModifierKey::RightControl,
        _ => KeyboardModifierKey::None,
    };
    (key, modifier)
}

fn map_legacy_key(key: SwitcherKey) -> Option<u16> {
    match key {
        SwitcherKey::Alt => Some(VK_MENU),
        SwitcherKey::Shift => Some(VK_SHIFT),
        SwitcherKey::Tab => Some(VK_TAB),
        SwitcherKey::Escape => Some(VK_ESCAPE),
        SwitcherKey::F4 => Some(VK_F4),
        SwitcherKey::D1 => Some(VK_1),
        SwitcherKey::D2 => Some(VK_1 + 1),
        SwitcherKey::D3 => Some(VK_1 + 2),
        SwitcherKey::D4 => Some(VK_1 + 3),
        SwitcherKey::D5 => Some(VK_1 + 4),
        SwitcherKey::D6 => Some(VK_1 + 5),
        SwitcherKey::D7 => Some(VK_1 + 6),
        SwitcherKey::D8 => Some(VK_1 + 7),
        SwitcherKey::D9 => Some(VK_9),
        SwitcherKey::NumPad1 => Some(VK_NUMPAD1),
        SwitcherKey::NumPad2 => Some(VK_NUMPAD1 + 1),
        SwitcherKey::NumPad3 => Some(VK_NUMPAD1 + 2),
        SwitcherKey::NumPad4 => Some(VK_NUMPAD1 + 3),
        SwitcherKey::NumPad5 => Some(VK_NUMPAD1 + 4),
        SwitcherKey::NumPad6 => Some(VK_NUMPAD1 + 5),
        SwitcherKey::NumPad7 => Some(VK_NUMPAD1 + 6),
        SwitcherKey::NumPad8 => Some(VK_NUMPAD1 + 7),
        SwitcherKey::NumPad9 => Some(VK_NUMPAD9),
        SwitcherKey::Unknown => None,
    }
}

fn shift_virtual_key(modifier: KeyboardModifierKey) -> u16 {
    match modifier {
        KeyboardModifierKey::LeftShift => VK_LSHIFT,
        KeyboardModifierKey::RightShift => VK_RSHIFT,
        _ => VK_SHIFT,
    }
}

fn same_alt_key(stored: u16, current: u16) -> bool {
    stored == current || stored == VK_MENU || current == VK_MENU
}

fn same_shift_key(stored: u16, current: u16) -> bool {
    stored == current || stored == VK_SHIFT || current == VK_SHIFT
}

fn clear_deferred_alt(state: &mut InputState) {
    state.deferred_alt_vk = 0;
    state.deferred_alt_scan = 0;
    state.deferred_alt_extended = false;
}

fn clear_deferred_shift(state: &mut InputState) {
    state.deferred_shift_vk = 0;
    state.deferred_shift_scan = 0;
    state.deferred_shift_extended = false;
}

fn native_key_bit(virtual_key: u16) -> Option<(usize, u64)> {
    if virtual_key > u8::MAX as u16 {
        return None;
    }
    let value = virtual_key as usize;
    Some((value / 64, 1u64 << (value % 64)))
}

fn block_native_release(state: &mut InputState, virtual_key: u16) {
    if let Some((word, bit)) = native_key_bit(virtual_key) {
        state.blocked_native_releases[word] |= bit;
    }
}

fn unblock_native_release(state: &mut InputState, virtual_key: u16) {
    if let Some((word, bit)) = native_key_bit(virtual_key) {
        state.blocked_native_releases[word] &= !bit;
    }
}

fn native_release_is_blocked(state: &InputState, virtual_key: u16) -> bool {
    native_key_bit(virtual_key)
        .is_some_and(|(word, bit)| state.blocked_native_releases[word] & bit != 0)
}

fn blocked_alt_is_down(state: &InputState) -> bool {
    native_release_is_blocked(state, VK_MENU)
        || native_release_is_blocked(state, VK_LMENU)
        || native_release_is_blocked(state, VK_RMENU)
}

fn native_keyboard_input(virtual_key: u16, scan_code: u16, extended: bool, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: virtual_key,
                wScan: scan_code,
                dwFlags: (if extended { KEYEVENTF_EXTENDEDKEY } else { 0 })
                    | (if up { KEYEVENTF_KEYUP } else { 0 }),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_native_keyboard_inputs(inputs: &[INPUT], operation: &str) -> usize {
    if inputs.is_empty() {
        return 0;
    }
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        )
    };
    if sent != inputs.len() as u32 {
        eprintln!(
            "FrigoTab {operation} SendInput wrote {sent} of {}: {}",
            inputs.len(),
            unsafe { GetLastError() }
        );
    }
    sent as usize
}

fn replay_native_alt_tab(
    alt_reached_target: bool,
    reverse: bool,
    shift_reached_target: bool,
) -> bool {
    let plan = AltTabRecoveryPlan::create(alt_reached_target, reverse, shift_reached_target);
    let mut native = [INPUT::default(); 6];
    let mut keys = [SwitcherKey::Unknown; 6];
    let mut downs = [false; 6];
    let mut count = 0usize;
    for input in plan {
        let Some(virtual_key) = map_legacy_key(input.key) else {
            continue;
        };
        native[count] = native_keyboard_input(virtual_key, 0, false, input.is_up());
        keys[count] = input.key;
        downs[count] = input.is_down();
        count += 1;
        if count == native.len() {
            break;
        }
    }
    let sent = send_native_keyboard_inputs(&native[..count], "Alt+Tab recovery");
    if sent == count {
        return true;
    }

    // SendInput normally accepts a whole transaction or none of it. If a
    // provider inserts only a prefix, release every down from that prefix so
    // recovery cannot strand an injected Alt, Shift, or Tab modifier/state.
    let mut affected_keys = [SwitcherKey::Unknown; 6];
    let mut key_balance = [0i8; 6];
    let mut affected_count = 0usize;
    for index in 0..sent {
        let position = (0..affected_count)
            .find(|&position| affected_keys[position] == keys[index])
            .unwrap_or_else(|| {
                let position = affected_count;
                affected_keys[position] = keys[index];
                affected_count += 1;
                position
            });
        key_balance[position] += if downs[index] { 1 } else { -1 };
    }

    let mut cleanup = [INPUT::default(); 6];
    let mut cleanup_count = 0usize;
    for index in (0..affected_count).rev() {
        let Some(virtual_key) = map_legacy_key(affected_keys[index]) else {
            continue;
        };
        while key_balance[index] > 0 {
            cleanup[cleanup_count] = native_keyboard_input(virtual_key, 0, false, true);
            cleanup_count += 1;
            key_balance[index] -= 1;
        }
        while key_balance[index] < 0 {
            cleanup[cleanup_count] = native_keyboard_input(virtual_key, 0, false, false);
            cleanup_count += 1;
            key_balance[index] += 1;
        }
    }
    if cleanup_count != 0 {
        let mut released = 0usize;
        while released < cleanup_count {
            let count = send_native_keyboard_inputs(
                &cleanup[released..cleanup_count],
                "partial Alt+Tab recovery cleanup",
            );
            if count == 0 {
                break;
            }
            released += count;
        }
    }
    false
}

fn win32_error(operation: &'static str) -> KeyHookError {
    KeyHookError::Win32 {
        operation,
        code: unsafe { GetLastError() },
    }
}
