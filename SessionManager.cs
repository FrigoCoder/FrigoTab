using System;
using System.Windows.Forms;

namespace FrigoTab {

    public class SessionManager : IDisposable {

        private Session _session;

        public void KeyCallBack (KeyHookEventArgs e) {
            if( e.Key == (Keys.Alt | Keys.Tab) ) {
                e.Handled = true;
                if( _session == null ) {
                    BeginSession();
                }
            }
            _session?.HandleKeyEvents(e);
        }

        public void MouseCallBack (MouseHookEventArgs e) {
            _session?.HandleMouseEvents(e);
        }

        public void Dispose () {
            _session?.Dispose();
        }

        private void BeginSession () {
            WindowFinder finder = new WindowFinder();
            if( finder.Windows.Count == 0 ) {
                return;
            }
            _session = new Session(finder);
            _session.FormClosed += EndSession;
        }

        private void EndSession (object sender, EventArgs e) {
            _session.FormClosed -= EndSession;
            _session = null;
        }

    }

}
