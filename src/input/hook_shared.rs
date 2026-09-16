use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32};
use std::sync::{Arc, Mutex};

use super::callback_queue::CallbackQueue;
use super::delivered_inputs::DeliveredInputs;
use super::input_state::InputState;
use crate::input::DeferredKeyboardDispatcher;

pub(crate) struct HookShared {
    pub(crate) input: Mutex<InputState>,
    pub(crate) dispatcher: Arc<DeferredKeyboardDispatcher>,
    pub(crate) callbacks: Arc<CallbackQueue>,
    pub(crate) delivered: Arc<DeliveredInputs>,
    pub(crate) disposed: AtomicBool,
    pub(crate) thread_id: AtomicU32,
    pub(crate) hook_id: AtomicPtr<std::ffi::c_void>,
}
