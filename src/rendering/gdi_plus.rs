//! Compatibility façade for the original GDI+ module.
//!
//! The concrete types live in their own modules so each public type has one
//! primary source file.  Re-exporting them here keeps existing imports such
//! as `crate::gdi_plus::Graphics` source-compatible.

pub use super::error::{Error, Result};
pub use super::font::Font;
pub use super::graphics::Graphics;
pub use super::startup::ensure_started;
