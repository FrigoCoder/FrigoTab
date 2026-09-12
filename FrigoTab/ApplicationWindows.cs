using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Linq;

namespace FrigoTab {

    public class ApplicationWindows : IDisposable {

        public Property<ApplicationWindow> Selected;
        public Property<bool> Visible;
        private readonly IList<ApplicationWindow> windows = new List<ApplicationWindow>();
        private bool disposed;

        public int Count => windows.Count;

        public ApplicationWindows (FrigoForm owner, WindowFinder finder) {
            Selected.Changed += (oldWindow, newWindow) => {
                if( oldWindow != null ) {
                    oldWindow.SetSelected(false);
                }
                if( newWindow != null ) {
                    newWindow.SetSelected(true);
                }
            };
            Visible.Changed += (oldValue, value) => {
                foreach( ApplicationWindow window in windows ) {
                    window.Visible = value;
                }
            };
            try {
                Layout layout = new Layout(finder.Windows);
                foreach( WindowHandle handle in finder.Windows ) {
                    Rectangle bounds;
                    if( !layout.Bounds.TryGetValue(handle, out bounds) ) {
                        // A window may disappear or move while the snapshot is
                        // being laid out.  Omit that stale candidate instead of
                        // publishing a partially valid session.
                        continue;
                    }
                    try {
                        windows.Add(new ApplicationWindow(owner, handle, windows.Count, bounds));
                    }
                    catch( Exception exception ) when(
                        exception is ArgumentException ||
                        exception is ExternalException ||
                        exception is Win32Exception ) {
                        // A candidate can disappear after layout, or one native
                        // preview can fail independently. Keep the valid windows.
                        Trace.WriteLine("Skipping an unavailable application window: " + exception);
                    }
                }
            }
            catch {
                Dispose();
                throw;
            }
        }

        public void Dispose () {
            if( disposed ) {
                return;
            }

            disposed = true;
            try {
                Visible.Value = false;
            }
            catch {
                // Continue releasing every native window even if one event
                // subscriber has already become invalid.
            }
            try {
                Selected.Value = null;
            }
            catch {
                // Selection rendering is best effort during teardown.
            }

            foreach( ApplicationWindow window in windows ) {
                try {
                    window.Close();
                }
                catch {
                    // A stale HWND must not prevent later candidates from
                    // being disposed.
                }
            }
            windows.Clear();
        }

        public void SelectByIndex (int index) {
            if( disposed ) {
                return;
            }
            Selected.Value = index >= 0 && index < windows.Count ? windows[index] : null;
        }

        public int? HitTest (Point point) {
            if( disposed ) {
                return null;
            }
            for( int index = 0; index < windows.Count; index++ ) {
                if( windows[index].Bounds.Contains(point) ) {
                    return index;
                }
            }
            return null;
        }

        public void SelectByPoint (Point point) {
            int? index = HitTest(point);
            SelectByIndex(index ?? -1);
        }

        public bool TryActivateSelected () {
            if( disposed || Selected.Value == null ) {
                return false;
            }
            return Selected.Value.TryActivate();
        }

    }

}
