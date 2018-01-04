using System;
using System.Diagnostics.CodeAnalysis;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    [SuppressMessage("ReSharper", "NotAccessedField.Local")]
    public struct Rect {

        public bool IsEmpty => topLeft.X >= bottomRight.X || topLeft.Y >= bottomRight.Y;

        private readonly Point topLeft;
        private readonly Point bottomRight;

        public Rect (Rectangle bounds) {
            topLeft = bounds.Location;
            bottomRight = new Point(bounds.X + bounds.Width, bounds.Y + bounds.Height);
        }

        private Rect (Point topLeft, Point bottomRight) {
            this.topLeft = topLeft;
            this.bottomRight = bottomRight;
        }

        public Rect ScreenToClient (WindowHandle window) {
            Point topleft = topLeft;
            Point bottomright = bottomRight;
            ScreenToClient(window, ref topleft);
            ScreenToClient(window, ref bottomright);
            return new Rect(topleft, bottomright);
        }

        [DllImport("user32.dll")]
        private static extern bool ScreenToClient (IntPtr hWnd, ref Point lpPoint);

    }

}
