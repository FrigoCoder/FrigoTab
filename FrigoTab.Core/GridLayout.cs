using System;
using System.Collections.Generic;

namespace FrigoTab.Core {

    /// <summary>
    /// Arranges windows in a near-square grid on each monitor.  It mirrors the
    /// legacy layout policy while remaining independent of System.Drawing:
    /// columns are ceil(sqrt(n)), rows are ceil(n / columns), margins are 0.5%
    /// of the monitor bounds, and each source rectangle is centered and scaled
    /// down without changing its aspect ratio.
    /// </summary>
    public sealed class GridLayout {

        public IDictionary<string, ScreenRectangle> Arrange (
            IEnumerable<LayoutWindow> windows,
            IEnumerable<LayoutMonitor> monitors) {
            if( windows == null ) {
                throw new ArgumentNullException(nameof(windows));
            }
            if( monitors == null ) {
                throw new ArgumentNullException(nameof(monitors));
            }

            var windowsByMonitor = new Dictionary<string, IList<LayoutWindow>>(StringComparer.Ordinal);
            foreach( LayoutWindow window in windows ) {
                if( window == null || window.Bounds.IsEmpty ) {
                    continue;
                }

                IList<LayoutWindow> monitorWindows;
                if( !windowsByMonitor.TryGetValue(window.MonitorId, out monitorWindows) ) {
                    monitorWindows = new List<LayoutWindow>();
                    windowsByMonitor.Add(window.MonitorId, monitorWindows);
                }
                monitorWindows.Add(window);
            }

            var result = new Dictionary<string, ScreenRectangle>(StringComparer.Ordinal);
            foreach( LayoutMonitor monitor in monitors ) {
                if( monitor == null || monitor.Bounds.IsEmpty || monitor.WorkingArea.IsEmpty ) {
                    continue;
                }

                IList<LayoutWindow> monitorWindows;
                if( !windowsByMonitor.TryGetValue(monitor.Id, out monitorWindows) || monitorWindows.Count == 0 ) {
                    continue;
                }

                LayoutMonitorWindows(monitor, monitorWindows, result);
            }
            return result;
        }

        private static void LayoutMonitorWindows (
            LayoutMonitor monitor,
            IList<LayoutWindow> windows,
            IDictionary<string, ScreenRectangle> result) {
            int columns = (int) Math.Ceiling(Math.Sqrt(windows.Count));
            if( columns <= 0 ) {
                return;
            }
            int rows = (int) Math.Ceiling((double) windows.Count / columns);
            if( rows <= 0 ) {
                return;
            }

            float xMargin = monitor.Bounds.Width * 0.005f;
            float yMargin = monitor.Bounds.Height * 0.005f;
            float cellWidth = (float) monitor.WorkingArea.Width / columns;
            float cellHeight = (float) monitor.WorkingArea.Height / rows;

            for( int index = 0; index < windows.Count; index++ ) {
                LayoutWindow window = windows[index];
                int column = index % columns;
                int row = index / columns;
                float cellX = column * cellWidth + xMargin;
                float cellY = row * cellHeight + yMargin;
                float availableWidth = cellWidth - 2 * xMargin;
                float availableHeight = cellHeight - 2 * yMargin;

                ScreenRectangle bounds = FitInside(
                    window.Bounds,
                    monitor.WorkingArea.X + cellX,
                    monitor.WorkingArea.Y + cellY,
                    availableWidth,
                    availableHeight);
                result[window.Id] = bounds;
            }
        }

        private static ScreenRectangle FitInside (
            ScreenRectangle source,
            float cellX,
            float cellY,
            float cellWidth,
            float cellHeight) {
            if( source.IsEmpty || cellWidth <= 0 || cellHeight <= 0 ) {
                return new ScreenRectangle();
            }

            float scale = Math.Min(1.0f, Math.Min(cellWidth / source.Width, cellHeight / source.Height));
            float width = source.Width * scale;
            float height = source.Height * scale;
            float x = cellX + (cellWidth - width) / 2;
            float y = cellY + (cellHeight - height) / 2;
            return new ScreenRectangle(
                (int) Math.Round(x),
                (int) Math.Round(y),
                Math.Max(1, (int) Math.Round(width)),
                Math.Max(1, (int) Math.Round(height)));
        }

    }

}
