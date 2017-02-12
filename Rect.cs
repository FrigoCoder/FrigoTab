using System.Drawing;

namespace FrigoTab {

    public struct Rect {

        private readonly int _left;
        private readonly int _top;
        private readonly int _right;
        private readonly int _bottom;

        public Size Size => new Size(_right - _left, _bottom - _top);

        public Rect (Point location, Size size) {
            _left = location.X;
            _top = location.Y;
            _right = location.X + size.Width;
            _bottom = location.Y + size.Height;
        }

        public Rect (Rectangle bounds) {
            _left = bounds.Left;
            _top = bounds.Top;
            _right = bounds.Right;
            _bottom = bounds.Bottom;
        }

    }

}
