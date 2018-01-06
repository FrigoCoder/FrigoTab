using System;
using System.Drawing;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Forms;

namespace FrigoTab {

    public class SessionForm : FrigoForm {

        private BackgroundWindows backgrounds;
        private ScreenForms screenForms;
        private ApplicationWindows applications;
        private bool active;

        public SessionForm () {
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;
        }

        public void HandleKeyEvents (KeyHookEventArgs e) {
            if( e.Key == (Keys.Alt | Keys.Tab) ) {
                e.Handled = true;
                WindowHandle.PostMessage(WindowMessages.BeginSession, 0, 0);
            }
        }

        public void HandleMouseEvents (MouseHookEventArgs e) {
            if( !active ) {
                return;
            }
            WindowHandle.PostMessage(WindowMessages.MouseMoved, e.Point.X, e.Point.Y);
            if( e.Click ) {
                e.Handled = true;
                WindowHandle.PostMessage(WindowMessages.MouseClicked, e.Point.X, e.Point.Y);
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

            WindowHandle.GetForegroundWindow().PostMessage(WindowMessages.ActivateApp, 0, Thread.CurrentThread.ManagedThreadId);
            foreach( Screen screen in Screen.AllScreens ) {
                ChangeDisplaySettingsEx(screen.DeviceName, IntPtr.Zero, IntPtr.Zero, 0, IntPtr.Zero);
            }
            Bounds = Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);

            backgrounds = new BackgroundWindows(this, finder);
            screenForms = new ScreenForms(this);
            applications = new ApplicationWindows(this, finder);

            applications.SelectByIndex(0);

            Visible = true;
            screenForms.Visible = true;
            applications.Visible = true;
            WindowHandle.SetForeground();

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

        private void MouseMoved (Point point) => applications.SelectByPoint(point);

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
            applications.Selected.Application.SetForeground();
            EndSession();
        }

        [DllImport("user32.dll")]
        private static extern int ChangeDisplaySettingsEx (string lpszDeviceName, IntPtr lpDevMode, IntPtr hwnd, int dwflags, IntPtr lParam);

    }

}
