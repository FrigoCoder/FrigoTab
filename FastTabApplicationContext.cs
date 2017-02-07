using System;
using System.Windows.Forms;

namespace FrigoTab {

    internal class FastTabApplicationContext : ApplicationContext {

        private readonly Form _form;
        private readonly TextBox _textBox;

        public FastTabApplicationContext () {
            _textBox = new TextBox {
                Multiline = true,
                Dock = DockStyle.Fill
            };

            _form = new Form();
            _form.Controls.Add(_textBox);
            _form.FormClosing += Exit;
            _form.Visible = true;
        }

        public void KeyCallBack (object sender, KeyHookEventArgs e) {
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

        private void Exit (object sender, EventArgs e) {
            _form.Visible = false;
            ExitThread();
        }

    }

}
