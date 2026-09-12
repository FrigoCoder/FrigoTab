using System;
using System.Diagnostics;
using System.Runtime.InteropServices;

#pragma warning disable 169
#pragma warning disable 414

namespace FrigoTab {

    public class Thumbnail : IDisposable {

        private static readonly IDwmThumbnailApi NativeApi = new DwmThumbnailApi();

        private readonly IDwmThumbnailApi api;
        private IntPtr thumbnail;

        public Thumbnail (WindowHandle source, WindowHandle destination)
            : this(source, destination, null) {
        }

        public Thumbnail (WindowHandle source, WindowHandle destination, IDwmThumbnailApi api) {
            this.api = api ?? NativeApi;
            int hresult = this.api.Register(destination, source, out thumbnail);
            if( hresult < 0 ) {
                ReleaseAfterFailedRegistration();
            }
            ThrowIfFailed(hresult, "DwmRegisterThumbnail");
            if( thumbnail == IntPtr.Zero ) {
                throw new ExternalException("DwmRegisterThumbnail returned a null thumbnail handle.");
            }
        }

        ~Thumbnail () => Dispose();

        public void Dispose () {
            if( thumbnail == IntPtr.Zero ) {
                GC.SuppressFinalize(this);
                return;
            }

            IntPtr handle = thumbnail;
            thumbnail = IntPtr.Zero;
            try {
                int hresult = api.Unregister(handle);
                if( hresult < 0 ) {
                    WriteDiagnostic("DwmUnregisterThumbnail", hresult);
                }
            }
            catch( Exception exception ) {
                // Dispose is also called from the finalizer.  Never allow a
                // native teardown failure to escape into the finalizer thread.
                Trace.WriteLine("DwmUnregisterThumbnail threw: " + exception);
            }
            finally {
                GC.SuppressFinalize(this);
            }
        }

        public void SetSourceRect (Rect sourceRect) {
            DwmThumbnailProperties properties = new DwmThumbnailProperties {
                Flags = (DwmThumbnailFlags) (ThumbnailFlags.RectSource | ThumbnailFlags.Opacity),
                Source = sourceRect
            };
            Update(properties, setOpacity: true);
        }

        public void SetDestinationRect (Rect destinationRect) {
            DwmThumbnailProperties properties = new DwmThumbnailProperties {
                Flags = (DwmThumbnailFlags) (ThumbnailFlags.RectDestination | ThumbnailFlags.Opacity),
                Destination = destinationRect
            };
            Update(properties, setOpacity: true);
        }

        public void SetVisible (bool value) {
            DwmThumbnailProperties properties = new DwmThumbnailProperties {
                Flags = (DwmThumbnailFlags) ThumbnailFlags.Visible,
                Visible = value
            };
            Update(properties, setOpacity: false);
        }

        // Kept as a private compatibility shape for existing contract probes;
        // the public DwmThumbnailFlags is the type used by the native seam.
        [Flags]
        private enum ThumbnailFlags : uint {

            RectDestination = 1,
            RectSource = 2,
            Opacity = 4,
            Visible = 8

        }

        private void Update (DwmThumbnailProperties properties, bool setOpacity) {
            if( thumbnail == IntPtr.Zero ) {
                throw new ObjectDisposedException(nameof(Thumbnail));
            }

            if( setOpacity ) {
                properties.Opacity = byte.MaxValue;
            }
            int hresult = api.Update(thumbnail, ref properties);
            ThrowIfFailed(hresult, "DwmUpdateThumbnailProperties");
        }

        private static void ThrowIfFailed (int hresult, string operation) {
            if( hresult >= 0 ) {
                return;
            }
            Marshal.ThrowExceptionForHR(hresult);
            throw new ExternalException(operation + " failed.", hresult);
        }

        private static void WriteDiagnostic (string operation, int hresult) {
            Trace.WriteLine(operation + " failed with HRESULT 0x" + hresult.ToString("X8"));
        }

        private void ReleaseAfterFailedRegistration () {
            if( thumbnail == IntPtr.Zero ) {
                return;
            }

            IntPtr handle = thumbnail;
            thumbnail = IntPtr.Zero;
            try {
                int unregisterResult = api.Unregister(handle);
                if( unregisterResult < 0 ) {
                    WriteDiagnostic("DwmUnregisterThumbnail after registration failure", unregisterResult);
                }
            }
            catch( Exception exception ) {
                Trace.WriteLine("DwmUnregisterThumbnail after registration failure threw: " + exception);
            }
        }

    }

}
