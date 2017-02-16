using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace FrigoTab {

    public class Session : FrigoForm {

        private readonly IList<Thumbnail> _toolwindows = new List<Thumbnail>();
        private readonly IList<Window> _windows = new List<Window>();

        public Session () {
            Bounds = Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);
            FormBorderStyle = FormBorderStyle.None;
            ShowInTaskbar = false;
            TopMost = true;

            WindowFinder finder = new WindowFinder();

            foreach( WindowHandle window in finder.ToolWindows.Reverse() ) {
                _toolwindows.Add(new Thumbnail(window, Handle, window.GetRestoredWindowRect()));
            }

            Layout layout = new Layout(finder.Windows);
            foreach( WindowHandle window in finder.Windows ) {
                _windows.Add(new Window(this, window, layout.Bounds[window], _windows.Count));
            }

            Visible = true;
            SetForeground();
        }

        public override void Dispose () {
            Visible = false;
            foreach( Window window in _windows ) {
                window.Dispose();
            }
            foreach( Thumbnail thumbnail in _toolwindows ) {
                thumbnail.Dispose();
            }
            base.Dispose();
        }

        protected override void OnKeyDown (KeyEventArgs e) {
            base.OnKeyDown(e);
            foreach( Window window in _windows ) {
                if( (e.KeyCode - Keys.D1 == window.Index) || (e.KeyCode - Keys.NumPad1 == window.Index) ) {
                    window.WindowHandle.SetForeground();
                    Dispose();
                }
            }
        }

        protected override void OnMouseClick (MouseEventArgs e) {
            base.OnMouseClick(e);
            foreach( Window window in _windows ) {
                if( window.Bounds.Contains(e.Location) ) {
                    window.WindowHandle.SetForeground();
                    Dispose();
                }
            }
        }

        private void SetForeground () {
            Activate();
            int current = GetCurrentThreadId();
            int foreground = GetWindowThreadProcessId(GetForegroundWindow(), IntPtr.Zero);
            if( current != foreground ) {
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
