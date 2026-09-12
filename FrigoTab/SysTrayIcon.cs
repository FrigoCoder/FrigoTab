using System;
using System.Windows.Forms;

namespace FrigoTab {

    public class SysTrayIcon : IDisposable {

        public event Action Exit;

        private readonly NotifyIcon notifyIcon;
        private readonly ContextMenuStrip contextMenu;

        public SysTrayIcon () {
            contextMenu = new ContextMenuStrip();
            contextMenu.Items.Add(new ToolStripMenuItem("Exit", null, (sender, args) => Exit?.Invoke()));
            notifyIcon = new NotifyIcon {
                Icon = Program.Icon,
                ContextMenuStrip = contextMenu,
                Visible = true
            };
        }

        public void Dispose () {
            notifyIcon.Dispose();
            contextMenu.Dispose();
        }

    }

}
