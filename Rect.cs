using System;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

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

        public static ScreenRect Round (RectangleF rect) => new ScreenRect(Rectangle.Round(rect));

        private readonly int _left;
        private readonly int _top;
        private readonly int _right;
        private readonly int _bottom;

        public Point Location => new Point(_left, _top);
        public Size Size => new Size(_right - _left, _bottom - _top);

        public ScreenRect (Point location, Size size) {
            _left = location.X;
            _top = location.Y;
            _right = location.X + size.Width;
            _bottom = location.Y + size.Height;
        }

        public ScreenRect (Rectangle bounds) {
            _left = bounds.Left;
            _top = bounds.Top;
            _right = bounds.Right;
            _bottom = bounds.Bottom;
        }

        public Rectangle ToRectangle () => Rectangle.FromLTRB(_left, _top, _right, _bottom);

        public RectangleF ToRectangleF () => RectangleF.FromLTRB(_left, _top, _right, _bottom);

        public ClientRect ScreenToClient (WindowHandle window) {
            Point location = Location;
            ScreenToClient(window, ref location);
            return new ClientRect(location, Size);
        }

        [DllImport ("user32.dll")]
        private static extern bool ScreenToClient (IntPtr hWnd, ref Point lpPoint);

    }

}
