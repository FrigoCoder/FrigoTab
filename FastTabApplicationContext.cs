using System;
using System.Collections.Generic;
using System.Windows.Forms;

using FastTab.Properties;

using static FastTab.KeyboardHook;

namespace FastTab {

    internal class FastTabApplicationContext : ApplicationContext {

        private readonly Form _form;
        private readonly NotifyIcon _notifyIcon;
        private readonly TextBox _textBox;
        private int _counter;
        private KeyboardHook _keyboardHook;

        public FastTabApplicationContext () {
            _notifyIcon = new NotifyIcon {
                Icon = Resources.ocean_through_window_frame,
                ContextMenu = new ContextMenu(new[] {
                    new MenuItem("Exit", Exit)
                }),
                Visible = true
            };

            _keyboardHook = new KeyboardHook(keyCallBack);

            _textBox = new TextBox {
                Multiline = true,
                Dock = DockStyle.Fill
            };

            _form = new Form();
            _form.Controls.Add(_textBox);
            _form.FormClosing += Exit;
            _form.Visible = true;
        }

        private void Exit (object sender, EventArgs e) {
            _form.Visible = false;
            _notifyIcon.Visible = false;
            ExitThread();
        }

        private bool keyCallBack (IDictionary<Keys, bool> keys, int wParam, Lparam lParam) {
            bool alt = keys[Keys.LMenu] || keys[Keys.RMenu];
            bool win = keys[Keys.LWin] || keys[Keys.RWin];
            bool tab = keys[Keys.Tab];
            bool altTab = (alt || win) && tab;

            WindowFinder finder = new WindowFinder();

            string text = "";
            foreach( IntPtr hWnd in finder.GetOpenWindows() ) {
                text += finder.GetWindowText(hWnd) + "\r\n";
            }
            _textBox.Text = text;

            return !altTab;
        }

    }

}
