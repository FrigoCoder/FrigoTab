using System;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class Thumbnail : IDisposable {

        private readonly IntPtr _thumbnail;

        public Thumbnail (WindowHandle source, WindowHandle destination, Rect bounds) {
            DwmRegisterThumbnail(destination, source, out _thumbnail);
            DwmThumbnailProperties properties = new DwmThumbnailProperties {
                Flags = DwmThumbnailFlags.RectDestination | DwmThumbnailFlags.RectSource,
                Destination = bounds,
                Source = source.GetWindowRect()
            };
            DwmUpdateThumbnailProperties(_thumbnail, ref properties);
        }

        public void Dispose () {
            DwmUnregisterThumbnail(_thumbnail);
        }

        internal struct DwmThumbnailProperties {

            public DwmThumbnailFlags Flags;
            public Rect Destination;
            public Rect Source;
            public byte Opacity;
            public bool Visible;
            public bool SourceClientAreaOnly;

        }

        [Flags]
        internal enum DwmThumbnailFlags {

            None = 0,
            RectDestination = 1,
            RectSource = 2,
            Opacity = 4,
            Visible = 8,
            SourceClientAreaOnly = 16

        }

        [DllImport ("dwmapi.dll")]
        private static extern int DwmRegisterThumbnail (IntPtr dest, IntPtr src, out IntPtr thumb);

        [DllImport ("dwmapi.dll")]
        private static extern int DwmUnregisterThumbnail (IntPtr thumb);

        [DllImport ("dwmapi.dll")]
        private static extern int DwmUpdateThumbnailProperties (IntPtr thumb, ref DwmThumbnailProperties props);

    }

}
