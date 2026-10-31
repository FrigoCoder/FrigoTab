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

use super::frigo_window::FrigoWindow;
use super::window_handle::WindowHandle;
use super::window_icon::WindowIcon;
use crate::rendering::{DwmThumbnail, Font, LayerUpdater};
use crate::switcher::CloseButtonMode;
use windows_sys::Win32::Graphics::GdiPlus::{FontStyleBold, FontStyleRegular, PointF, RectF};

const PAD: f32 = 8.0;
const ARGB_BLACK: u32 = 0xff00_0000;
const ARGB_WHITE: u32 = 0xffff_ffff;
const ARGB_SELECTED: u32 = 0x8000_00ff;

/// Size and inset of the optional per-tile close button, in physical pixels.
/// These values are shared by drawing and hit-testing.
pub(crate) const CLOSE_BUTTON_SIZE: i32 = 32;
pub(crate) const CLOSE_BUTTON_INSET: i32 = 8;

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
    hovered: bool,
    hovered_state: Rc<Cell<bool>>,
    close_button_mode_state: Rc<Cell<CloseButtonMode>>,
}

impl ApplicationWindow {
    pub fn new(owner: HWND, application: HWND, index: usize, bounds: RECT) -> Result<Self, String> {
        Self::with_close_button_mode(
            owner,
            application,
            index,
            bounds,
            CloseButtonMode::AlwaysVisible,
        )
    }

    pub(crate) fn with_close_button_mode(
        owner: HWND,
        application: HWND,
        index: usize,
        bounds: RECT,
        close_button_mode: CloseButtonMode,
    ) -> Result<Self, String> {
        Self::with_close_button_mode_and_thumbnail_visibility(
            owner,
            application,
            index,
            bounds,
            close_button_mode,
            true,
        )
    }

    pub(crate) fn with_close_button_mode_and_thumbnail_visibility(
        owner: HWND,
        application: HWND,
        index: usize,
        bounds: RECT,
        close_button_mode: CloseButtonMode,
        thumbnail_visible: bool,
    ) -> Result<Self, String> {
        let thumbnail = match DwmThumbnail::register(owner, application) {
            Ok(thumbnail) => {
                let destination = screen_to_client_rect(owner, bounds);
                let configured = if thumbnail_visible {
                    thumbnail.set_destination_rect(destination)
                } else {
                    thumbnail.set_hidden_destination_rect(destination)
                };
                if configured.is_ok() {
                    Some(thumbnail)
                } else {
                    None
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
        let hovered_state = Rc::new(Cell::new(false));
        let callback_hovered = Rc::clone(&hovered_state);
        let close_button_mode_state = Rc::new(Cell::new(close_button_mode));
        let callback_close_button_mode = Rc::clone(&close_button_mode_state);
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
                    callback_hovered.get(),
                    callback_close_button_mode.get(),
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
            hovered: false,
            hovered_state,
            close_button_mode_state,
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
        self.set_interaction_state(selected, self.hovered)
    }

    pub fn is_hovered(&self) -> bool {
        self.hovered
    }

    /// Show or hide the layered title/number overlay. The DWM thumbnail stays
    /// prepared for this short-lived session; hiding the owner suppresses its
    /// output until the registration is dropped.
    pub fn set_session_visible(&self, visible: bool) {
        self.popup.set_visible(visible);
    }

    pub(crate) fn set_thumbnail_visible(&self, visible: bool) -> Result<(), i32> {
        if let Some(thumbnail) = self.thumbnail.as_ref() {
            thumbnail.set_visible(visible)?;
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

    /// Update this tile's close-button mode and redraw its current overlay.
    pub fn set_close_button_mode(&mut self, mode: CloseButtonMode) -> Result<(), String> {
        if self.close_button_mode_state.get() == mode {
            return Ok(());
        }
        self.close_button_mode_state.set(mode);
        self.render_overlay()
    }

    /// Update whether the pointer is over this tile and redraw only when that
    /// state changes. The hover state is kept separate from keyboard selection
    /// so a selected tile does not show a Windows-style close glyph by itself.
    pub fn set_hovered(&mut self, hovered: bool) -> Result<(), String> {
        self.set_interaction_state(self.selected, hovered)
    }

    pub(crate) fn set_interaction_state(
        &mut self,
        selected: bool,
        hovered: bool,
    ) -> Result<(), String> {
        let selection_changed = self.selected != selected;
        let hover_changed = self.hovered != hovered;
        if !selection_changed && !hover_changed {
            return Ok(());
        }
        self.selected = selected;
        self.selected_state.set(selected);
        self.hovered = hovered;
        self.hovered_state.set(hovered);
        if selection_changed
            || (hover_changed && self.close_button_mode_state.get() == CloseButtonMode::HoverOnly)
        {
            self.render_overlay()?;
        }
        Ok(())
    }

    /// Return whether a screen point is inside this tile's close button.
    pub(crate) fn close_button_hit(&self, x: i32, y: i32) -> bool {
        match self.close_button_mode_state.get() {
            CloseButtonMode::Hidden => return false,
            CloseButtonMode::HoverOnly if !self.hovered => return false,
            CloseButtonMode::AlwaysVisible | CloseButtonMode::HoverOnly => {}
        }
        close_button_bounds(self.bounds).is_some_and(|button| {
            x >= button.left && x < button.right && y >= button.top && y < button.bottom
        })
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
                self.hovered_state.get(),
                self.close_button_mode_state.get(),
            )
        });
        icon_result.unwrap_or_else(|| Err("Window icon is unavailable".to_string()))
    }
}

#[allow(clippy::too_many_arguments)] // Keeps the original small rendering helper flat.
fn render_overlay_with_icon(
    layer: &mut LayerUpdater,
    icon: windows_sys::Win32::UI::WindowsAndMessaging::HICON,
    icon_width: i32,
    icon_height: i32,
    application: HWND,
    index: usize,
    selected: bool,
    hovered: bool,
    close_button_mode: CloseButtonMode,
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
            .map_err(|error| crate::rendering::gdi_plus::Error(error.0))?;
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
            .map_err(|error| crate::rendering::gdi_plus::Error(error.0))?;
        let number_size = graphics.measure_string(&number, &number_font)?;
        let number_background = RectF {
            X: (graphics.width - number_size.Width) / 2.0,
            Y: (graphics.height - number_size.Height) / 2.0,
            Width: number_size.Width,
            Height: number_size.Height,
        };
        graphics.fill_rect(number_background, ARGB_BLACK)?;
        graphics.set_text_rendering_hint()?;
        graphics.draw_string(&number, &number_font, number_background, ARGB_WHITE)?;

        let close_button_visible = match close_button_mode {
            CloseButtonMode::AlwaysVisible => true,
            CloseButtonMode::HoverOnly => hovered,
            CloseButtonMode::Hidden => false,
        };
        if close_button_visible
            && let Some(button) = close_button_bounds(RECT {
                left: 0,
                top: 0,
                right: graphics.width.round() as i32,
                bottom: graphics.height.round() as i32,
            })
        {
            render_close_button(
                graphics,
                button,
                close_button_mode == CloseButtonMode::AlwaysVisible,
            )?;
        }
        Ok(())
    })
}

