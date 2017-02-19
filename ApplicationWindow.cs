using System;
using System.Drawing;
using System.Windows.Forms;

namespace FrigoTab {

    public class ApplicationWindow : FrigoForm, IDisposable {

        public readonly WindowHandle WindowHandle;
        public readonly int Index;
        public readonly Icon WindowIcon;
        private readonly Thumbnail _thumbnail;
        private readonly Overlay _overlay;
        private bool _selected;

        public bool Selected {
            get { return _selected; }
            set {
                if( _selected == value ) {
                    return;
                }
                _selected = value;
                _overlay.Draw();
            }
        }

        public ApplicationWindow (Form session, WindowHandle windowHandle, Rectangle bounds, int index) {
            Owner = session;
            WindowHandle = windowHandle;
            Bounds = bounds;
            Index = index;
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;

            WindowIcon = WindowHandle.IconFromSendMessageTimeout();
            WindowIcon = WindowIcon ?? WindowHandle.IconFromGetClassLongPtr();
            WindowIcon = WindowIcon ?? Program.Icon;

            _thumbnail = new Thumbnail(windowHandle, session.Handle, new Rect(bounds));
            _overlay = new Overlay(this);
        }

        public new void Dispose () {
            _thumbnail.Dispose();
            Close();
        }

    }

}
