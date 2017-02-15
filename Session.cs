using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Session : Form, IDisposable {

        private readonly IList<Thumbnail> _toolwindows = new List<Thumbnail>();
        private readonly IList<Window> _windows = new List<Window>();

        public Session () {
            Bounds = Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);
            FormBorderStyle = FormBorderStyle.None;
            ShowInTaskbar = false;
            TopMost = true;

            WindowFinder finder = new WindowFinder();

            foreach( WindowHandle window in finder.ToolWindows.Reverse() ) {
                _toolwindows.Add(new Thumbnail(window, Handle, window.GetWindowRect()));
            }

            Layout layout = new Layout(finder.Windows);
            foreach( WindowHandle window in finder.Windows ) {
                _windows.Add(new Window(Handle, window, layout.Bounds[window]));
            }

            Visible = true;
        }

        public new void Dispose () {
            Visible = false;
            foreach( Window window in _windows ) {
                window.Dispose();
            }
            foreach( Thumbnail thumbnail in _toolwindows ) {
                thumbnail.Dispose();
            }
            Close();
        }

        protected override void OnKeyDown (KeyEventArgs e) {
            base.OnKeyDown(e);
            Dispose();
        }

    }

}
