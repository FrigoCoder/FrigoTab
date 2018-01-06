using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;

namespace FrigoTab {

    public class ApplicationWindows : IDisposable {

        public ApplicationWindow Selected {
            get => selected;
            private set {
                if( selected == value ) {
                    return;
                }
                if( selected != null ) {
                    selected.Selected = false;
                }
                selected = value;
                if( selected != null ) {
                    selected.Selected = true;
                }
            }
        }

        public bool Visible {
            set {
                foreach( ApplicationWindow window in windows ) {
                    window.Visible = value;
                }
            }
        }

        private readonly IList<ApplicationWindow> windows = new List<ApplicationWindow>();
        private ApplicationWindow selected;

        public ApplicationWindows (FrigoForm owner, WindowFinder finder) {
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

        public void SelectByIndex (int index) => Selected = index >= 0 && index < windows.Count ? windows[index] : null;
        public void SelectByPoint (Point point) => Selected = windows.FirstOrDefault(window => window.Bounds.Contains(point));

    }

}
