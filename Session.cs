using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Session : Form, IDisposable {

        private readonly IList<Thumbnail> _toolwindows = new List<Thumbnail>();
        private readonly IList<Thumbnail> _thumbnails = new List<Thumbnail>();

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
                Rectangle bounds = layout.Bounds[window];
                _thumbnails.Add(new Thumbnail(window, Handle, new Rect(bounds)));
            }

            Visible = true;
        }

        public new void Dispose () {
            Visible = false;
            foreach( Thumbnail thumbnail in _toolwindows ) {
                thumbnail.Dispose();
            }
            foreach( Thumbnail thumbnail in _thumbnails ) {
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
