//! Native system integration components.

mod owned_handle;

pub mod application_ready;
pub mod single_instance_error;
pub mod single_instance_guard;

pub use application_ready::WM_APPLICATION_READY;
pub use single_instance_error::SingleInstanceError;
pub use single_instance_guard::{APPLICATION_MUTEX_NAME, SingleInstanceGuard};
