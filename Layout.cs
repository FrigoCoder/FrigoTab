using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Layout {

        public readonly Dictionary<WindowHandle, Rectangle> Bounds = new Dictionary<WindowHandle, Rectangle>();

        public Layout (IList<WindowHandle> windows) {
            foreach( Screen screen in Screen.AllScreens ) {
                LayoutScreen(screen, GetWindowsOnScreen(windows, screen));
            }
        }

        private void LayoutScreen (Screen screen, IList<WindowHandle> windows) {
            int n = windows.Count;
            if( n == 0 ) {
                return;
            }

            double sqrtn = Math.Sqrt(n);
            int columns = (int) Math.Ceiling(sqrtn);
            int rows = (int) Math.Ceiling((double) n / columns);

            for( int i = 0; i < windows.Count; i++ ) {
                RectangleF bounds = GetCellBounds(screen, columns, rows, i % columns, i / columns);
                bounds.Offset(screen.WorkingArea.Location);
                Bounds[windows[i]] = Rectangle.Round(bounds);
            }
        }

        private static List<WindowHandle> GetWindowsOnScreen (IList<WindowHandle> windows, Screen screen) {
            return windows.Where(window => Screen.FromHandle(window).Equals(screen)).ToList();
        }

        private static RectangleF GetCellBounds (Screen screen, int columns, int rows, int column, int row) {
            SizeF size = new SizeF((float) screen.WorkingArea.Width / columns, (float) screen.WorkingArea.Height / rows);
            PointF location = new PointF(column * size.Width, row * size.Height);
            return new RectangleF(location, size);
        }

    }

}
