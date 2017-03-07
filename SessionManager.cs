using System;
using System.Windows.Forms;

using Microsoft.Win32;

namespace FrigoTab {

    public class SessionManager : FrigoForm, IDisposable {

        private Session _session;

        public SessionManager () {
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;
            SystemEvents.DisplaySettingsChanged += RefreshSession;
        }

        ~SessionManager () {
            SystemEvents.DisplaySettingsChanged -= RefreshSession;
        }

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

        public new void Dispose () {
            _session?.Dispose();
            base.Dispose();
        }

        private void BeginSession () {
            WindowFinder finder = new WindowFinder();
            if( finder.Windows.Count == 0 ) {
                return;
            }
            _session = new Session();
            _session.FormClosed += EndSession;
        }

        private void EndSession (object sender, EventArgs e) {
            _session.FormClosed -= EndSession;
            _session = null;
        }

        private void RefreshSession (object sender, EventArgs e) {
            if( _session == null ) {
                return;
            }
            _session.Dispose();
            BeginSession();
        }

    }

}
