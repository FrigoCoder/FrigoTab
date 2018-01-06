using System;
using System.Drawing;

namespace FrigoTab {

    public class WindowIcon {

        public event Action Changed;

        public Icon Icon {
            get => icon;
            private set {
                icon = value;
                Changed?.Invoke();
            }
        }

        private Icon icon;

        public WindowIcon (WindowHandle handle) {
            icon = handle.IconFromGetClassLongPtr() ?? Program.Icon;
            handle.RegisterIconCallback(icon => Icon = icon);
        }

    }

}
