using System;
using System.Windows.Forms;

namespace FrigoTab {

    internal static class Program {

        [STAThread]
        private static void Main () {
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);

            using( SysTrayIcon sysTrayIcon = new SysTrayIcon() ) {
                sysTrayIcon.Exit += Application.Exit;

                using( SessionManager sessionManager = new SessionManager() ) {
                    using( KeyHook keyHook = new KeyHook() ) {
                        keyHook.KeyEvent += sessionManager.KeyCallBack;

                        Application.Run();
                    }
                }
            }
        }

    }

}
