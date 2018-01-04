using System.Drawing;
using System.Linq;
using System.Threading;
using System.Windows.Forms;

namespace FrigoTab {

    public class SessionForm : FrigoForm {

        private BackgroundWindows backgrounds;
        private ScreenForms screenForms;
        private ApplicationWindows applications;
        private bool active;

        public SessionForm () {
            Bounds = GetScreenBounds();
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;
        }

        public void HandleKeyEvents (KeyHookEventArgs e) {
            if( e.Key == (Keys.Alt | Keys.Tab) ) {
                e.Handled = true;
                Handle.PostMessage(WindowMessages.BeginSession, 0, 0);
            }
        }

        public void HandleMouseEvents (MouseHookEventArgs e) {
            if( !active ) {
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
                case WindowMessages.KeyDown:
                case WindowMessages.KeyUp:
                case WindowMessages.SysKeyDown:
                case WindowMessages.SysKeyUp:
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
            if( active ) {
                return;
            }

            WindowFinder finder = new WindowFinder();
            if( finder.Windows.Count == 0 ) {
                return;
            }

            ForceResolutionChange();

            backgrounds = new BackgroundWindows(this, finder);
            screenForms = new ScreenForms(this);
            applications = new ApplicationWindows(this, finder);

            applications.SelectByIndex(0);

            Visible = true;
            screenForms.Visible = true;
            applications.Visible = true;
            Handle.SetForeground();

            active = true;
        }

        private void EndSession () {
            if( !active ) {
                return;
            }
            active = false;

            applications.Visible = false;
            screenForms.Visible = false;
            Visible = false;

            applications.Dispose();
            screenForms.Dispose();
            backgrounds.Dispose();
        }

        private void KeyPressed (Keys key) {
            if( Keys.D1 <= key && key <= Keys.D9 || Keys.NumPad1 <= key && key <= Keys.NumPad9 ) {
                applications.SelectByIndex((char) key - '1');
                ActivateEndSession();
            }
        }

        private void MouseMoved (Point point) {
            applications.SelectByPoint(point);
        }

        private void MouseClicked (Point point) {
            applications.SelectByPoint(point);
            if( screenForms.IsOnAToolBar(point) ) {
                EndSession();
            } else {
                ActivateEndSession();
            }
        }

        private void ActivateEndSession () {
            if( applications.Selected == null ) {
                return;
            }
            active = false;
            applications.Selected.Application.SetForeground();
            active = true;
            EndSession();
        }

        private void DisplayChange () {
            bool preserve = active;
            if( preserve ) {
                EndSession();
            }
            Bounds = GetScreenBounds();
            if( preserve ) {
                BeginSession();
            }
        }

        private static void ForceResolutionChange () {
            WindowHandle foreground = WindowHandle.GetForegroundWindowHandle();
            foreground.PostMessage(WindowMessages.ActivateApp, 0, Thread.CurrentThread.ManagedThreadId);
        }

        private static Rectangle GetScreenBounds () {
            return Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);
        }

    }

}
