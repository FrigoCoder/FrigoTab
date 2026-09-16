use std::collections::HashMap;

use super::layout_monitor::LayoutMonitor;
use super::layout_window::LayoutWindow;
use super::screen_rectangle::ScreenRectangle;

/// The core grid arrangement policy, copied from `GridLayout` without any
/// UI or window-manager dependencies.
#[derive(Clone, Copy, Debug, Default)]
pub struct GridLayout;

impl GridLayout {
    /// Arrange non-empty windows on their corresponding non-empty monitors.
    ///
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
            Self::layout_monitor_windows(monitor, monitor_windows, &mut result);
        }
        result
    }

    fn layout_monitor_windows(
        monitor: &LayoutMonitor,
        windows: &[&LayoutWindow],
        result: &mut HashMap<String, ScreenRectangle>,
    ) {
        // The original implementation performs these calculations in double for
        // the integer grid dimensions.
        let columns = (windows.len() as f64).sqrt().ceil() as usize;
        if columns == 0 {
            return;
        }
        let rows = ((windows.len() as f64) / columns as f64).ceil() as usize;
        if rows == 0 {
            return;
        }

        // The rest of the policy is deliberately f32, matching the original
        // single-precision behavior.
        let x_margin = monitor.bounds.width as f32 * 0.005_f32;
        let y_margin = monitor.bounds.height as f32 * 0.005_f32;
        let cell_width = monitor.working_area.width as f32 / columns as f32;
        let cell_height = monitor.working_area.height as f32 / rows as f32;

        for (index, window) in windows.iter().enumerate() {
            let column = index % columns;
            let row = index / columns;
            let cell_x = column as f32 * cell_width + x_margin;
            let cell_y = row as f32 * cell_height + y_margin;
            let available_width = cell_width - 2.0_f32 * x_margin;
            let available_height = cell_height - 2.0_f32 * y_margin;
            let bounds = Self::fit_inside(
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

        let scale =
            1.0_f32.min((cell_width / source.width as f32).min(cell_height / source.height as f32));
        let width = source.width as f32 * scale;
        let height = source.height as f32 * scale;
        let x = cell_x + (cell_width - width) / 2.0_f32;
        let y = cell_y + (cell_height - height) / 2.0_f32;
        ScreenRectangle::new(
            round_to_even(x),
            round_to_even(y),
            round_to_even(width).max(1),
            round_to_even(height).max(1),
        )
    }
}

/// Round exactly as `Math.Round(double)`'s default ToEven mode.
///
/// The layout arithmetic is single precision, then promoted to double for the
/// same ties-to-even rounding used by the original implementation.
fn round_to_even(value: f32) -> i32 {
    (value as f64).round_ties_even() as i32
}
