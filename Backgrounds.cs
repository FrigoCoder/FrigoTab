using System;
using System.Collections.Generic;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Backgrounds : IDisposable {

        private readonly Form _owner;
        private readonly IList<BackgroundWindow> _backgrounds = new List<BackgroundWindow>();

        public Backgrounds (Form owner) {
            _owner = owner;
        }

        ~Backgrounds () {
            Dispose();
        }

        public void Populate () {
            WindowFinder finder = new WindowFinder();
            foreach( WindowHandle window in finder.ToolWindows.Reverse() ) {
                _backgrounds.Add(new BackgroundWindow(_owner, window));
            }
        }

        public void Dispose () {
            foreach( BackgroundWindow window in _backgrounds ) {
                window.Dispose();
            }
            _backgrounds.Clear();
        }

    }

}
