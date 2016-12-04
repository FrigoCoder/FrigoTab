using FastTab.Properties;
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace FastTab
{
    class FastTabApplicationContext : ApplicationContext
    {
        private NotifyIcon notifyIcon;

        public FastTabApplicationContext ()
        {
            notifyIcon = new NotifyIcon();
            notifyIcon.Icon = Resources.ocean_through_window_frame;
            notifyIcon.ContextMenu = new ContextMenu(new MenuItem[] { new MenuItem("Exit", Exit) });
            notifyIcon.Visible = true;
        }

        private void Exit(object sender, EventArgs e)
        {
            notifyIcon.Visible = false;
            Application.Exit();
        }
    }

}
