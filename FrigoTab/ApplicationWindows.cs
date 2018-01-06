using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;

namespace FrigoTab {

    public class ApplicationWindows : IDisposable {

        public bool Visible {
            set {
                foreach( ApplicationWindow window in windows ) {
                    window.Visible = value;
                }
            }
        }

        public readonly Property<ApplicationWindow> Selected = new Property<ApplicationWindow>();
        private readonly IList<ApplicationWindow> windows = new List<ApplicationWindow>();

        public ApplicationWindows (FrigoForm owner, WindowFinder finder) {
            Selected.Changed += (oldWindow, newWindow) => {
                oldWindow?.Selected?.Set(false);
                newWindow?.Selected?.Set(true);
            };

            foreach( WindowHandle window in finder.Windows ) {
                windows.Add(new ApplicationWindow(owner, window, windows.Count));
            }
            Layout.LayoutWindows(windows);
        }

        ~ApplicationWindows () => Dispose();

        public void Dispose () {
            foreach( ApplicationWindow window in windows ) {
                window.Close();
            }
            windows.Clear();
        }

        public void SelectByIndex (int index) => Selected.Set(index >= 0 && index < windows.Count ? windows[index] : null);
        public void SelectByPoint (Point point) => Selected.Set(windows.FirstOrDefault(window => window.Bounds.Contains(point)));

    }

}
