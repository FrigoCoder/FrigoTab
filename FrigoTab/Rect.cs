﻿using System.Diagnostics.CodeAnalysis;
using System.Drawing;

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

        public Rect ScreenToClient (WindowHandle window) => new Rect(topLeft.ScreenToClient(window), bottomRight.ScreenToClient(window));

    }

}
