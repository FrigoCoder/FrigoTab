using System;
using System.Diagnostics;
using System.Runtime.InteropServices;

namespace FrigoTab {

    /// <summary>
    /// Extends the DWM frame across the no-redirection owner so the compositor
    /// supplies the live desktop behind FrigoTab. No desktop pixels are copied
    /// into this process and application thumbnails keep their full opacity.
    /// </summary>
    public sealed class DwmGlassBackdrop : IDisposable {

        private readonly WindowHandle destination;
        private readonly IDwmGlassApi api;
        private bool applied;
        private bool disposed;

        public DwmGlassBackdrop (WindowHandle destination)
            : this(destination, new DwmGlassApi()) {
        }

        public DwmGlassBackdrop (WindowHandle destination, IDwmGlassApi api) {
            this.destination = destination;
            this.api = api ?? throw new ArgumentNullException(nameof(api));
            TryApply();
        }

        public bool IsAvailable { get; private set; }

        /// <summary>
        /// Applies an all-client-area glass frame. Failure is non-fatal because
        /// SessionForm can fall back to a compositor-managed shell thumbnail.
        /// </summary>
        public bool TryApply () {
            if( disposed ) {
                throw new ObjectDisposedException(nameof(DwmGlassBackdrop));
            }
            IsAvailable = false;
            if( destination == WindowHandle.Null ) {
                return false;
            }

            try {
                DwmMargins margins = DwmMargins.EntireWindow;
                int hresult = api.ExtendFrame(destination, ref margins);
                if( hresult < 0 ) {
                    Trace.WriteLine("DwmExtendFrameIntoClientArea failed with HRESULT 0x" +
                        hresult.ToString("X8"));
                    return false;
                }
                applied = true;
                IsAvailable = true;
                return true;
            }
            catch( Exception exception ) when(
                exception is DllNotFoundException ||
                exception is EntryPointNotFoundException ||
                exception is ExternalException ) {
                Trace.WriteLine("DWM glass backdrop unavailable: " + exception);
                return false;
            }
        }

        public void Dispose () {
            if( disposed ) {
                return;
            }

            disposed = true;
            IsAvailable = false;
            if( !applied || destination == WindowHandle.Null ) {
                return;
            }

            applied = false;
            try {
                DwmMargins margins = DwmMargins.None;
                int hresult = api.ExtendFrame(destination, ref margins);
                if( hresult < 0 ) {
                    Trace.WriteLine("Resetting the DWM glass frame failed with HRESULT 0x" +
                        hresult.ToString("X8"));
                }
            }
            catch( Exception exception ) when(
                exception is DllNotFoundException ||
                exception is EntryPointNotFoundException ||
                exception is ExternalException ) {
                Trace.WriteLine("Resetting the DWM glass frame failed: " + exception);
            }
        }

    }

    public interface IDwmGlassApi {

        int ExtendFrame (WindowHandle destination, ref DwmMargins margins);

    }

    [StructLayout(LayoutKind.Sequential)]
    public struct DwmMargins {

        public static DwmMargins None => new DwmMargins();

        public static DwmMargins EntireWindow => new DwmMargins {
            Left = -1,
            Right = -1,
            Top = -1,
            Bottom = -1
        };

        public int Left;
        public int Right;
        public int Top;
        public int Bottom;

    }

    public sealed class DwmGlassApi : IDwmGlassApi {

        public int ExtendFrame (WindowHandle destination, ref DwmMargins margins) =>
            DwmExtendFrameIntoClientArea(destination, ref margins);

        [DllImport("dwmapi.dll", ExactSpelling = true)]
        private static extern int DwmExtendFrameIntoClientArea (
            WindowHandle destination,
            ref DwmMargins margins);

    }

}
