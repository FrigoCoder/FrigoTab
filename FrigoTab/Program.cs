using System;
using System.ComponentModel;
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

            KeyHook keyHook;
            try {
                keyHook = new KeyHook();
            }
            catch( Win32Exception exception ) {
                MessageBox.Show(
                    "FrigoTab could not install its global keyboard hook.\n\n" + exception.Message,
                    "FrigoTab could not start",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Error);
                return;
            }

            using( keyHook ) {
                using( ApplicationContext context = new ApplicationContext() ) {
                    using( SysTrayIcon sysTrayIcon = new SysTrayIcon() ) {
                        using( SessionForm sessionForm = new SessionForm() ) {
                            // A form-less application context keeps the
                            // on-demand overlay hidden at startup. Create its
                            // HWND explicitly so native notifications work
                            // without making it the main form.
                            _ = sessionForm.Handle;
                            sessionForm.FormClosed += (sender, args) => context.ExitThread();
                            keyHook.KeyEvent += sessionForm.HandleKeyEvents;
                            sysTrayIcon.Exit += () => {
                                sessionForm.Close();
                                context.ExitThread();
                            };

                            StartQuitTimer();
                            Application.Run(context);
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