/// Calculate the close-button rectangle in the same coordinate space as the
/// supplied tile bounds. For rendering, pass a rectangle whose origin is
/// `(0, 0)`; for input, pass the tile's screen rectangle.
pub(crate) fn close_button_bounds(bounds: RECT) -> Option<RECT> {
    let width = bounds.right.saturating_sub(bounds.left);
    let height = bounds.bottom.saturating_sub(bounds.top);
    if width < CLOSE_BUTTON_SIZE + CLOSE_BUTTON_INSET
        || height < CLOSE_BUTTON_SIZE + CLOSE_BUTTON_INSET
    {
        return None;
    }

    Some(RECT {
        left: bounds.right - CLOSE_BUTTON_INSET - CLOSE_BUTTON_SIZE,
        top: bounds.top + CLOSE_BUTTON_INSET,
        right: bounds.right - CLOSE_BUTTON_INSET,
        bottom: bounds.top + CLOSE_BUTTON_INSET + CLOSE_BUTTON_SIZE,
    })
}

fn render_close_button(
    graphics: &mut crate::rendering::Graphics,
    button: RECT,
    with_background: bool,
) -> crate::rendering::gdi_plus::Result<()> {
    if with_background {
        let background = RectF {
            X: button.left as f32,
            Y: button.top as f32,
            Width: (button.right - button.left) as f32,
            Height: (button.bottom - button.top) as f32,
        };
        graphics.fill_rect(background, ARGB_BLACK)?;
    }

    let left = button.left as f32 + 8.0;
    let top = button.top as f32 + 8.0;
    let right = button.right as f32 - 8.0;
    let bottom = button.bottom as f32 - 8.0;
    graphics.fill_polygon(
        &[
            PointF {
                X: left,
                Y: top + 3.0,
            },
            PointF {
                X: left + 3.0,
                Y: top,
            },
            PointF {
                X: right,
                Y: bottom - 3.0,
            },
            PointF {
                X: right - 3.0,
                Y: bottom,
            },
        ],
        ARGB_WHITE,
    )?;
    graphics.fill_polygon(
        &[
            PointF {
                X: right - 3.0,
                Y: top,
            },
            PointF {
                X: right,
                Y: top + 3.0,
            },
            PointF {
                X: left + 3.0,
                Y: bottom,
            },
            PointF {
                X: left,
                Y: bottom - 3.0,
            },
        ],
        ARGB_WHITE,
    )
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
