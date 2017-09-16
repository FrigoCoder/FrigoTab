using System;
using System.Windows.Forms;

namespace FrigoTab {

    public class BackgroundWindow : IDisposable {

        private readonly Thumbnail _thumbnail;
        private bool _disposed;

        public BackgroundWindow (Form owner, WindowHandle window) {
            _thumbnail = new Thumbnail(window, owner.Handle);
            _thumbnail.SetSourceRect(Dwm.GetExtendedFrameBounds(window).ScreenToClient(window));
            _thumbnail.SetDestinationRect(Dwm.GetExtendedFrameBounds(window).ScreenToClient(owner.Handle));
        }

        ~BackgroundWindow () {
            Dispose();
        }

        public void Dispose () {
            if( _disposed ) {
                return;
            }
            _thumbnail.Dispose();
            _disposed = true;
            GC.SuppressFinalize(this);
        }

    }

}
