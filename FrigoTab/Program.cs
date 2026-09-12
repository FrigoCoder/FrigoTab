using System;
using System.Diagnostics;
using System.Drawing;
using System.Windows.Forms;

namespace FrigoTab {

    public static class Program {

        public static readonly Icon Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
        private static Timer debugQuitTimer;

        [STAThread]
        private static void Main () {
            SingleInstanceGuard instanceGuard;
            if( !SingleInstanceGuard.TryAcquire(SingleInstanceGuard.ApplicationMutexName, out instanceGuard) ) {
                return;
            }

            using( instanceGuard ) {
                Run();
            }
        }

        private static void Run () {
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);

            using( ApplicationContext context = new ApplicationContext() ) {
                using( SysTrayIcon sysTrayIcon = new SysTrayIcon() ) {
                    using( SessionForm sessionForm = new SessionForm() ) {
                        // A form-less application context keeps the on-demand
                        // overlay hidden. Its HWND must exist before KeyHook so
                        // the native callback can post bounded work to this UI.
                        _ = sessionForm.Handle;

                        KeyHook keyHook;
                        try {
                            keyHook = new KeyHook(sessionForm);
                        }
                        catch( Exception exception ) {
                            MessageBox.Show(
                                "FrigoTab could not install its global keyboard hook.\n\n" + exception.Message,
                                "FrigoTab could not start",
                                MessageBoxButtons.OK,
                                MessageBoxIcon.Error);
                            return;
                        }

                        using( keyHook ) {
                            sessionForm.FormClosed += (sender, args) => context.ExitThread();
                            keyHook.KeyEvent += sessionForm.HandleKeyEvents;
                            sessionForm.SessionVisibilityChanged += keyHook.SetSessionVisible;
                            sessionForm.InputResetRequested += keyHook.ResetInputState;
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
            debugQuitTimer = new Timer {
                Interval = 10 * 1000
            };
            debugQuitTimer.Tick += (sender, args) => {
                debugQuitTimer.Stop();
                debugQuitTimer.Dispose();
                debugQuitTimer = null;
                Application.Exit();
            };
            debugQuitTimer.Start();
        }

    }

}
