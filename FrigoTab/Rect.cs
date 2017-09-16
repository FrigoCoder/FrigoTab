using System;
using System.Diagnostics.CodeAnalysis;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    [SuppressMessage("ReSharper", "NotAccessedField.Local")]
    public struct Rect {

        private readonly Point _topLeft;
        private readonly Point _bottomRight;

        private Rect (Point topLeft, Point bottomRight) {
            _topLeft = topLeft;
            _bottomRight = bottomRight;
        }

        public Rect (Point location, Size size) {
            _topLeft = location;
            _bottomRight = new Point(location.X + size.Width, location.Y + size.Height);
        }

        public Rect (Rectangle bounds) {
            _topLeft = bounds.Location;
            _bottomRight = new Point(bounds.X + bounds.Width, bounds.Y + bounds.Height);
        }

        public Size Size => new Size(_bottomRight.X - _topLeft.X, _bottomRight.Y - _topLeft.Y);

        public Rect MapWindowPoints (WindowHandle from, WindowHandle to) {
            Rect rect = this;
            MapWindowPoints(from, to, ref rect, 2);
            return rect;
        }

        public Rect ScreenToClient (WindowHandle window) {
            Point topLeft = _topLeft;
            Point bottomRight = _bottomRight;
            ScreenToClient(window, ref topLeft);
            ScreenToClient(window, ref bottomRight);
            return new Rect(topLeft, bottomRight);
        }

        [DllImport("user32.dll")]
        private static extern int MapWindowPoints (IntPtr hWndFrom, IntPtr hWndTo, ref Rect lpRect, int cPoints);

        [DllImport("user32.dll")]
        private static extern bool ScreenToClient (IntPtr hWnd, ref Point lpPoint);

    }

}
