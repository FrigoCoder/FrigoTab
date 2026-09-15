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

use std::collections::VecDeque;
use std::fmt;
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
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_1, VK_9, VK_ESCAPE,
    VK_F4, VK_LMENU, VK_LSHIFT, VK_MENU, VK_NUMPAD1, VK_NUMPAD9, VK_RMENU, VK_RSHIFT, VK_SHIFT,
    VK_TAB,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, LLKHF_INJECTED,
    LLKHF_LOWER_IL_INJECTED, MSG, PM_NOREMOVE, PeekMessageW, PostMessageW, PostThreadMessageW,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_APP, WM_KEYDOWN,
    WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::alt_tab_recovery_plan::AltTabRecoveryPlan;
use crate::deferred_keyboard_dispatcher::DeferredKeyboardDispatcher;
use crate::key_handling::KeyHandling;
use crate::keyboard_input::{KeyTransition, KeyboardInput, KeyboardModifierKey, SwitcherKey};
use crate::keyboard_modifier_state::KeyboardModifierState;
use crate::keyboard_suppression_state::KeyboardSuppressionState;
use crate::window_handle::WindowHandle;

/// Message posted to the UI window for an admitted keyboard callback.
///
/// `wParam` is a one-based slot number owned by this [`KeyHook`].  A slot is
/// taken exactly once by [`KeyHook::dispatch_ui_message`], and slots that are
/// still queued are dropped by `drain_ui_messages` during shutdown.
pub const WM_KEY_HOOK_INPUT: u32 = WM_APP + 2;

const DISPATCH_CAPACITY: usize = 64;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

type UiCallback = Box<dyn FnOnce() + Send + 'static>;

/// A fixed-size owner for callbacks posted to the UI queue.
///
/// Using numeric slots instead of putting a raw allocation in `LPARAM` means
/// the hook can release every callback if its owner HWND is destroyed before
/// Windows delivers the message.  The queue never grows beyond the same
/// 64+1 bound used by the deferred dispatcher.
struct CallbackQueue {
    slots: Mutex<Vec<Option<UiCallback>>>,
}

impl CallbackQueue {
    fn new() -> Self {
        Self {
            slots: Mutex::new((0..DISPATCH_CAPACITY + 1).map(|_| None).collect()),
        }
    }

    fn insert(&self, callback: UiCallback) -> Option<usize> {
        let mut slots = self.slots.lock().unwrap_or_else(|error| error.into_inner());
        let index = slots.iter().position(Option::is_none)?;
        slots[index] = Some(callback);
        Some(index + 1)
    }

    fn take(&self, slot: usize) -> Option<UiCallback> {
        if slot == 0 {
            return None;
        }
        let mut slots = self.slots.lock().unwrap_or_else(|error| error.into_inner());
        slots.get_mut(slot - 1)?.take()
    }

    fn drain(&self) {
        let mut slots = self.slots.lock().unwrap_or_else(|error| error.into_inner());
        for callback in slots.iter_mut() {
            // Dropping a callback releases its captured KeyboardInput and
            // any dispatcher release guard it owns.
            callback.take();
        }
    }
}

struct DeliveredInputs {
    inputs: Mutex<VecDeque<KeyboardInput>>,
}

impl DeliveredInputs {
    fn new() -> Self {
        Self {
            inputs: Mutex::new(VecDeque::with_capacity(DISPATCH_CAPACITY + 1)),
        }
    }

    fn push(&self, input: KeyboardInput) {
        let mut inputs = self
            .inputs
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        // One UI message consumes one callback before the next is dispatched.
        // Keep this defensive bound in case a future dispatcher changes its
        // ordering; dropping here is safer than allocating in the UI bridge.
        if inputs.len() < DISPATCH_CAPACITY + 1 {
            inputs.push_back(input);
        }
    }

    fn pop(&self) -> Option<KeyboardInput> {
        self.inputs
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .pop_front()
    }
}

struct InputState {
    modifiers: KeyboardModifierState,
    suppression: KeyboardSuppressionState,
}

struct HookShared {
    input: Mutex<InputState>,
    dispatcher: Arc<DeferredKeyboardDispatcher>,
    callbacks: Arc<CallbackQueue>,
    delivered: Arc<DeliveredInputs>,
    disposed: AtomicBool,
    thread_id: AtomicU32,
    hook_id: AtomicPtr<std::ffi::c_void>,
}

// WH_KEYBOARD_LL callbacks do not receive a user-data pointer.  The process
// has one production hook, so an atomic pointer is the smallest bridge to the
// Arc-owned state.  It remains valid until the hook thread has unhooked and
// been joined.
static ACTIVE_HOOK: AtomicPtr<HookShared> = AtomicPtr::new(null_mut());

/// Errors returned while starting or shutting down the native hook.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum KeyHookError {
    InvalidOwner,
    ThreadStart,
    StartupTimeout,
    ThreadExited,
    AlreadyInstalled,
    Win32 { operation: &'static str, code: u32 },
}

