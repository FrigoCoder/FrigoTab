using System;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class Thumbnail : IDisposable {

        static Thumbnail () {
            DwmEnableComposition(DwmEnableCompositionConstants.EnableComposition);
        }

        private readonly WindowHandle _destination;
        private readonly IntPtr _thumbnail;

        public Thumbnail (WindowHandle source, WindowHandle destination) {
            _destination = destination;
            DwmRegisterThumbnail(destination, source, out _thumbnail);
            SetSourceRect(new ClientRect(Point.Empty, GetSourceSize()));
        }

        public Thumbnail (WindowHandle source, WindowHandle destination, ScreenRect bounds) : this(source, destination) {
            SetDestinationRect(bounds);
        }

        public void Dispose () {
            DwmUnregisterThumbnail(_thumbnail);
        }

        public void SetDestinationRect (ScreenRect bounds) {
            SetDestinationRect(bounds.ScreenToClient(_destination));
        }

        public Size GetSourceSize () {
            Size size;
            DwmQueryThumbnailSourceSize(_thumbnail, out size);
            return size;
        }

        private void SetSourceRect (ClientRect bounds) {
            DwmThumbnailProperties properties = new DwmThumbnailProperties {
                Flags = DwmThumbnailFlags.RectSource,
                Source = bounds
            };
            DwmUpdateThumbnailProperties(_thumbnail, ref properties);
        }

        private void SetDestinationRect (ClientRect bounds) {
            DwmThumbnailProperties properties = new DwmThumbnailProperties {
                Flags = DwmThumbnailFlags.RectDestination,
                Destination = bounds
            };
            DwmUpdateThumbnailProperties(_thumbnail, ref properties);
        }

        private struct DwmThumbnailProperties {

            public DwmThumbnailFlags Flags;
            public ClientRect Destination;
            public ClientRect Source;
            public byte Opacity;
            public bool Visible;
            public bool SourceClientAreaOnly;

        }

        [Flags]
        private enum DwmThumbnailFlags {

            RectDestination = 1,
            RectSource = 2,
            Opacity = 4,
            Visible = 8,
            SourceClientAreaOnly = 16

        }

        private enum DwmEnableCompositionConstants {

            EnableComposition = 1

        }

        [DllImport ("dwmapi.dll")]
        private static extern int DwmEnableComposition (DwmEnableCompositionConstants uCompositionAction);

        [DllImport ("dwmapi.dll")]
        private static extern int DwmRegisterThumbnail (IntPtr dest, IntPtr src, out IntPtr thumb);

        [DllImport ("dwmapi.dll")]
        private static extern int DwmUnregisterThumbnail (IntPtr thumb);

        [DllImport ("dwmapi.dll")]
        private static extern int DwmUpdateThumbnailProperties (IntPtr thumb, ref DwmThumbnailProperties props);

        [DllImport ("dwmapi.dll")]
        private static extern int DwmQueryThumbnailSourceSize (IntPtr thumb, out Size pSize);

    }

}
