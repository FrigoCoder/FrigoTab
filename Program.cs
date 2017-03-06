using System;
using System.Diagnostics;
using System.Drawing;
using System.Windows.Forms;

namespace FrigoTab {

    public static class Program {

        public static readonly Icon Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);

        [STAThread]
        private static void Main () {
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);

            using( KeyHook keyHook = new KeyHook() ) {
                using( MouseHook mouseHook = new MouseHook() ) {
                    using( SysTrayIcon sysTrayIcon = new SysTrayIcon() ) {
                        using( SessionManager sessionManager = new SessionManager() ) {
                            keyHook.KeyEvent += sessionManager.KeyCallBack;
                            mouseHook.MouseEvent += sessionManager.MouseCallBack;
                            sysTrayIcon.Exit += sessionManager.Dispose;

                            StartQuitTimer();
                            Application.Run(sessionManager);
                        }
                    }
                }
            }
        }

        [Conditional ("DEBUG")]
        private static void StartQuitTimer () {
            Timer timer = new Timer {
                Interval = 10 * 1000
            };
            timer.Tick += (sender, args) => { Application.Exit(); };
            timer.Start();
        }

    }

}