impl fmt::Display for KeyHookError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOwner => {
                formatter.write_str("keyboard hook owner HWND is null or invalid")
            }
            Self::ThreadStart => formatter.write_str("unable to start keyboard hook thread"),
            Self::StartupTimeout => formatter.write_str("keyboard hook startup timed out"),
            Self::ThreadExited => formatter.write_str("keyboard hook thread exited during startup"),
            Self::AlreadyInstalled => {
                formatter.write_str("only one FrigoTab keyboard hook may be installed")
            }
            Self::Win32 { operation, code } => {
                write!(formatter, "{operation} failed with Win32 error {code}")
            }
        }
    }
}

impl std::error::Error for KeyHookError {}

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
        if !WindowHandle::new(owner).is_valid() {
            return Err(KeyHookError::InvalidOwner);
        }

        let callbacks = Arc::new(CallbackQueue::new());
        let delivered = Arc::new(DeliveredInputs::new());
        let post_callbacks = Arc::clone(&callbacks);
        let delivered_by_handler = Arc::clone(&delivered);
        let owner_value = owner as usize;

        let dispatcher = Arc::new(
            DeferredKeyboardDispatcher::new(
                move |callback: UiCallback| post_callback(&post_callbacks, owner_value, callback),
                move |input| delivered_by_handler.push(input),
                DISPATCH_CAPACITY,
            )
            .map_err(|_| KeyHookError::ThreadStart)?,
        );

        let shared = Arc::new(HookShared {
            input: Mutex::new(InputState {
                modifiers: KeyboardModifierState::new(),
                suppression: KeyboardSuppressionState::new(),
            }),
            dispatcher,
            callbacks,
            delivered,
            disposed: AtomicBool::new(false),
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

    /// Invalidate queued callbacks and clear modifier/suppression state after
    /// a desktop interruption, display transition, or shutdown.
    pub fn reset_input_state(&self) {
        let mut input = self
            .shared
            .input
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.shared.dispatcher.invalidate_pending();
        input.modifiers.reset();
        input.suppression.reset();
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
            let (alt_still_down, shift_still_down) = {
                let mut state = self
                    .shared
                    .input
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                self.shared.dispatcher.invalidate_pending();
                state.suppression.set_session_visible(false);
                (state.modifiers.alt_down(), state.modifiers.shift_down())
            };
            let _ = replay_native_alt_tab(alt_still_down, input.shift, shift_still_down);
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

struct CompletionSignal(Option<mpsc::SyncSender<()>>);

impl Drop for CompletionSignal {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
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
    let injected = event.flags & (LLKHF_INJECTED | LLKHF_LOWER_IL_INJECTED) != 0;
    let (key, modifier) = map_virtual_key(event.vkCode as u16);
    if key == SwitcherKey::Unknown || injected {
        return false;
    }

    let input;
    let consume;
    let mut recovery = None;
    {
        let mut state = shared
            .input
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if shared.disposed.load(Ordering::Acquire) {
            return false;
        }
        input = state.modifiers.create_input_with_modifier(
            key,
            transition,
            event.flags & LLKHF_ALTDOWN != 0,
            false,
            modifier,
        );
        let (should_consume, admission_token) = state.suppression.should_consume_with_token(input);
        consume = should_consume;

        let ending = is_session_ending_input(input);
        let delivered = if ending {
            shared.dispatcher.try_dispatch_critical(input)
        } else {
            shared.dispatcher.try_dispatch(input)
        };
        if !delivered {
            if consume && state.suppression.abort_pending_admission(admission_token) {
                recovery = Some((
                    state.modifiers.alt_down(),
                    input.shift,
                    state.modifiers.shift_down(),
                ));
            } else {
                return consume;
            }
        }
    }

    if let Some((alt_still_down, reverse, shift_still_down)) = recovery {
        // This is the only native input recovery path.  The fixed gesture is
        // bounded and is executed after releasing the input-state lock.
        return replay_native_alt_tab(alt_still_down, reverse, shift_still_down);
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

fn replay_native_alt_tab(alt_still_down: bool, reverse: bool, shift_still_down: bool) -> bool {
    let plan = AltTabRecoveryPlan::create(alt_still_down, reverse, shift_still_down);
    let mut native = [INPUT::default(); 6];
    let mut count = 0usize;
    for input in plan {
        let Some(virtual_key) = map_legacy_key(input.key) else {
            continue;
        };
        native[count] = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: virtual_key,
                    wScan: 0,
                    dwFlags: if input.is_up() { KEYEVENTF_KEYUP } else { 0 },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        count += 1;
        if count == native.len() {
            break;
        }
    }
    if count == 0 {
        return false;
    }
    let sent = unsafe { SendInput(count as u32, native.as_ptr(), size_of::<INPUT>() as i32) };
    if sent != count as u32 {
        eprintln!(
            "FrigoTab Alt+Tab recovery SendInput wrote {sent} of {count}: {}",
            unsafe { GetLastError() }
        );
        return false;
    }
    true
}

fn win32_error(operation: &'static str) -> KeyHookError {
    KeyHookError::Win32 {
        operation,
        code: unsafe { GetLastError() },
    }
}
