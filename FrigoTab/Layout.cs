using System.Collections.Generic;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Layout {

        public static void LayoutWindows (IList<ApplicationWindow> windows) {
            foreach( Screen screen in Screen.AllScreens ) {
                LayoutScreen layout = new LayoutScreen(screen, GetWindowsOnScreen(windows, screen));
                layout.Layout();
            }
        }

        private static List<ApplicationWindow> GetWindowsOnScreen (IEnumerable<ApplicationWindow> windows, Screen screen) =>
            windows.Where(window => window.Application.GetScreen().Equals(screen)).ToList();

    }

}
