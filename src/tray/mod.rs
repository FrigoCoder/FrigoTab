//! Native notification-area integration components.

mod application_icon;
mod owned_icon;
mod popup_menu;

pub mod sys_tray_icon;
pub mod tray_action;

pub use sys_tray_icon::{SysTrayIcon, TRAY_CALLBACK_MESSAGE};
pub use tray_action::TrayAction;
