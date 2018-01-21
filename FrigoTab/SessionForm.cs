using System;
using System.Drawing;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Forms;

namespace FrigoTab {

    public class SessionForm : FrigoForm {

        private BackgroundWindows backgrounds;
        private ApplicationWindows applications;
        private bool active;

        public void HandleKeyEvents (KeyHookEventArgs e) {
            if( e.Key == (Keys.Alt | Keys.Tab) ) {
                e.Handled = true;
                WindowHandle.PostMessage(WindowMessages.BeginSession, 0, 0);
            }
        }

        protected override void Dispose (bool disposing) {
            EndSession();
            base.Dispose(disposing);
        }

        protected override void WndProc (ref Message m) {
            WindowMessages wm = (WindowMessages) m.Msg;
            if( wm == WindowMessages.BeginSession ) {
                BeginSession();
            }
            base.WndProc(ref m);
        }

        protected override void OnKeyDown (KeyEventArgs e) => KeyPressed(e.KeyCode);
        protected override void OnMouseMove (MouseEventArgs e) => MouseMoved(e.Location.ClientToScreen(WindowHandle));
        protected override void OnMouseDown (MouseEventArgs e) => MouseClicked(e.Location.ClientToScreen(WindowHandle));

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
            applications = new ApplicationWindows(this, finder);

            Visible = true;
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
            Visible = false;

            applications.Dispose();
            backgrounds.Dispose();
        }

        private void KeyPressed (Keys key) {
            if( key == Keys.Escape || key == (Keys.Menu | Keys.F4) ) {
                EndSession();
            }
            if( Keys.D1 <= key && key <= Keys.D9 || Keys.NumPad1 <= key && key <= Keys.NumPad9 ) {
                applications.SelectByIndex((char) key - '1');
                ActivateEndSession();
            }
        }

        private void MouseMoved (Point point) => applications.SelectByPoint(point);

        private void MouseClicked (Point point) {
            applications.SelectByPoint(point);
            ActivateEndSession();
        }

        private void ActivateEndSession () {
            if( applications.Selected.Value == null ) {
                return;
            }
            applications.Selected.Value.Application.SetForeground();
            EndSession();
        }

        [DllImport("user32.dll")]
        private static extern int ChangeDisplaySettingsEx (string lpszDeviceName, IntPtr lpDevMode, IntPtr hwnd, int dwflags, IntPtr lParam);

    }

}
