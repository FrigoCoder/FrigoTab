using System;
using System.Diagnostics.CodeAnalysis;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    [SuppressMessage("ReSharper", "NotAccessedField.Local")]
    public struct Rect {

        public bool IsEmpty => _topLeft.X >= _bottomRight.X || _topLeft.Y >= _bottomRight.Y;

        private readonly Point _topLeft;
        private readonly Point _bottomRight;

        public Rect (Rectangle bounds) {
            _topLeft = bounds.Location;
            _bottomRight = new Point(bounds.X + bounds.Width, bounds.Y + bounds.Height);
        }

        private Rect (Point topLeft, Point bottomRight) {
            _topLeft = topLeft;
            _bottomRight = bottomRight;
        }

        public Rect ScreenToClient (WindowHandle window) {
            Point topLeft = _topLeft;
            Point bottomRight = _bottomRight;
            ScreenToClient(window, ref topLeft);
            ScreenToClient(window, ref bottomRight);
            return new Rect(topLeft, bottomRight);
        }

        [DllImport("user32.dll")]
        private static extern bool ScreenToClient (IntPtr hWnd, ref Point lpPoint);

    }

}
