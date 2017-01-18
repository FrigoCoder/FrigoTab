﻿using System;
using System.Collections.Generic;
using System.Windows.Forms;

using FastTab.Properties;

using static FastTab.KeyboardHook;

namespace FastTab {

    internal class FastTabApplicationContext : ApplicationContext {

        private readonly Form form;

        private readonly NotifyIcon notifyIcon;
        private readonly PictureBox pictureBox;

        private readonly TextBox textBox;

        private int counter;
        private KeyboardHook keyboardHook;

        public FastTabApplicationContext () {
            notifyIcon = new NotifyIcon {
                Icon = Resources.ocean_through_window_frame,
                ContextMenu = new ContextMenu(new[] {
                    new MenuItem("Exit", exit)
                }),
                Visible = true
            };

            keyboardHook = new KeyboardHook(keyCallBack);

            textBox = new TextBox {
                Multiline = true,
                Dock = DockStyle.Fill
            };

            pictureBox = new PictureBox {
                Dock = DockStyle.Fill
            };

            form = new Form();

            //            form.Controls.Add(textBox);
            form.Controls.Add(pictureBox);
            form.FormClosing += exit;
            form.Visible = true;
        }

        private void exit (object sender, EventArgs e) {
            form.Visible = false;
            notifyIcon.Visible = false;
            ExitThread();
        }

        private bool keyCallBack (IReadOnlyDictionary<Keys, bool> keys, int wParam, LPARAM lParam) {
            bool alt = keys[Keys.LMenu] || keys[Keys.RMenu];
            bool win = keys[Keys.LWin] || keys[Keys.RWin];
            bool tab = keys[Keys.Tab];
            bool altTab = (alt || win) && tab;

            WindowFinder finder = new WindowFinder();

            string text = "";
            foreach( IntPtr hWnd in finder.getOpenWindows() ) {
                text += finder.getWindowText(hWnd) + "\r\n";
            }
            textBox.Text = text;

            foreach( IntPtr hWnd in finder.getOpenWindows() ) {
                pictureBox.Image = finder.printWindow(hWnd);
            }

            return !altTab;
        }

    }

}