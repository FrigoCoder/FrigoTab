using System;
using System.Windows.Forms;

namespace FrigoTab {

    public class BackgroundWindow : IDisposable {

        private readonly Thumbnail thumbnail;
        private bool disposed;

        public BackgroundWindow (Form owner, WindowHandle window) {
            thumbnail = new Thumbnail(window, owner.Handle);
            thumbnail.SetSourceRect(Dwm.GetExtendedFrameBounds(window).ScreenToClient(window));
            thumbnail.SetDestinationRect(Dwm.GetExtendedFrameBounds(window).ScreenToClient(owner.Handle));
        }

        ~BackgroundWindow () {
            Dispose();
        }

        public void Dispose () {
            if( disposed ) {
                return;
            }
            thumbnail.Dispose();
            disposed = true;
            GC.SuppressFinalize(this);
        }

    }

}
