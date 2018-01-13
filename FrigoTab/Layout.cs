using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Layout {

        public readonly IDictionary<WindowHandle, Rectangle> Bounds = new Dictionary<WindowHandle, Rectangle>();

        public Layout (IList<WindowHandle> windows) {
            foreach( Screen screen in Screen.AllScreens ) {
                LayoutScreen layout = new LayoutScreen(screen, GetWindowsOnScreen(windows, screen));
                layout.Layout();
                foreach( WindowHandle window in layout.Bounds.Keys ) {
                    Bounds[window] = layout.Bounds[window];
                }
            }
        }

        private static List<WindowHandle> GetWindowsOnScreen (IEnumerable<WindowHandle> windows, Screen screen) =>
            windows.Where(window => window.GetScreen().Equals(screen)).ToList();

    }

}
