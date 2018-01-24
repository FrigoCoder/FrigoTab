using System.Collections.Generic;
using System.Drawing;
using System.Linq;

namespace FrigoTab {

    public class ApplicationWindows : FrigoForm {

        public Property<ApplicationWindow> Selected;
        public readonly LayerUpdater LayerUpdater;
        private readonly IList<ApplicationWindow> windows = new List<ApplicationWindow>();

        public ApplicationWindows (FrigoForm owner, WindowFinder finder) {
            Owner = owner;
            Bounds = owner.Bounds;
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;
            Selected.Changed += (oldWindow, newWindow) => {
                oldWindow?.Selected.Set(false);
                newWindow?.Selected.Set(true);
            };
            LayerUpdater = new LayerUpdater(this);
            Layout layout = new Layout(finder.Windows);
            foreach( WindowHandle handle in finder.Windows ) {
                windows.Add(new ApplicationWindow(owner, handle, windows.Count, layout.Bounds[handle], LayerUpdater));
            }
        }

        public void SelectByIndex (int index) => Selected.Value = index >= 0 && index < windows.Count ? windows[index] : null;

        public void SelectByPoint (Point point) =>
            Selected.Value = windows.FirstOrDefault(window => window.Bounds.Contains(point.ScreenToClient(WindowHandle)));

        protected override void Dispose (bool disposing) {
            foreach( ApplicationWindow window in windows ) {
                window.Dispose();
            }
            windows.Clear();
            LayerUpdater.Dispose();
            base.Dispose(disposing);
        }

    }

}
