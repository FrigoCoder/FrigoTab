using System.Drawing;

namespace FrigoTab {

    public struct ClientRect {

        public static ClientRect Round (RectangleF rect) => new ClientRect(Rectangle.Round(rect));

        private readonly int _left;
        private readonly int _top;
        private readonly int _right;
        private readonly int _bottom;

        public Point Location => new Point(_left, _top);
        public Size Size => new Size(_right - _left, _bottom - _top);

        public ClientRect (Point location, Size size) {
            _left = location.X;
            _top = location.Y;
            _right = location.X + size.Width;
            _bottom = location.Y + size.Height;
        }

        public ClientRect (Rectangle bounds) {
            _left = bounds.Left;
            _top = bounds.Top;
            _right = bounds.Right;
            _bottom = bounds.Bottom;
        }

        public Rectangle ToRectangle () => Rectangle.FromLTRB(_left, _top, _right, _bottom);

        public RectangleF ToRectangleF () => RectangleF.FromLTRB(_left, _top, _right, _bottom);

        public bool IsEmpty () => (_right - _left <= 0) || (_bottom - _top <= 0);

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

        public bool IsEmpty () => (_right - _left <= 0) || (_bottom - _top <= 0);

    }

    public struct WorkspaceRect {

        public static WorkspaceRect Round (RectangleF rect) => new WorkspaceRect(Rectangle.Round(rect));

        private readonly int _left;
        private readonly int _top;
        private readonly int _right;
        private readonly int _bottom;

        public Point Location => new Point(_left, _top);
        public Size Size => new Size(_right - _left, _bottom - _top);

        public WorkspaceRect (Point location, Size size) {
            _left = location.X;
            _top = location.Y;
            _right = location.X + size.Width;
            _bottom = location.Y + size.Height;
        }

        public WorkspaceRect (Rectangle bounds) {
            _left = bounds.Left;
            _top = bounds.Top;
            _right = bounds.Right;
            _bottom = bounds.Bottom;
        }

        public Rectangle ToRectangle () => Rectangle.FromLTRB(_left, _top, _right, _bottom);

        public RectangleF ToRectangleF () => RectangleF.FromLTRB(_left, _top, _right, _bottom);

        public bool IsEmpty () => (_right - _left <= 0) || (_bottom - _top <= 0);

    }

}
