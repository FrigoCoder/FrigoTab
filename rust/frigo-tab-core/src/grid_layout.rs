use std::collections::HashMap;

use crate::{LayoutMonitor, LayoutWindow, ScreenRectangle};

/// Arranges windows in a near-square grid on each monitor.  It mirrors the
/// existing policy: columns are ceil(sqrt(n)), rows are ceil(n / columns),
/// margins are 0.5% of monitor bounds, and each source rectangle is centered
/// and scaled down without changing its aspect ratio.
#[derive(Clone, Copy, Debug, Default)]
pub struct GridLayout;

impl GridLayout {
    pub fn new() -> Self {
        Self
    }

    pub fn arrange(
        &self,
        windows: &[LayoutWindow],
        monitors: &[LayoutMonitor],
    ) -> HashMap<String, ScreenRectangle> {
        let mut windows_by_monitor: HashMap<&str, Vec<&LayoutWindow>> = HashMap::new();
        for window in windows {
            if window.bounds.is_empty() {
                continue;
            }
            windows_by_monitor
                .entry(window.monitor_id.as_str())
                .or_default()
                .push(window);
        }

        let mut result = HashMap::new();
        for monitor in monitors {
            if monitor.bounds.is_empty() || monitor.working_area.is_empty() {
                continue;
            }
            let Some(monitor_windows) = windows_by_monitor.get(monitor.id.as_str()) else {
                continue;
            };
            if monitor_windows.is_empty() {
                continue;
            }
            layout_monitor(monitor, monitor_windows, &mut result);
        }
        result
    }
}

fn layout_monitor(
    monitor: &LayoutMonitor,
    windows: &[&LayoutWindow],
    result: &mut HashMap<String, ScreenRectangle>,
) {
    let columns = (windows.len() as f64).sqrt().ceil() as usize;
    if columns == 0 {
        return;
    }
    let rows = ((windows.len() as f64) / columns as f64).ceil() as usize;
    if rows == 0 {
        return;
    }

    let x_margin = monitor.bounds.width as f32 * 0.005;
    let y_margin = monitor.bounds.height as f32 * 0.005;
    let cell_width = monitor.working_area.width as f32 / columns as f32;
    let cell_height = monitor.working_area.height as f32 / rows as f32;

    for (index, window) in windows.iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        let cell_x = column as f32 * cell_width + x_margin;
        let cell_y = row as f32 * cell_height + y_margin;
        let available_width = cell_width - 2.0 * x_margin;
        let available_height = cell_height - 2.0 * y_margin;
        let bounds = fit_inside(
            window.bounds,
            monitor.working_area.x as f32 + cell_x,
            monitor.working_area.y as f32 + cell_y,
            available_width,
            available_height,
        );
        result.insert(window.id.clone(), bounds);
    }
}

fn fit_inside(
    source: ScreenRectangle,
    cell_x: f32,
    cell_y: f32,
    cell_width: f32,
    cell_height: f32,
) -> ScreenRectangle {
    if source.is_empty() || cell_width <= 0.0 || cell_height <= 0.0 {
        return ScreenRectangle::new(0, 0, 0, 0);
    }

    let scale = (cell_width / source.width as f32)
        .min(cell_height / source.height as f32)
        .min(1.0);
    let width = source.width as f32 * scale;
    let height = source.height as f32 * scale;
    let x = cell_x + (cell_width - width) / 2.0;
    let y = cell_y + (cell_height - height) / 2.0;
    ScreenRectangle::new(
        round_to_even(x),
        round_to_even(y),
        1.max(round_to_even(width)),
        1.max(round_to_even(height)),
    )
}

// System.Math.Round uses midpoint-to-even.  Rust's f32::round uses a
// different midpoint rule, so keep layout parity at exact half pixels.
fn round_to_even(value: f32) -> i32 {
    let lower = value.floor();
    let fraction = value - lower;
    let rounded = if fraction < 0.5 {
        lower
    } else if fraction > 0.5 {
        lower + 1.0
    } else if (lower as i64) % 2 == 0 {
        lower
    } else {
        lower + 1.0
    };
    rounded as i32
}
