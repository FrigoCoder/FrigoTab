using System.Drawing;
using System.Linq;
using System.Threading;
using System.Windows.Forms;

namespace FrigoTab {

    public class SessionForm : FrigoForm {

        private BackgroundWindows _backgrounds;
        private ScreenForms _screenForms;
        private ApplicationWindows _applications;
        private bool _active;

        public SessionForm () {
            Bounds = GetScreenBounds();
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;
        }

        public void HandleKeyEvents (KeyHookEventArgs e) {
            if( e.Key == (Keys.Alt | Keys.Tab) ) {
                e.Handled = true;
                Handle.PostMessage(WindowMessages.BeginSession, 0, 0);
            }
            if( !_active ) {
                return;
            }
            if( e.Key == Keys.Escape || e.Key == (Keys.Alt | Keys.F4) ) {
                e.Handled = true;
                Handle.PostMessage(WindowMessages.EndSession, 0, 0);
            }
            if( Keys.D1 <= e.Key && e.Key <= Keys.D9 || Keys.NumPad1 <= e.Key && e.Key <= Keys.NumPad9 ) {
                e.Handled = true;
                Handle.PostMessage(WindowMessages.KeyPressed, (int) e.Key, 0);
            }
        }

        public void HandleMouseEvents (MouseHookEventArgs e) {
            if( !_active ) {
                return;
            }
            Handle.PostMessage(WindowMessages.MouseMoved, e.Point.X, e.Point.Y);
            if( e.Click ) {
                Handle.PostMessage(WindowMessages.MouseClicked, e.Point.X, e.Point.Y);
            }
        }

        protected override void Dispose (bool disposing) {
            EndSession();
            base.Dispose(disposing);
        }

        protected override void WndProc (ref Message m) {
            WindowMessages wm = (WindowMessages) m.Msg;
            switch( wm ) {
                case WindowMessages.BeginSession:
                    BeginSession();
                    break;
                case WindowMessages.EndSession:
                    EndSession();
                    break;
                case WindowMessages.KeyPressed:
                    KeyPressed((Keys) m.WParam);
                    break;
                case WindowMessages.MouseMoved:
                    MouseMoved(new Point((int) m.WParam, (int) m.LParam));
                    break;
                case WindowMessages.MouseClicked:
                    MouseClicked(new Point((int) m.WParam, (int) m.LParam));
                    break;
                case WindowMessages.DisplayChange:
                    DisplayChange();
                    break;
            }
            base.WndProc(ref m);
        }

        private void BeginSession () {
            if( _active ) {
                return;
            }

            WindowFinder finder = new WindowFinder();
            if( finder.Windows.Count == 0 ) {
                return;
            }

            ForceResolutionChange();

            _backgrounds = new BackgroundWindows(this, finder);
            _screenForms = new ScreenForms(this);
            _applications = new ApplicationWindows(this, finder);

            _applications.SelectByIndex(0);

            Visible = true;
            _screenForms.Visible = true;
            _applications.Visible = true;
            Handle.SetForeground();

            _active = true;
        }

        private void EndSession () {
            if( !_active ) {
                return;
            }
            _active = false;

            _applications.Visible = false;
            _screenForms.Visible = false;
            Visible = false;

            _applications.Dispose();
            _screenForms.Dispose();
            _backgrounds.Dispose();
        }

        private void KeyPressed (Keys key) {
            _applications.SelectByIndex((char) key - '1');
            ActivateEndSession();
        }

        private void MouseMoved (Point point) {
            _applications.SelectByPoint(point);
        }

        private void MouseClicked (Point point) {
            _applications.SelectByPoint(point);
            if( _screenForms.IsOnAToolBar(point) ) {
                EndSession();
            } else {
                ActivateEndSession();
            }
        }

        private void ActivateEndSession () {
            if( _applications.Selected == null ) {
                return;
            }
            _applications.Selected.Application.SetForeground();
            EndSession();
        }

        private void DisplayChange () {
            if( !_active || Bounds == GetScreenBounds() ) {
                return;
            }
            EndSession();
            Bounds = GetScreenBounds();
            BeginSession();
        }

        private static void ForceResolutionChange () {
            WindowHandle foreground = WindowHandle.GetForegroundWindowHandle();
            foreground.SendMessage(WindowMessages.ActivateApp, 0, Thread.CurrentThread.ManagedThreadId);
        }

        private static Rectangle GetScreenBounds () {
            return Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);
        }

    }

}
