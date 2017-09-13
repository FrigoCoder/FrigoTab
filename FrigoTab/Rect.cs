using System;
using System.Diagnostics.CodeAnalysis;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    [SuppressMessage("ReSharper", "NotAccessedField.Local")]
    public struct Rect {

        private readonly Point TopLeft;
        private readonly Point BottomRight;

        public Rect (Point topLeft, Point bottomRight) {
            TopLeft = topLeft;
            BottomRight = bottomRight;
        }

        public Rect (Point location, Size size) {
            TopLeft = location;
            BottomRight = new Point(location.X + size.Width, location.Y + size.Height);
        }

        public Rect (Rectangle bounds) {
            TopLeft = bounds.Location;
            BottomRight = new Point(bounds.X + bounds.Width, bounds.Y + bounds.Height);
        }

        public Size Size => new Size(BottomRight.X - TopLeft.X, BottomRight.Y - TopLeft.Y);

        public Rect MapWindowPoints (WindowHandle from, WindowHandle to) {
            Rect rect = this;
            MapWindowPoints(from, to, ref rect, 2);
            return rect;
        }

        public Rect ScreenToClient (WindowHandle window) {
            Point topLeft = TopLeft;
            Point bottomRight = BottomRight;
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
