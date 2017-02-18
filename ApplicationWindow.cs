using System;
using System.Drawing;
using System.Windows.Forms;

namespace FrigoTab {

    public class ApplicationWindow : FrigoForm, IDisposable {

        public readonly WindowHandle WindowHandle;
        public readonly int Index;
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
            _thumbnail = new Thumbnail(windowHandle, session.Handle, new Rect(bounds));
            _overlay = new Overlay(this);

            Icon = WindowHandle.IconFromSendMessageTimeout();
            Icon = Icon ?? WindowHandle.IconFromGetClassLongPtr();
            Icon = Icon ?? Program.Icon;
        }

        public new void Dispose () {
            _thumbnail.Dispose();
            Close();
        }

        private void SetIcon (IntPtr hwnd, int msg, IntPtr dwData, IntPtr lResult) {
            if( lResult != IntPtr.Zero ) {
                Icon = Icon.FromHandle(lResult);
            }
            _overlay.Draw();
        }

    }

}
