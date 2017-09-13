using System;
using System.Diagnostics.CodeAnalysis;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class Thumbnail : IDisposable {

        private IntPtr _thumbnail;

        public Thumbnail (WindowHandle source, WindowHandle destination) {
            DwmRegisterThumbnail(destination, source, out _thumbnail);
        }

        ~Thumbnail () {
            Dispose();
        }

        public void Dispose () {
            if( _thumbnail == IntPtr.Zero ) {
                return;
            }
            DwmUnregisterThumbnail(_thumbnail);
            _thumbnail = IntPtr.Zero;
            GC.SuppressFinalize(this);
        }

        public Size GetSourceSize () {
            Size size;
            DwmQueryThumbnailSourceSize(_thumbnail, out size);
            return size;
        }

        public Rect GetSourceRect () {
            return new Rect(Point.Empty, GetSourceSize());
        }

        public void Update (Rect destinationRect) {
            DwmThumbnailProperties properties = new DwmThumbnailProperties {
                Flags = DwmThumbnailFlags.RectSource | DwmThumbnailFlags.RectDestination,
                Source = GetSourceRect(),
                Destination = destinationRect
            };
            DwmUpdateThumbnailProperties(_thumbnail, ref properties);
        }

        private struct DwmThumbnailProperties {

            public DwmThumbnailFlags Flags;
            public Rect Destination;
            public Rect Source;
            public byte Opacity;
            public bool Visible;
            public bool SourceClientAreaOnly;

        }

        [Flags, SuppressMessage("ReSharper", "UnusedMember.Local")]
        private enum DwmThumbnailFlags {

            RectDestination = 1,
            RectSource = 2,
            Opacity = 4,
            Visible = 8,
            SourceClientAreaOnly = 16

        }

        [DllImport("dwmapi.dll")]
        private static extern int DwmRegisterThumbnail (IntPtr dest, IntPtr src, out IntPtr thumb);

        [DllImport("dwmapi.dll")]
        private static extern int DwmUnregisterThumbnail (IntPtr thumb);

        [DllImport("dwmapi.dll")]
        private static extern int DwmUpdateThumbnailProperties (IntPtr thumb, ref DwmThumbnailProperties props);

        [DllImport("dwmapi.dll")]
        private static extern int DwmQueryThumbnailSourceSize (IntPtr thumb, out Size pSize);

    }

}
