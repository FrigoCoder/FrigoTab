using System;
using System.Collections.Generic;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Backgrounds : IDisposable {

        private readonly IList<BackgroundWindow> _backgrounds = new List<BackgroundWindow>();

        public Backgrounds (Form owner, WindowFinder finder) {
            foreach( WindowHandle window in finder.ToolWindows.Reverse() ) {
                _backgrounds.Add(new BackgroundWindow(owner, window));
            }
        }

        ~Backgrounds () {
            Dispose();
        }

        public void Dispose () {
            foreach( BackgroundWindow window in _backgrounds ) {
                window.Dispose();
            }
            _backgrounds.Clear();
        }

    }

}
