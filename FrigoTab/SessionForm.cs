using System;
using System.Diagnostics;
using System.Drawing;
using System.Threading;
using System.Windows.Forms;
using FrigoTab.Core;

namespace FrigoTab {

    public class SessionForm : FrigoForm, ISwitcherSessionPort {

        private readonly SwitcherApplication controller;
        private ShellDesktopSnapshot desktopSnapshot;
        private Rectangle desktopSnapshotBounds;
        private ApplicationWindows applications;
        private bool disposed;
        private bool reportedSessionVisibility;
        private int desktopSnapshotRefreshRunning;

        public event Action<bool> SessionVisibilityChanged;
        public event Action InputResetRequested;

        public bool IsSessionVisible => controller.State == SwitcherState.Visible;

        public SessionForm () {
            controller = new SwitcherApplication(this);

            // Prepare the relatively expensive Explorer render before the
            // keyboard hook is installed. Opening Alt+Tab can then reuse one
            // immutable native frame instead of synchronously asking another
            // process to paint while the gesture is in flight.
            Rectangle bounds;
            if( TryGetVirtualBounds(out bounds) ) {
                ShellDesktopSnapshot initialSnapshot = new ShellDesktopSnapshot(bounds);
                if( initialSnapshot.IsAvailable ) {
                    desktopSnapshot = initialSnapshot;
                    desktopSnapshotBounds = bounds;
                }
                else {
                    initialSnapshot.Dispose();
                }
            }
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
                Volatile.Write(ref disposed, true);
                controller.Close();
                ShellDesktopSnapshot currentSnapshot = desktopSnapshot;
                desktopSnapshot = null;
                desktopSnapshotBounds = Rectangle.Empty;
                currentSnapshot?.Dispose();
            }
            base.Dispose(disposing);
        }

        protected override void OnPaint (PaintEventArgs e) {
            ShellDesktopSnapshot currentSnapshot = desktopSnapshot;
            if( currentSnapshot == null || desktopSnapshotBounds != Bounds ) {
                e.Graphics.Clear(Color.Black);
                return;
            }
            currentSnapshot.Draw(e.Graphics, ClientRectangle);
        }

        protected override void WndProc (ref Message m) {
            WindowMessages wm = (WindowMessages) m.Msg;
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
                    QueueDesktopSnapshotRefresh();
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

                // The shell frame was prepared before hook installation and
                // is refreshed while the switcher is idle. A topology change
                // may briefly use the opaque black fallback until the new
                // frame is ready; opening never blocks on Explorer rendering.
                if( desktopSnapshot == null || desktopSnapshotBounds != bounds ) {
                    QueueDesktopSnapshotRefresh(bounds);
                }

                // Keep the resource graph local until every constructor has
                // succeeded. Only a complete session is published to the
                // controller; any failure releases already-created objects.
                newApplications = new ApplicationWindows(this, finder);
                if( newApplications.Count == 0 ) {
                    newApplications.Dispose();
                    return false;
                }

                applications = newApplications;
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
                CloseSessionResources();
                candidateCount = 0;
                return false;
            }
        }

        private void CloseSessionResources () {
            ApplicationWindows currentApplications = applications;
            applications = null;

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
            if( currentApplications != null && !disposed ) {
                QueueDesktopSnapshotRefresh();
            }
        }

        private void QueueDesktopSnapshotRefresh () {
            Rectangle bounds;
            if( TryGetVirtualBounds(out bounds) ) {
                QueueDesktopSnapshotRefresh(bounds);
            }
        }

        private void QueueDesktopSnapshotRefresh (Rectangle bounds) {
            if( disposed || bounds.Width <= 0 || bounds.Height <= 0 ||
                Interlocked.CompareExchange(ref desktopSnapshotRefreshRunning, 1, 0) != 0 ) {
                return;
            }

            try {
                bool refreshQueued = ThreadPool.QueueUserWorkItem(state => {
                    ShellDesktopSnapshot candidate = null;
                    bool publicationQueued = false;
                    try {
                        candidate = new ShellDesktopSnapshot(bounds);
                        if( !candidate.IsAvailable || Volatile.Read(ref disposed) ) {
                            candidate.Dispose();
                            candidate = null;
                            return;
                        }

                        ShellDesktopSnapshot completedSnapshot = candidate;
                        candidate = null;
                        try {
                            // Keep refresh admission until the UI callback
                            // publishes or rejects this exact frame. Otherwise
                            // a later capture can overtake this callback and an
                            // older frame can overwrite the newer one.
                            BeginInvoke(new Action(() => PublishDesktopSnapshot(completedSnapshot, bounds)));
                            publicationQueued = true;
                        }
                        catch( Exception exception ) {
                            completedSnapshot.Dispose();
                            Trace.WriteLine("Could not publish the refreshed shell desktop snapshot: " + exception);
                        }
                    }
                    catch( Exception exception ) {
                        Trace.WriteLine("Shell desktop snapshot refresh failed: " + exception);
                    }
                    finally {
                        candidate?.Dispose();
                        if( !publicationQueued ) {
                            Interlocked.Exchange(ref desktopSnapshotRefreshRunning, 0);
                        }
                    }
                });
                if( !refreshQueued ) {
                    Interlocked.Exchange(ref desktopSnapshotRefreshRunning, 0);
                    Trace.WriteLine("The shell desktop snapshot refresh was not queued.");
                }
            }
            catch( Exception exception ) {
                Interlocked.Exchange(ref desktopSnapshotRefreshRunning, 0);
                Trace.WriteLine("Could not queue the shell desktop snapshot refresh: " + exception);
            }
        }

        private void PublishDesktopSnapshot (ShellDesktopSnapshot snapshot, Rectangle bounds) {
            // Publication completes the admitted refresh. Release it on the UI
            // thread so a rejected topology can safely enqueue its replacement
            // without racing an older queued publication.
            Interlocked.Exchange(ref desktopSnapshotRefreshRunning, 0);
            if( snapshot == null ) {
                return;
            }
            if( disposed || controller.State == SwitcherState.Visible ) {
                snapshot.Dispose();
                return;
            }

            Rectangle currentBounds;
            if( !TryGetVirtualBounds(out currentBounds) || currentBounds != bounds ) {
                snapshot.Dispose();
                QueueDesktopSnapshotRefresh(currentBounds);
                return;
            }

            ShellDesktopSnapshot previousSnapshot = desktopSnapshot;
            desktopSnapshot = snapshot;
            desktopSnapshotBounds = bounds;
            previousSnapshot?.Dispose();
            Invalidate();
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
