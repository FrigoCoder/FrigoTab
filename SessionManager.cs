using System.Windows.Forms;

namespace FrigoTab {

    public class SessionManager : FrigoForm {

        private readonly SessionForm _sessionForm = new SessionForm();

        public SessionManager () {
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;
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
            _sessionForm.Close();
            base.Dispose(disposing);
        }

    }

}
