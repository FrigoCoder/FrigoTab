using System;
using System.Drawing;

namespace FrigoTab {

    public class ApplicationWindow : IDisposable {

        public readonly WindowHandle Handle;
        public readonly Rectangle Bounds;
        public readonly int Index;
        public readonly Icon Icon;
        public readonly Overlay Overlay;
        private readonly Thumbnail _thumbnail;
        private bool _selected;

        public bool Selected {
            get { return _selected; }
            set {
                if( _selected == value ) {
                    return;
                }
                _selected = value;
                Overlay.Draw();
            }
        }

        public ApplicationWindow (Session session, WindowHandle handle, Rectangle bounds, int index) {
            Handle = handle;
            Bounds = bounds;
            Index = index;

            Icon = IconManager.IconFromSendMessageTimeout(Handle);
            Icon = Icon ?? IconManager.IconFromGetClassLongPtr(Handle);
            Icon = Icon ?? Program.Icon;

            _thumbnail = new Thumbnail(handle, session.Handle, new Rect(bounds));
            Overlay = new Overlay(session, this);
        }

        public void Dispose () {
            Overlay.Close();
            _thumbnail.Dispose();
        }

    }

}
