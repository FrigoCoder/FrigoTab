using System;
using System.Windows.Forms;

namespace FrigoTab {

    public class SysTrayIcon : IDisposable {

        private readonly NotifyIcon _notifyIcon;

        public event Action Exit;

        public SysTrayIcon () {
            _notifyIcon = new NotifyIcon {
                Icon = Program.Icon,
                ContextMenu = new ContextMenu(new[] {
                    new MenuItem("Exit", (sender, args) => { Exit?.Invoke(); })
                }),
                Visible = true
            };
        }

        public void Dispose () {
            _notifyIcon.Dispose();
        }

    }

}
