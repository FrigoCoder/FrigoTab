using System;
using System.Windows.Forms;

namespace FrigoTab {

    public class SessionManager : IDisposable {

        private Session _session;

        public void KeyCallBack (object sender, KeyHookEventArgs e) {
            if( e.Alt && (e.Key == Keys.Tab) ) {
                e.Handled = true;
                if( _session == null ) {
                    BeginSession();
                }
            }
            _session?.HandleKeyEvents(sender, e);
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
