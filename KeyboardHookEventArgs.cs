using System.Collections.Generic;
using System.Windows.Forms;

namespace FrigoTab {

    public class KeyboardHookEventArgs {

        public bool Handled;

        public Dictionary<Keys, bool> Keys;

    }

}
