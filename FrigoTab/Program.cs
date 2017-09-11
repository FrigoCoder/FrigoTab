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
                        using( SessionForm sessionForm = new SessionForm() ) {
                            keyHook.KeyEvent += sessionForm.HandleKeyEvents;
                            mouseHook.MouseEvent += sessionForm.HandleMouseEvents;
                            sysTrayIcon.Exit += sessionForm.Close;

                            StartQuitTimer();
                            Application.Run(sessionForm);
                        }
                    }
                }
            }
        }

        [Conditional("DEBUG")]
        private static void StartQuitTimer () {
            Timer timer = new Timer {
                Interval = 10 * 1000
            };
            timer.Tick += (sender, args) => { Application.Exit(); };
            timer.Start();
        }

    }

}
