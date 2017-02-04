using System;
using System.Drawing;
using System.Windows.Forms;

namespace FrigoTab {

    internal class FastTabApplicationContext : ApplicationContext {

        private readonly Form _form;
        private readonly NotifyIcon _notifyIcon;
        private readonly TextBox _textBox;
        private readonly KeyHook _keyHook;

        public FastTabApplicationContext () {
            _notifyIcon = new NotifyIcon {
                Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath),
                ContextMenu = new ContextMenu(new[] {
                    new MenuItem("Exit", Exit)
                }),
                Visible = true
            };

            _keyHook = new KeyHook();
            _keyHook.KeyEvent += KeyCallBack;

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

        private void KeyCallBack (object sender, KeyHookEventArgs e) {
            if( e.Alt && (e.Key == Keys.Tab) ) {
                e.Handled = true;

                WindowFinder finder = new WindowFinder();

                string text = "";
                foreach( IntPtr hWnd in finder.GetOpenWindows() ) {
                    text += finder.GetWindowText(hWnd) + "\r\n";
                }
                _textBox.Text = text;
            }
        }

    }

}
