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

        public Session (WindowFinder finder) {
            Bounds = Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);

            foreach( WindowHandle window in finder.ToolWindows.Reverse() ) {
                _backgrounds.Add(new Thumbnail(window, Handle, window.GetWindowRect()));
            }

            foreach( WindowHandle window in finder.Windows ) {
                _applications.Add(new ApplicationWindow(this, window, _applications.Count));
            }

            FrigoTab.Layout.LayoutWindows(_applications);

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
            foreach( ApplicationWindow window in _applications ) {
                window.KeyEventHandler(e);
            }
            if( e.KeyCode == Keys.Escape ) {
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
