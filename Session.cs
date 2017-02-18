using System;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace FrigoTab {

    public class Session : FrigoForm, IDisposable {

        private readonly IList<ApplicationWindow> _applications = new List<ApplicationWindow>();
        private readonly IList<Thumbnail> _backgrounds = new List<Thumbnail>();
        private ApplicationWindow _selectedWindow;

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

        public Session () {
            Bounds = Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);

            WindowFinder finder = new WindowFinder();
            if( finder.Windows.Count == 0 ) {
                Dispose();
                return;
            }

            foreach( WindowHandle window in finder.ToolWindows.Reverse() ) {
                _backgrounds.Add(new Thumbnail(window, Handle, window.GetRestoredWindowRect()));
            }

            Layout layout = new Layout(finder.Windows);
            foreach( WindowHandle window in finder.Windows ) {
                _applications.Add(new ApplicationWindow(this, window, layout.Bounds[window], _applications.Count));
            }

            SelectedWindow = _applications[0];

            Visible = true;
            foreach( ApplicationWindow window in _applications ) {
                window.Visible = true;
            }
            SetForeground();
        }

        public new void Dispose () {
            Visible = false;
            foreach( ApplicationWindow window in _applications ) {
                window.Dispose();
            }
            foreach( Thumbnail thumbnail in _backgrounds ) {
                thumbnail.Dispose();
            }
            Close();
        }

        protected override void OnKeyDown (KeyEventArgs e) {
            base.OnKeyDown(e);
            _applications.Where(window => window.Index == (char) e.KeyCode - '1').ToList().ForEach(window => {
                SelectedWindow = window;
                End();
            });
        }

        protected override void OnMouseMove (MouseEventArgs e) {
            base.OnMouseMove(e);
            SelectedWindow = _applications.FirstOrDefault(window => window.Bounds.Contains(e.Location));
        }

        protected override void OnMouseClick (MouseEventArgs e) {
            base.OnMouseClick(e);
            SelectedWindow = _applications.FirstOrDefault(window => window.Bounds.Contains(e.Location));
            End();
        }

        private void End () {
            if( SelectedWindow != null ) {
                SelectedWindow.WindowHandle.SetForeground();
                Dispose();
            }
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
