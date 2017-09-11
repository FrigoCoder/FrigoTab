using System;
using System.Diagnostics.CodeAnalysis;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    [SuppressMessage("ReSharper", "NotAccessedField.Local")]
    public struct ClientRect {

        private readonly int _left;
        private readonly int _top;
        private readonly int _right;
        private readonly int _bottom;

        public ClientRect (Point location, Size size) {
            _left = location.X;
            _top = location.Y;
            _right = location.X + size.Width;
            _bottom = location.Y + size.Height;
        }

    }

    public struct ScreenRect {

        private readonly int _left;
        private readonly int _top;
        private readonly int _right;
        private readonly int _bottom;

        public Size Size => new Size(_right - _left, _bottom - _top);
        private Point Location => new Point(_left, _top);

        public ScreenRect (Rectangle bounds) {
            _left = bounds.Left;
            _top = bounds.Top;
            _right = bounds.Right;
            _bottom = bounds.Bottom;
        }

        public ClientRect ScreenToClient (WindowHandle window) {
            Point location = Location;
            ScreenToClient(window, ref location);
            return new ClientRect(location, Size);
        }

        [DllImport("user32.dll")]
        private static extern bool ScreenToClient (IntPtr hWnd, ref Point lpPoint);

    }

}
