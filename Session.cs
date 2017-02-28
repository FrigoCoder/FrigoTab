using System;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace FrigoTab {

    public class Session : FrigoForm, IDisposable {

        private readonly IList<Thumbnail> _backgrounds = new List<Thumbnail>();
        private readonly IDictionary<Screen, ScreenForm> _screenForms = new Dictionary<Screen, ScreenForm>();
        private readonly IList<ApplicationWindow> _applications = new List<ApplicationWindow>();
        private ApplicationWindow _selectedWindow;

        public Session (WindowFinder finder) {
            Bounds = Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;

            foreach( WindowHandle window in finder.ToolWindows.Reverse() ) {
                _backgrounds.Add(new Thumbnail(window, Handle, window.GetWindowRect()));
            }

            foreach( Screen screen in Screen.AllScreens ) {
                _screenForms[screen] = new ScreenForm(this, screen);
            }

            foreach( WindowHandle window in finder.Windows ) {
                _applications.Add(new ApplicationWindow(this, window, _applications.Count));
            }

            FrigoTab.Layout.LayoutWindows(_applications);

            Visible = true;
            foreach( ScreenForm screenForm in _screenForms.Values ) {
                screenForm.Visible = true;
            }
            foreach( ApplicationWindow window in _applications ) {
                window.Visible = true;
            }
            SetForeground();
        }

        private ApplicationWindow SelectedWindow {
            get { return _selectedWindow; }
            set {
                if( _selectedWindow == value ) {
                    return;
                }
                if( _selectedWindow != null ) {
                    _selectedWindow.Selected = false;
                }
                _selectedWindow = value;
                if( _selectedWindow != null ) {
                    _selectedWindow.Selected = true;
                }
            }
        }

        public new void Dispose () {
            Visible = false;
            foreach( ApplicationWindow window in _applications ) {
                window.Dispose();
            }
            foreach( ScreenForm screenForm in _screenForms.Values ) {
                screenForm.Dispose();
            }
            foreach( Thumbnail thumbnail in _backgrounds ) {
                thumbnail.Dispose();
            }
            Close();
        }

        public void HandleKeyEvents (KeyHookEventArgs e) {
            if( ((Keys.D1 <= e.Key) && (e.Key <= Keys.D9)) || ((Keys.NumPad1 <= e.Key) && (e.Key <= Keys.NumPad9)) ) {
                int index = (char) e.Key - '1';
                if( (index >= 0) && (index < _applications.Count) ) {
                    e.Handled = true;
                    SelectedWindow = _applications[index];
                    End();
                }
            }
            if( e.Key == Keys.Escape ) {
                e.Handled = true;
                Dispose();
            }
            if( e.Key == (Keys.Alt | Keys.F4) ) {
                e.Handled = true;
            }
        }

        public void HandleMouseEvents (MouseHookEventArgs e) {
            SelectedWindow = _applications.FirstOrDefault(window => window.Bounds.Contains(e.Point));
            if( e.Click ) {
                if( IsOnAToolbar(e.Point) ) {
                    Dispose();
                } else {
                    End();
                }
            }
        }

        private bool IsOnAToolbar (Point point) {
            return _screenForms.Values.FirstOrDefault(form => form.Bounds.Contains(point)) == null;
        }

        private void End () {
            if( SelectedWindow == null ) {
                return;
            }
            SelectedWindow.SetForeground();
            Dispose();
        }

        private void SetForeground () {
            Activate();
            int current = GetCurrentThreadId();
            int foreground = GetWindowThreadProcessId(GetForegroundWindow(), IntPtr.Zero);
            if( current != foreground ) {
                File.AppendAllText("log.txt", "Had to AttachThreadInput\n");
                AttachThreadInput(current, foreground, true);
                Activate();
                AttachThreadInput(current, foreground, false);
            }
        }

        [DllImport ("kernel32.dll")]
        private static extern int GetCurrentThreadId ();

        [DllImport ("user32.dll")]
        private static extern int GetWindowThreadProcessId (IntPtr hWnd, IntPtr dwProcessId);

        [DllImport ("user32.dll")]
        private static extern IntPtr GetForegroundWindow ();

        [DllImport ("user32.dll")]
        private static extern bool AttachThreadInput (int idAttach, int idAttachTo, bool fAttach);

    }

}
