//! Native system integration components.

mod owned_handle;

pub mod single_instance_error;
pub mod single_instance_guard;

pub use single_instance_error::SingleInstanceError;
pub use single_instance_guard::{APPLICATION_MUTEX_NAME, SingleInstanceGuard};
