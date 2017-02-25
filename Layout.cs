using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public static class Layout {

        public static void LayoutWindows (IList<ApplicationWindow> windows) {
            foreach( Screen screen in Screen.AllScreens ) {
                LayoutScreen(screen, GetWindowsOnScreen(windows, screen));
            }
        }

        private static void LayoutScreen (Screen screen, IList<ApplicationWindow> windows) {
            int n = windows.Count;
            if( n == 0 ) {
                return;
            }

            double sqrtn = Math.Sqrt(n);
            int columns = (int) Math.Ceiling(sqrtn);
            int rows = (int) Math.Ceiling((double) n / columns);

            for( int i = 0; i < windows.Count; i++ ) {
                RectangleF cell = GetCellBounds(screen, columns, rows, i % columns, i / columns);
                RectangleF bounds = CenterWithin(windows[i].GetSourceSize(), cell);
                bounds.Offset(screen.WorkingArea.Location);
                windows[i].Bounds = Rectangle.Round(bounds);
            }
        }

        private static List<ApplicationWindow> GetWindowsOnScreen (IList<ApplicationWindow> windows, Screen screen) {
            return windows.Where(window => Screen.FromHandle(window.Application).Equals(screen)).ToList();
        }

        private static RectangleF GetCellBounds (Screen screen, int columns, int rows, int column, int row) {
            SizeF size = new SizeF((float) screen.WorkingArea.Width / columns, (float) screen.WorkingArea.Height / rows);
            PointF location = new PointF(column * size.Width, row * size.Height);
            return new RectangleF(location, size);
        }

        private static RectangleF CenterWithin (Size rectSize, RectangleF bounds) {
            SizeF size = ScaleWithin(rectSize, bounds.Size);
            SizeF margin = bounds.Size - size;
            PointF location = new PointF(bounds.X + margin.Width / 2, bounds.Y + margin.Height / 2);
            return new RectangleF(location, size);
        }

        private static SizeF ScaleWithin (SizeF size, SizeF bounds) {
            size = Scale(size, Math.Min(bounds.Width, size.Width) / size.Width);
            size = Scale(size, Math.Min(bounds.Height, size.Height) / size.Height);
            return size;
        }

        private static SizeF Scale (SizeF size, float scale) {
            return new SizeF(size.Width * scale, size.Height * scale);
        }

    }

}
