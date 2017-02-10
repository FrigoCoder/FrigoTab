using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Session : Form, IDisposable {

        private readonly IList<Thumbnail> _thumbnails = new List<Thumbnail>();

        public Session () {
            Bounds = Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);


            WindowFinder finder = new WindowFinder();
            foreach( WindowHandle window in finder.Windows ) {
                _thumbnails.Add(new Thumbnail(window, Handle, window.GetWindowRect()));
            }
            Visible = true;
        }

        public new void Dispose () {
            Visible = false;
            foreach( Thumbnail thumbnail in _thumbnails ) {
                thumbnail.Dispose();
            }
            Close();
        }

    }

}
