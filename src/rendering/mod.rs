//! Native graphics primitives used by the switcher tiles.
//!
//! The public modules mirror the original top-level modules while the
//! secondary modules keep each public type in its own file.  The top-level
//! `gdi_plus` module is a compatibility façade for callers that use the
//! original API.

pub mod error;
pub mod font;
pub mod gdi_plus;
pub mod graphics;
pub mod layer_updater;
pub mod thumbnail;
pub mod window_snapshot;

pub use error::{Error, Result};
pub use font::Font;
pub use graphics::Graphics;
pub use layer_updater::LayerUpdater;
pub use startup::ensure_started;
pub use thumbnail::DwmThumbnail;
pub use window_snapshot::WindowSnapshot;

mod font_family;
mod graphics_dc;
mod layer_errors;
mod memory_surface;
mod screen_dc;
mod solid_brush;
mod startup;
