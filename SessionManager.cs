using System;
using System.Windows.Forms;

using Microsoft.Win32;

namespace FrigoTab {

    public class SessionManager : FrigoForm {

        private SessionForm _sessionForm;

        public SessionManager () {
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;
            SystemEvents.DisplaySettingsChanged += RefreshSession;
        }

        public void KeyCallBack (KeyHookEventArgs e) {
            if( e.Key == (Keys.Alt | Keys.Tab) ) {
                e.Handled = true;
                if( _sessionForm == null ) {
                    BeginSession();
                }
            }
            _sessionForm?.HandleKeyEvents(e);
        }

        public void MouseCallBack (MouseHookEventArgs e) {
            _sessionForm?.HandleMouseEvents(e);
        }

        protected override void Dispose (bool disposing) {
            SystemEvents.DisplaySettingsChanged -= RefreshSession;
            _sessionForm?.Close();
            base.Dispose(disposing);
        }

        private void BeginSession () {
            WindowFinder finder = new WindowFinder();
            if( finder.Windows.Count == 0 ) {
                return;
            }
            _sessionForm = new SessionForm();
            _sessionForm.FormClosed += EndSessionForm;
        }

        private void EndSessionForm (object sender, EventArgs e) {
            _sessionForm.FormClosed -= EndSessionForm;
            _sessionForm = null;
        }

        private void RefreshSession (object sender, EventArgs e) {
            if( _sessionForm == null ) {
                return;
            }
            _sessionForm.Close();
            BeginSession();
        }

    }

}
