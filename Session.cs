using System;
using System.Windows.Forms;

namespace FrigoTab {

    public class Session : Form {

        public Session () {
            TextBox textBox = new TextBox {
                Dock = DockStyle.Fill,
                Multiline = true,
                Text = GetOpenWindowsList()
            };
            Controls.Add(textBox);
            Visible = true;
        }

        private static string GetOpenWindowsList () {
            WindowFinder finder = new WindowFinder();
            string text = "";
            foreach( IntPtr hWnd in finder.GetOpenWindows() ) {
                text += finder.GetWindowText(hWnd) + "\r\n";
            }
            return text;
        }

    }

}
