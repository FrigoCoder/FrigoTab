using System;
using System.Windows.Forms;

namespace FrigoTab {

    public class SysTrayIcon : IDisposable {

        public event Action Exit;

        private readonly NotifyIcon notifyIcon;

        public SysTrayIcon () {
            notifyIcon = new NotifyIcon {
                Icon = Program.Icon,
                ContextMenu = new ContextMenu(new[] {new MenuItem("Exit", (sender, args) => { Exit?.Invoke(); })}),
                Visible = true
            };
        }

        public void Dispose () {
            notifyIcon.Dispose();
        }

    }

}
