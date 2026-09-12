using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Layout {

        public readonly IDictionary<WindowHandle, Rectangle> Bounds = new Dictionary<WindowHandle, Rectangle>();

        public Layout (IList<WindowHandle> windows) {
            Screen[] screens = Screen.AllScreens;
            List<FrigoTab.Core.LayoutMonitor> monitors = screens.Select(screen => new FrigoTab.Core.LayoutMonitor(
                screen.DeviceName,
                ToCoreRectangle(screen.Bounds),
                ToCoreRectangle(screen.WorkingArea))).ToList();

            var ids = new Dictionary<string, WindowHandle>();
            var candidates = new List<FrigoTab.Core.LayoutWindow>();
            for( int index = 0; index < windows.Count; index++ ) {
                WindowHandle window = windows[index];
                Rectangle restoredRectangle;
                if( !window.TryGetRect(out restoredRectangle) ) {
                    // HWNDs can be destroyed between EnumWindows and layout.
                    // Omit only that candidate; keep the remaining session.
                    continue;
                }

                Screen monitor = Screen.FromRectangle(restoredRectangle);
                if( monitor == null ) {
                    continue;
                }

                string id = index.ToString(System.Globalization.CultureInfo.InvariantCulture);
                ids.Add(id, window);
                candidates.Add(new FrigoTab.Core.LayoutWindow(
                    id,
                    monitor.DeviceName,
                    new FrigoTab.Core.ScreenRectangle(0, 0, restoredRectangle.Width, restoredRectangle.Height)));
            }

            IDictionary<string, FrigoTab.Core.ScreenRectangle> arranged =
                new FrigoTab.Core.GridLayout().Arrange(candidates, monitors);
            foreach( KeyValuePair<string, FrigoTab.Core.ScreenRectangle> item in arranged ) {
                FrigoTab.Core.ScreenRectangle bounds = item.Value;
                Bounds[ids[item.Key]] = new Rectangle(bounds.X, bounds.Y, bounds.Width, bounds.Height);
            }
        }

        private static FrigoTab.Core.ScreenRectangle ToCoreRectangle (Rectangle rectangle) =>
            new FrigoTab.Core.ScreenRectangle(rectangle.X, rectangle.Y, rectangle.Width, rectangle.Height);

    }

}
