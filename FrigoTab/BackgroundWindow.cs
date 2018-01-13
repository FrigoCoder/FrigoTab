using System;

namespace FrigoTab {

    public class BackgroundWindow : IDisposable {

        private readonly Thumbnail thumbnail;
        private bool disposed;

        public BackgroundWindow (FrigoForm owner, WindowHandle window) {
            thumbnail = new Thumbnail(window, owner.WindowHandle);
            thumbnail.SetSourceRect(window.GetRect().ScreenToClient(window));
            thumbnail.SetDestinationRect(window.GetRect().ScreenToClient(owner.WindowHandle));
        }

        ~BackgroundWindow () => Dispose();

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
