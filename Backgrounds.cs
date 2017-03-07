using System;
using System.Collections.Generic;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Backgrounds : IDisposable {

        private readonly Form _owner;
        private readonly IList<Thumbnail> _backgrounds = new List<Thumbnail>();

        public Backgrounds (Form owner) {
            _owner = owner;
        }

        ~Backgrounds () {
            Dispose();
        }

        public void Populate () {
            WindowFinder finder = new WindowFinder();
            foreach( WindowHandle window in finder.ToolWindows.Reverse() ) {
                _backgrounds.Add(new Thumbnail(window, _owner.Handle, window.GetWindowRect()));
            }
        }

        public void Dispose () {
            foreach( Thumbnail thumbnail in _backgrounds ) {
                thumbnail.Dispose();
            }
            _backgrounds.Clear();
        }

        public void Refresh () {
            Dispose();
            Populate();
        }

    }

}
