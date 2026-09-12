using System;
using System.Diagnostics;
using System.Drawing;
using System.Windows.Forms;
using FrigoTab.Core;

namespace FrigoTab {

    public class SessionForm : FrigoForm, ISwitcherSessionPort {

        private readonly SwitcherApplication controller;
        private DwmGlassBackdrop glassBackdrop;
        private DwmDesktopBackdrop desktopBackdrop;
        private ApplicationWindows applications;
        private bool disposed;
        private bool reportedSessionVisibility;

        public event Action<bool> SessionVisibilityChanged;
        public event Action InputResetRequested;

        public bool IsSessionVisible => controller.State == SwitcherState.Visible;

        public SessionForm () {
            // DWM owns the live glass pixels. Avoid allocating a second full
            // redirected surface for this virtual-desktop-sized owner.
            ExStyle |= WindowExStyles.NoRedirectionBitmap;
            controller = new SwitcherApplication(this);
        }

        public void HandleKeyEvents (KeyHookEventArgs e) {
            if( disposed || e == null ) {
                return;
            }
            e.Handled = controller.HandleKeyboard(e.Input) == KeyHandling.Consume;
            NotifySessionVisibility();
        }

        protected override void Dispose (bool disposing) {
            if( !disposed ) {
                disposed = true;
                controller.Close();
            }
            base.Dispose(disposing);
        }

        protected override void OnPaint (PaintEventArgs e) {
            DwmGlassBackdrop currentGlass = glassBackdrop;
            if( currentGlass != null && currentGlass.IsAvailable ) {
                // Alpha-zero/no-paint client pixels let the DWM frame expose
                // the live desktop. DWM application thumbnails are composed
                // into this same non-layered owner at full opacity.
                return;
            }

            DwmDesktopBackdrop currentBackdrop = desktopBackdrop;
            if( currentBackdrop == null || !currentBackdrop.IsAvailable ) {
                e.Graphics.Clear(Color.Black);
                return;
            }
            // The DWM thumbnail is composited directly into this owner HWND.
            // Painting a solid base keeps the fallback deterministic without
            // copying or scaling a full virtual-desktop bitmap on the UI
            // thread.
            currentBackdrop.DrawFallback(e.Graphics);
        }

        protected override void WndProc (ref Message m) {
            WindowMessages wm = (WindowMessages) m.Msg;
            if( wm == WindowMessages.EraseBackground &&
                glassBackdrop != null && glassBackdrop.IsAvailable ) {
                // Prevent WinForms/GDI from replacing the transparent glass
                // client with its default opaque background.
                m.Result = new IntPtr(1);
                return;
            }
            switch( wm ) {
                case WindowMessages.BeginSession:
                    // Keep the private message as a compatibility path for
                    // older callers; the native hook now posts equivalent work
                    // to the UI message queue.
                    HandleKeyEvents(new KeyHookEventArgs(Keys.Alt | Keys.Tab));
                    break;
                case WindowMessages.EndSession:
                case WindowMessages.EndSessionNative:
                case WindowMessages.QueryEndSession:
                    controller.Interrupt();
                    InputResetRequested?.Invoke();
                    NotifySessionVisibility();
                    break;
                case WindowMessages.ActivateApp:
                    if( m.WParam == IntPtr.Zero && controller.State == SwitcherState.Visible ) {
                        controller.Interrupt();
                        InputResetRequested?.Invoke();
                        NotifySessionVisibility();
                    }
                    break;
                case WindowMessages.DisplayChange:
                case WindowMessages.DpiChanged:
                case WindowMessages.DwmCompositionChanged:
                    bool wasVisible = controller.State == SwitcherState.Visible;
                    controller.Relayout();
                    if( wasVisible && controller.State != SwitcherState.Visible ) {
                        // Relayout currently closes a live session.  Reset the
                        // hook's modifier/suppression state as well because a
                        // desktop or compositor transition may have lost the
                        // matching native key-up event.
                        InputResetRequested?.Invoke();
                    }
                    NotifySessionVisibility();
                    break;
            }
            base.WndProc(ref m);
        }

        protected override void OnMouseMove (MouseEventArgs e) {
            Point point = e.Location.ClientToScreen(WindowHandle);
            controller.HandleMouseMove(new ScreenPoint(point.X, point.Y));
            NotifySessionVisibility();
        }

        protected override void OnMouseDown (MouseEventArgs e) {
            Point point = e.Location.ClientToScreen(WindowHandle);
            controller.HandleMouseClick(new ScreenPoint(point.X, point.Y));
            NotifySessionVisibility();
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

            DwmGlassBackdrop newGlassBackdrop = null;
            DwmDesktopBackdrop newDesktopBackdrop = null;
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

                // The non-layered DWM glass owner exposes the actual live
                // desktop without a GDI readback or per-window reconstruction.
                // A shell thumbnail remains a cheap fallback when frame
                // extension is unavailable.
                newGlassBackdrop = new DwmGlassBackdrop(WindowHandle);
                if( !newGlassBackdrop.IsAvailable ) {
                    newDesktopBackdrop = new DwmDesktopBackdrop(WindowHandle, bounds);
                }

                // Keep the resource graph local until every constructor has
                // succeeded. Only a complete session is published to the
                // controller; any failure releases already-created objects.
                newApplications = new ApplicationWindows(this, finder);
                if( newApplications.Count == 0 ) {
                    newApplications.Dispose();
                    newDesktopBackdrop?.Dispose();
                    newGlassBackdrop.Dispose();
                    return false;
                }

                glassBackdrop = newGlassBackdrop;
                desktopBackdrop = newDesktopBackdrop;
                applications = newApplications;
                newGlassBackdrop = null;
                newDesktopBackdrop = null;
                newApplications = null;

                Visible = true;
                applications.Visible.Value = true;
                if( !WindowHandle.SetForeground() ) {
                    // Windows may legitimately deny SetForegroundWindow even
                    // though this process owns the physical Alt+Tab gesture.
                    // The overlay and hook admission have already succeeded;
                    // treating focus acquisition as fatal would replay the
                    // gesture and expose the native switcher intermittently.
                    Trace.WriteLine("The switcher opened, but Windows denied foreground activation.");
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
                    newDesktopBackdrop?.Dispose();
                }
                catch {
                    // Best-effort cleanup; preserve fail-open behavior.
                }
                try {
                    newGlassBackdrop?.Dispose();
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
            DwmGlassBackdrop currentGlassBackdrop = glassBackdrop;
            DwmDesktopBackdrop currentDesktopBackdrop = desktopBackdrop;
            applications = null;
            glassBackdrop = null;
            desktopBackdrop = null;

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
                currentDesktopBackdrop?.Dispose();
            }
            catch {
                // Application teardown is intentionally idempotent/best effort.
            }
            try {
                currentGlassBackdrop?.Dispose();
            }
            catch {
                // Restore ordinary owner margins even if other teardown fails.
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

        private void NotifySessionVisibility () {
            bool visible = controller.State == SwitcherState.Visible;
            if( visible == reportedSessionVisibility ) {
                return;
            }
            reportedSessionVisibility = visible;
            SessionVisibilityChanged?.Invoke(visible);
        }

    }

}
