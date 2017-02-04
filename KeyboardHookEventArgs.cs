using System.Windows.Forms;

namespace FrigoTab {

    public class KeyboardHookEventArgs {

        public readonly Keys Key;
        public readonly bool Alt;
        public bool Handled;

        public KeyboardHookEventArgs (Keys key, bool alt) {
            Key = key;
            Alt = alt;
        }

    }

}
