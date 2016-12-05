using FastTab.Properties;
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;
using static FastTab.KeyboardHook;

namespace FastTab {
    class FastTabApplicationContext : ApplicationContext {

        private NotifyIcon notifyIcon;
        private KeyboardHook keyboardHook;

        public FastTabApplicationContext() {
            notifyIcon = new NotifyIcon {
                Icon = Resources.ocean_through_window_frame,
                ContextMenu = new ContextMenu(new MenuItem[] { new MenuItem("Exit", Exit) }),
                Visible = true,
            };
            keyboardHook = new KeyboardHook(keyCallBack);
        }

        private void Exit(object sender, EventArgs e) {
            notifyIcon.Visible = false;
            Application.Exit();
        }

        private bool keyCallBack(int wParam, LPARAM lParam) {
            bool altTab = lParam.flags == 32 && lParam.vkCode == 9;
            //            MessageBox.Show("Flags " + lParam.flags + ", vkCode " + lParam.vkCode);
            //            return altTab;
            return false;
        }

    }

}
