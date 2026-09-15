//! One application preview and its title/number overlay.
//!
//! This follows the original `ApplicationWindow` contract: the DWM thumbnail
//! is registered against the session owner, while this separate owned popup
//! contains only the transparent/layered GDI+ overlay.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;

use crate::frigo_window::FrigoWindow;
use crate::gdi_plus::Font;
use crate::layer_updater::LayerUpdater;
use crate::thumbnail::DwmThumbnail;
use crate::window_handle::WindowHandle;
use crate::window_icon::WindowIcon;
use windows_sys::Win32::Graphics::GdiPlus::{FontStyleBold, FontStyleRegular, RectF};

const PAD: f32 = 8.0;
const ARGB_BLACK: u32 = 0xff00_0000;
const ARGB_WHITE: u32 = 0xffff_ffff;
const ARGB_SELECTED: u32 = 0x8000_00ff;

/// A separate, owned top-level popup carrying one application's overlay.
pub struct ApplicationWindow {
    // Declaration order is intentional: Rust drops fields in this order,
    // matching ApplicationWindow.Dispose (icon, layer, thumbnail, popup).
    window_icon: WindowIcon,
    layer_updater: Rc<RefCell<LayerUpdater>>,
    thumbnail: Option<DwmThumbnail>,
    popup: FrigoWindow,
    application: HWND,
    index: usize,
    bounds: RECT,
    selected: bool,
    selected_state: Rc<Cell<bool>>,
}

impl ApplicationWindow {
    pub fn new(owner: HWND, application: HWND, index: usize, bounds: RECT) -> Result<Self, String> {
        let thumbnail = match DwmThumbnail::register(owner, application) {
            Ok(thumbnail) => {
                let configured = thumbnail
                    .set_destination_rect(screen_to_client_rect(owner, bounds))
                    .map_err(|hresult| {
                        format!("DwmUpdateThumbnailProperties failed (HRESULT 0x{hresult:08x})")
                    });
                if configured.is_err() || thumbnail.set_visible(false).is_err() {
                    None
                } else {
                    Some(thumbnail)
                }
            }
            Err(_) => None,
        };
        let popup = FrigoWindow::new(owner, bounds)?;
        let layer_updater = Rc::new(RefCell::new(LayerUpdater::new(popup.hwnd(), bounds)?));
        let window_icon = WindowIcon::new(application)?;
        let icon_state = window_icon.weak();
        let callback_layer = Rc::clone(&layer_updater);
        let selected_state = Rc::new(Cell::new(false));
        let callback_selected = Rc::clone(&selected_state);
        window_icon.on_changed(move || {
            let _ = icon_state.with_icon(|icon, icon_width, icon_height| {
                let _ = render_overlay_with_icon(
                    &mut callback_layer.borrow_mut(),
                    icon,
                    icon_width,
                    icon_height,
                    application,
                    index,
                    callback_selected.get(),
                );
            });
        });

        let mut result = Self {
            popup,
            application,
            index,
            bounds,
            selected: false,
            selected_state,
            window_icon,
            layer_updater,
            thumbnail,
        };
        result.render_overlay()?;
        Ok(result)
    }

    pub fn hwnd(&self) -> HWND {
        self.popup.hwnd()
    }

    pub fn application(&self) -> HWND {
        self.application
    }

    pub fn bounds(&self) -> RECT {
        self.bounds
    }

    pub fn is_selected(&self) -> bool {
        self.selected
    }

    pub fn set_selected(&mut self, selected: bool) -> Result<(), String> {
        if self.selected == selected {
            return Ok(());
        }
        self.selected = selected;
        self.selected_state.set(selected);
        self.render_overlay()
    }

    /// Mirrors `ApplicationWindow.SetSessionVisible`: the DWM preview is
    /// toggled before the overlay popup so the compositor never exposes a
    /// stale frame between those operations.
    pub fn set_session_visible(&self, visible: bool) -> Result<(), String> {
        // Always update the overlay, even when DWM rejects the visibility
        // update, and report the native error after the popup attempt.
        let thumbnail_error = self.thumbnail.as_ref().and_then(|thumbnail| {
            thumbnail.set_visible(visible).err().map(|hresult| {
                format!("DwmUpdateThumbnailProperties failed (HRESULT 0x{hresult:08x})")
            })
        });
        self.popup.set_visible(visible);
        if let Some(error) = thumbnail_error {
            return Err(error);
        }
        Ok(())
    }

