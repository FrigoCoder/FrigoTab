using System;
using System.Windows.Forms;

namespace FrigoTab {

    internal class SessionManager : ApplicationContext {

        private Session _session;

        public void KeyCallBack (object sender, KeyHookEventArgs e) {
            if( e.Alt && (e.Key == Keys.Tab) ) {
                e.Handled = true;
                if( _session == null ) {
                    BeginSession();
                }
            }
        }

        public void BeginSession () {
            _session = new Session();
            _session.FormClosed += EndSession;
        }

        public void EndSession (object sender, EventArgs e) {
            _session.FormClosed -= EndSession;
            _session = null;
        }

    }

}
