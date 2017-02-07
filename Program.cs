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

                FastTabApplicationContext applicationContext = new FastTabApplicationContext();
                Application.Run(applicationContext);
            }
        }

    }

}
