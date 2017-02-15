using System;
using System.Drawing;

namespace FrigoTab {

    public class Window : IDisposable {

        public readonly WindowHandle Handle;
        public readonly Rectangle Bounds;
        private readonly Thumbnail _thumbnail;

        public Window (WindowHandle owner, WindowHandle handle, Rectangle bounds) {
            Handle = handle;
            Bounds = bounds;
            _thumbnail = new Thumbnail(handle, owner, new Rect(bounds));
        }

        public void Dispose () {
            _thumbnail.Dispose();
        }

    }

}
