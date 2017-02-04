using System.Windows.Forms;

namespace FrigoTab {

    public class KeyHookEventArgs {

        public readonly Keys Key;
        public readonly bool Alt;
        public bool Handled;

        public KeyHookEventArgs (Keys key, bool alt) {
            Key = key;
            Alt = alt;
        }

    }

}
