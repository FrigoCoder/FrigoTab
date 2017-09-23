using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Layout {

        public static void LayoutWindows (IList<ApplicationWindow> windows) {
            foreach( Screen screen in Screen.AllScreens ) {
                Layout layout = new Layout(screen, GetWindowsOnScreen(windows, screen));
                layout.LayoutScreen();
            }
        }

        private readonly Screen _screen;
        private readonly IList<ApplicationWindow> _windows;

        private readonly int _n;
        private readonly int _columns;
        private readonly int _rows;

        private Layout (Screen screen, IList<ApplicationWindow> windows) {
            _screen = screen;
            _windows = windows;
            _n = windows.Count;
            if( _n == 0 ) {
                return;
            }
            double sqrtn = Math.Sqrt(_n);
            _columns = (int) Math.Ceiling(sqrtn);
            _rows = (int) Math.Ceiling((double) _n / _columns);
        }

        private void LayoutScreen () {
            for( int i = 0; i < _windows.Count; i++ ) {
                RectangleF cell = GetCellBounds(i % _columns, i / _columns);
                RectangleF bounds = CenterWithin(_windows[i].GetSourceSize(), cell);
                bounds.Offset(_screen.WorkingArea.Location);
                _windows[i].Bounds = Rectangle.Round(bounds);
            }
        }

        private RectangleF GetCellBounds (int column, int row) {
            SizeF size = new SizeF((float) _screen.WorkingArea.Width / _columns,
                (float) _screen.WorkingArea.Height / _rows);
            PointF location = new PointF(column * size.Width, row * size.Height);
            return new RectangleF(location, size);
        }

        private static List<ApplicationWindow> GetWindowsOnScreen (IEnumerable<ApplicationWindow> windows,
            Screen screen) {
            return windows.Where(window => window.Application.GetScreen().Equals(screen)).ToList();
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
