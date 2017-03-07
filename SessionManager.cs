using System;
using System.Windows.Forms;

using Microsoft.Win32;

namespace FrigoTab {

    public class SessionManager : FrigoForm {

        private readonly SessionForm _sessionForm = new SessionForm();

        public SessionManager () {
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;
            SystemEvents.DisplaySettingsChanged += RefreshSession;
        }

        public void KeyCallBack (KeyHookEventArgs e) {
            if( e.Key == (Keys.Alt | Keys.Tab) ) {
                e.Handled = true;
                _sessionForm.BeginSession();
            }
            _sessionForm.HandleKeyEvents(e);
        }

        public void MouseCallBack (MouseHookEventArgs e) {
            _sessionForm.HandleMouseEvents(e);
        }

        protected override void Dispose (bool disposing) {
            SystemEvents.DisplaySettingsChanged -= RefreshSession;
            _sessionForm.Close();
            base.Dispose(disposing);
        }

        private void RefreshSession (object sender, EventArgs e) {
            _sessionForm.RefreshSession();
        }

    }

}
