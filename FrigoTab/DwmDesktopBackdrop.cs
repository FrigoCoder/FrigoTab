using System;
using System.Diagnostics;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    /// <summary>
    /// Supplies the shell desktop as a live DWM thumbnail.
    ///
    /// The shell already owns a composited desktop surface.  Keeping that
    /// surface in the compositor avoids copying a full virtual-desktop frame
    /// through GDI every time the switcher opens or repaints.
    /// </summary>
    public sealed class DwmDesktopBackdrop : IDisposable {

        private static readonly IDesktopWindowSource NativeDesktopSource = new DesktopWindowSource();

        private readonly WindowHandle destination;
        private readonly Rectangle destinationBounds;
        private readonly IDwmThumbnailApi api;
        private readonly IDesktopWindowSource desktopSource;
        private Thumbnail thumbnail;
        private bool disposed;

        public DwmDesktopBackdrop (WindowHandle destination, Rectangle destinationBounds)
            : this(destination, destinationBounds, new DwmThumbnailApi(), NativeDesktopSource) {
        }

        public DwmDesktopBackdrop (
            WindowHandle destination,
            Rectangle destinationBounds,
            IDwmThumbnailApi api,
            IDesktopWindowSource desktopSource) {
            this.destination = destination;
            this.destinationBounds = destinationBounds;
            this.api = api ?? throw new ArgumentNullException(nameof(api));
            this.desktopSource = desktopSource ?? throw new ArgumentNullException(nameof(desktopSource));
            TryRegister();
        }

        /// <summary>
        /// True when DWM accepted the shell desktop thumbnail.  A false value
        /// is expected on desktops where DWM is unavailable; the owner paints
        /// the solid fallback without delaying session admission.
        /// </summary>
        public bool IsAvailable => thumbnail != null;

        public void DrawFallback (Graphics graphics) {
            if( disposed ) {
                throw new ObjectDisposedException(nameof(DwmDesktopBackdrop));
            }
            graphics.Clear(Color.Black);
        }

        public void Dispose () {
            if( disposed ) {
                return;
            }

            disposed = true;
            Thumbnail currentThumbnail = thumbnail;
            thumbnail = null;
            try {
                currentThumbnail?.Dispose();
            }
            catch( Exception exception ) {
                // Thumbnail.Dispose is already non-throwing for native DWM
                // teardown, but keep this owner cleanup fail-open as well.
                Trace.WriteLine("Desktop backdrop cleanup failed: " + exception);
            }
            GC.SuppressFinalize(this);
        }

        private void TryRegister () {
            Thumbnail candidate = null;
            try {
                if( destination == WindowHandle.Null ||
                    destinationBounds.Width <= 0 || destinationBounds.Height <= 0 ) {
                    return;
                }

                WindowHandle source = desktopSource.Find();
                if( source == WindowHandle.Null ) {
                    return;
                }

                candidate = new Thumbnail(source, destination, api);
                Rect destinationRect = new Rect(destinationBounds).ScreenToClient(destination);
                candidate.SetDestinationRect(destinationRect);
                thumbnail = candidate;
                candidate = null;
            }
            catch( Exception exception ) when(
                exception is ExternalException ||
                exception is ArgumentException ||
                exception is DllNotFoundException ||
                exception is EntryPointNotFoundException ||
                exception is InvalidOperationException ) {
                // DWM is an optional rendering path.  A black owner surface
                // is preferable to delaying or rejecting the keyboard gesture
                // when the shell source or compositor is unavailable.
                Trace.WriteLine("DWM desktop backdrop unavailable: " + exception);
            }
            finally {
                candidate?.Dispose();
            }
        }

    }

    /// <summary>
    /// Injectable shell-window lookup used by the backdrop.  Keeping lookup
    /// outside DWM registration makes source selection testable without a
    /// live Explorer desktop.
    /// </summary>
    public interface IDesktopWindowSource {

        WindowHandle Find ();

    }

    public sealed class DesktopWindowSource : IDesktopWindowSource {

        public WindowHandle Find () => new WindowHandle(GetShellWindow());

        [DllImport("user32.dll", ExactSpelling = true)]
        private static extern IntPtr GetShellWindow ();

    }

}
