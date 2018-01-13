using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;

namespace FrigoTab {

    public class ApplicationWindows : IDisposable {

        public Property<ApplicationWindow> Selected;
        public Property<bool> Visible;
        private readonly IList<ApplicationWindow> windows = new List<ApplicationWindow>();

        public ApplicationWindows (FrigoForm owner, WindowFinder finder) {
            Selected.Changed += (oldWindow, newWindow) => {
                oldWindow?.Selected.Set(false);
                newWindow?.Selected.Set(true);
            };
            Visible.Changed += (oldValue, value) => {
                foreach( ApplicationWindow window in windows ) {
                    window.Visible = value;
                }
            };
            Layout layout = new Layout(finder.Windows);
            foreach( WindowHandle handle in finder.Windows ) {
                windows.Add(new ApplicationWindow(owner, handle, windows.Count, layout.Bounds[handle]));
            }
        }

        ~ApplicationWindows () => Dispose();

        public void Dispose () {
            foreach( ApplicationWindow window in windows ) {
                window.Close();
            }
            windows.Clear();
        }

        public void SelectByIndex (int index) => Selected.Value = index >= 0 && index < windows.Count ? windows[index] : null;
        public void SelectByPoint (Point point) => Selected.Value = windows.FirstOrDefault(window => window.Bounds.Contains(point));

    }

}
