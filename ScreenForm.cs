using System;
using System.Windows.Forms;

namespace FrigoTab {

    public class ScreenForm : FrigoForm, IDisposable {

        public ScreenForm (Form owner, Screen screen) {
            Owner = owner;
            Bounds = screen.WorkingArea;
        }

        public new void Dispose () {
            Close();
        }

    }

}
