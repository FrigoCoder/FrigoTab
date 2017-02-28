using System.Drawing;

namespace FrigoTab {

    public class MouseHookEventArgs {

        public readonly Point Point;
        public readonly bool Click;
        public bool Handled;

        public MouseHookEventArgs (Point point, bool click) {
            Point = point;
            Click = click;
        }

    }

}
