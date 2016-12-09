using FastTab.Properties;
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Windows.Forms;
using static FastTab.KeyboardHook;

namespace FastTab {

    class FastTabApplicationContext : ApplicationContext {

        private NotifyIcon notifyIcon;
        private KeyboardHook keyboardHook;
        private Form form;
        private TextBox textBox;

        public FastTabApplicationContext() {
            notifyIcon = new NotifyIcon {
                Icon = Resources.ocean_through_window_frame,
                ContextMenu = new ContextMenu(new MenuItem[] { new MenuItem("Exit", Exit) }),
                Visible = true,
            };

            keyboardHook = new KeyboardHook(keyCallBack);

            textBox = new TextBox();
            textBox.Multiline = true;
            textBox.Dock = DockStyle.Fill;

            form = new Form();
            form.Visible = true;
            form.Controls.Add(textBox);
            form.FormClosing += Exit;
        }

        private void Exit(object sender, EventArgs e) {
            notifyIcon.Visible = false;
            ExitThread();
        }

        private bool keyCallBack(int wParam, LPARAM lParam) {
            bool altTab = (lParam.flags == 32) && (lParam.vkCode == 9);
            textBox.Text = "Flags " + lParam.flags + ", vkCode " + lParam.vkCode + "\r\n";
            return !altTab;
        }

    }

}
