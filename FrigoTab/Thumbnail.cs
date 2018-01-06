using System;
using System.Drawing;
using System.Runtime.InteropServices;

#pragma warning disable 169
#pragma warning disable 414

namespace FrigoTab {

    public class Thumbnail : IDisposable {

        private IntPtr thumbnail;

        public Thumbnail (WindowHandle source, WindowHandle destination) => DwmRegisterThumbnail(destination, source, out thumbnail);

        ~Thumbnail () => Dispose();

        public void Dispose () {
            if( thumbnail == IntPtr.Zero ) {
                return;
            }
            DwmUnregisterThumbnail(thumbnail);
            thumbnail = IntPtr.Zero;
            GC.SuppressFinalize(this);
        }

        public Size GetSourceSize () {
            DwmQueryThumbnailSourceSize(thumbnail, out Size size);
            return size;
        }

        public void SetSourceRect (Rect sourceRect) {
            ThumbnailProperties properties = new ThumbnailProperties {
                Flags = ThumbnailFlags.RectSource,
                Source = sourceRect
            };
            DwmUpdateThumbnailProperties(thumbnail, ref properties);
        }

        public void SetDestinationRect (Rect destinationRect) {
            ThumbnailProperties properties = new ThumbnailProperties {
                Flags = ThumbnailFlags.RectDestination,
                Destination = destinationRect
            };
            DwmUpdateThumbnailProperties(thumbnail, ref properties);
        }

        private struct ThumbnailProperties {

            public ThumbnailFlags Flags;
            public Rect Destination;
            public Rect Source;
            public byte Opacity;
            public bool Visible;
            public bool SourceClientAreaOnly;

        }

        [Flags]
        private enum ThumbnailFlags {

            RectDestination = 1,
            RectSource = 2

        }

        [DllImport("dwmapi.dll")]
        private static extern int DwmRegisterThumbnail (WindowHandle dest, WindowHandle src, out IntPtr thumb);

        [DllImport("dwmapi.dll")]
        private static extern int DwmUnregisterThumbnail (IntPtr thumb);

        [DllImport("dwmapi.dll")]
        private static extern int DwmUpdateThumbnailProperties (IntPtr thumb, ref ThumbnailProperties props);

        [DllImport("dwmapi.dll")]
        private static extern int DwmQueryThumbnailSourceSize (IntPtr thumb, out Size pSize);

    }

}
