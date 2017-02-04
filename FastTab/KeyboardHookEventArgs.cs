using System.Collections.Generic;
using System.Windows.Forms;

namespace FastTab.Sources {

    public class KeyboardHookEventArgs {

        public bool Handled;

        public Dictionary<Keys, bool> Keys;

    }

}
