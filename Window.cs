using System;
using System.Drawing;

namespace FrigoTab {

    public class Window : FrigoForm, IDisposable {

        public readonly WindowHandle WindowHandle;
        public readonly int Index;
        private readonly Thumbnail _thumbnail;

        public Window (Session session, WindowHandle windowHandle, Rectangle bounds, int index) {
            Owner = session;
            WindowHandle = windowHandle;
            Bounds = bounds;
            Index = index;
            ExStyle |= WindowExStyles.Layered;
            _thumbnail = new Thumbnail(windowHandle, session.Handle, new Rect(bounds));
        }

        public new void Dispose () {
            _thumbnail.Dispose();
            Close();
        }

    }

}