    pub fn try_activate(&self) -> bool {
        WindowHandle::new(self.application).set_foreground()
    }

    pub fn hit_test(&self, point: POINT) -> bool {
        point.x >= self.bounds.left
            && point.x < self.bounds.right
            && point.y >= self.bounds.top
            && point.y < self.bounds.bottom
    }

    fn render_overlay(&mut self) -> Result<(), String> {
        let selected = self.selected;
        let icon_result = self.window_icon.with_icon(|icon, icon_width, icon_height| {
            render_overlay_with_icon(
                &mut self.layer_updater.borrow_mut(),
                icon,
                icon_width,
                icon_height,
                self.application,
                self.index,
                selected,
            )
        });
        icon_result.unwrap_or_else(|| Err("Window icon is unavailable".to_string()))
    }
}

fn render_overlay_with_icon(
    layer: &mut LayerUpdater,
    icon: windows_sys::Win32::UI::WindowsAndMessaging::HICON,
    icon_width: i32,
    icon_height: i32,
    application: HWND,
    index: usize,
    selected: bool,
) -> Result<(), String> {
    let title = WindowHandle::new(application).get_window_text();
    layer.update(|graphics| {
        graphics.set_overlay_quality()?;
        if selected {
            graphics.fill_rect(
                RectF {
                    X: 0.0,
                    Y: 0.0,
                    Width: graphics.width,
                    Height: graphics.height,
                },
                ARGB_SELECTED,
            )?;
        }

        let title_font = Font::new("Segoe UI", 11.0, FontStyleRegular)
            .map_err(|error| crate::gdi_plus::Error(error.0))?;
        let title_size = graphics.measure_string(&title, &title_font)?;
        let icon_width_f = icon_width as f32;
        let icon_height_f = icon_height as f32;
        let title_width = PAD + icon_width_f + PAD + title_size.Width + PAD;
        let title_height = PAD + icon_height_f.max(title_size.Height) + PAD;
        let title_background = RectF {
            X: 0.0,
            Y: 0.0,
            Width: title_width,
            Height: title_height,
        };
        graphics.fill_rect(title_background, ARGB_BLACK)?;
        let icon_y = (title_height - icon_height_f) / 2.0;
        graphics.draw_icon(icon, PAD as i32, icon_y as i32, icon_width, icon_height)?;
        graphics.draw_string_at(
            &title,
            &title_font,
            PAD + icon_width_f + PAD,
            (title_height - title_size.Height) / 2.0,
            ARGB_WHITE,
        )?;

        let number = (index + 1).to_string();
        let number_font = Font::new("Segoe UI", 72.0, FontStyleBold)
            .map_err(|error| crate::gdi_plus::Error(error.0))?;
        let number_size = graphics.measure_string(&number, &number_font)?;
        let number_background = RectF {
            X: (graphics.width - number_size.Width) / 2.0,
            Y: (graphics.height - number_size.Height) / 2.0,
            Width: number_size.Width,
            Height: number_size.Height,
        };
        graphics.fill_rect(number_background, ARGB_BLACK)?;
        graphics.set_text_rendering_hint()?;
        graphics.draw_string(&number, &number_font, number_background, ARGB_WHITE)
    })
}

fn screen_to_client_rect(owner: HWND, bounds: RECT) -> RECT {
    let mut top_left = POINT {
        x: bounds.left,
        y: bounds.top,
    };
    let mut bottom_right = POINT {
        x: bounds.right,
        y: bounds.bottom,
    };
    unsafe {
        ScreenToClient(owner, &mut top_left);
        ScreenToClient(owner, &mut bottom_right);
    }
    RECT {
        left: top_left.x,
        top: top_left.y,
        right: bottom_right.x,
        bottom: bottom_right.y,
    }
}
