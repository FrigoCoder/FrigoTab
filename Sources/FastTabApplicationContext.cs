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
        private int counter = 0;

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
            form.Controls.Add(textBox);
            form.FormClosing += Exit;
            form.Visible = true;
        }

        private void Exit(object sender, EventArgs e) {
            notifyIcon.Visible = false;
            ExitThread();
        }

        private bool keyCallBack(IReadOnlyDictionary<Keys, bool> keys, int wParam, LPARAM lParam) {
            bool alt = keys[Keys.LMenu] || keys[Keys.RMenu];
            bool win = keys[Keys.LWin] || keys[Keys.RWin];
            bool tab = keys[Keys.Tab];
            bool altTab = (alt || win) && tab;

            textBox.Text = "wParam=" + wParam + "\r\n";
            textBox.Text += "vkCode=" + lParam.vkCode + "\r\n";
            textBox.Text += "scanCode=" + lParam.scanCode + "\r\n";
            textBox.Text += "flags=" + lParam.flags + "\r\n";
            textBox.Text += "Counter " + counter++;

            return !altTab;
        }

    }

}
