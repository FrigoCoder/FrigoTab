using System;
using System.Drawing;
using System.Windows.Forms;
using FrigoTab.Core;

namespace FrigoTab {

    public class SessionForm : FrigoForm, ISwitcherSessionPort {

        private readonly SwitcherApplication controller;
        private BackgroundWindows backgrounds;
        private ApplicationWindows applications;
        private bool disposed;

        public SessionForm () {
            controller = new SwitcherApplication(this);
        }

        public void HandleKeyEvents (KeyHookEventArgs e) {
            if( disposed || e == null ) {
                return;
            }
            e.Handled = controller.HandleKeyboard(e.Input) == KeyHandling.Consume;
        }

        protected override void Dispose (bool disposing) {
            if( !disposed ) {
                disposed = true;
                controller.Close();
            }
            base.Dispose(disposing);
        }

        protected override void WndProc (ref Message m) {
            WindowMessages wm = (WindowMessages) m.Msg;
            switch( wm ) {
                case WindowMessages.BeginSession:
                    // Keep the private message as a compatibility path for
                    // older callers, while the hook now opens synchronously.
                    HandleKeyEvents(new KeyHookEventArgs(Keys.Alt | Keys.Tab));
                    break;
                case WindowMessages.EndSession:
                case WindowMessages.EndSessionNative:
                case WindowMessages.QueryEndSession:
                    controller.Interrupt();
                    break;
                case WindowMessages.ActivateApp:
                    if( m.WParam == IntPtr.Zero && controller.State == SwitcherState.Visible ) {
                        controller.Interrupt();
                    }
                    break;
                case WindowMessages.DisplayChange:
                case WindowMessages.DpiChanged:
                    controller.Relayout();
                    break;
            }
            base.WndProc(ref m);
        }

        protected override void OnMouseMove (MouseEventArgs e) {
            Point point = e.Location.ClientToScreen(WindowHandle);
            controller.HandleMouseMove(new ScreenPoint(point.X, point.Y));
        }

        protected override void OnMouseDown (MouseEventArgs e) {
            Point point = e.Location.ClientToScreen(WindowHandle);
            controller.HandleMouseClick(new ScreenPoint(point.X, point.Y));
        }

        bool ISwitcherSessionPort.TryOpen (out int candidateCount) => TryOpen(out candidateCount);

        void ISwitcherSessionPort.Select (int index) {
            if( applications == null ) {
                throw new InvalidOperationException("The switcher session is not open.");
            }
            applications.SelectByIndex(index);
        }

        void ISwitcherSessionPort.ClearSelection () => applications?.SelectByIndex(-1);

        int? ISwitcherSessionPort.HitTest (ScreenPoint point) {
            return applications?.HitTest(new Point(point.X, point.Y));
        }

        bool ISwitcherSessionPort.TryActivateSelected () {
            return applications != null && applications.TryActivateSelected();
        }

        void ISwitcherSessionPort.Close () => CloseSessionResources();

        void ISwitcherSessionPort.Relayout () {
            if( applications != null ) {
                // TODO: Rebuild the native thumbnails and overlay bitmaps for
                // the new monitor/DPI topology.  The controller catches this
                // and closes the session rather than showing stale UI.
                throw new NotSupportedException("Live session relayout is not implemented.");
            }
        }

        private bool TryOpen (out int candidateCount) {
            candidateCount = 0;
            if( disposed ) {
                return false;
            }

            CloseSessionResources();

            BackgroundWindows newBackgrounds = null;
            ApplicationWindows newApplications = null;
            try {
                WindowFinder finder = new WindowFinder();
                if( finder.Windows == null || finder.Windows.Count == 0 ) {
                    return false;
                }

                Rectangle bounds;
                if( !TryGetVirtualBounds(out bounds) ) {
                    return false;
                }
                Bounds = bounds;

                // Keep both resource graphs local until every constructor has
                // succeeded.  Only a complete session is published to the
                // controller; any failure releases already-created objects.
                newBackgrounds = new BackgroundWindows(this, finder);
                newApplications = new ApplicationWindows(this, finder);
                if( newApplications.Count == 0 ) {
                    newApplications.Dispose();
                    newBackgrounds.Dispose();
                    return false;
                }

                backgrounds = newBackgrounds;
                applications = newApplications;
                newBackgrounds = null;
                newApplications = null;

                Visible = true;
                applications.Visible.Value = true;
                if( !WindowHandle.SetForeground() ) {
                    CloseSessionResources();
                    return false;
                }

                candidateCount = applications.Count;
                return candidateCount > 0;
            }
            catch {
                try {
                    newApplications?.Dispose();
                }
                catch {
                    // Best-effort cleanup; preserve fail-open behavior.
                }
                try {
                    newBackgrounds?.Dispose();
                }
                catch {
                    // Best-effort cleanup; preserve fail-open behavior.
                }
                CloseSessionResources();
                candidateCount = 0;
                return false;
            }
        }

        private void CloseSessionResources () {
            ApplicationWindows currentApplications = applications;
            BackgroundWindows currentBackgrounds = backgrounds;
            applications = null;
            backgrounds = null;

            try {
                if( currentApplications != null ) {
                    currentApplications.Visible.Value = false;
                }
            }
            catch {
                // Continue to close the owner and every remaining resource.
            }

            try {
                Visible = false;
            }
            catch {
                // The native form may already be in destruction.
            }

            try {
                currentApplications?.Dispose();
            }
            catch {
                // Application teardown is intentionally idempotent/best effort.
            }
            try {
                currentBackgrounds?.Dispose();
            }
            catch {
                // Application teardown is intentionally idempotent/best effort.
            }
        }

        private static bool TryGetVirtualBounds (out Rectangle bounds) {
            bounds = Rectangle.Empty;
            bool found = false;
            foreach( Screen screen in Screen.AllScreens ) {
                if( screen == null || screen.Bounds.Width <= 0 || screen.Bounds.Height <= 0 ) {
                    continue;
                }
                bounds = found ? Rectangle.Union(bounds, screen.Bounds) : screen.Bounds;
                found = true;
            }
            return found && bounds.Width > 0 && bounds.Height > 0;
        }

    }

}
