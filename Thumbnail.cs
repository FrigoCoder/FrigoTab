using System;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class Thumbnail : IDisposable {

        static Thumbnail () {
            DwmEnableComposition(DwmEnableCompositionConstants.EnableComposition);
        }

        private readonly IntPtr _thumbnail;

        public Thumbnail (WindowHandle source, WindowHandle destination, ScreenRect bounds) {
            DwmRegisterThumbnail(destination, source, out _thumbnail);
            DwmThumbnailProperties properties = new DwmThumbnailProperties {
                Flags = DwmThumbnailFlags.RectSource | DwmThumbnailFlags.RectDestination,
                Source = new ClientRect(Point.Empty, source.GetRect().Size),
                Destination = ScreenToClient(destination, bounds)
            };
            DwmUpdateThumbnailProperties(_thumbnail, ref properties);
        }

        public void Dispose () {
            DwmUnregisterThumbnail(_thumbnail);
        }

        private ClientRect ScreenToClient (WindowHandle window, ScreenRect bounds) {
            Point location = bounds.Location;
            ScreenToClient(window, ref location);
            return new ClientRect(location, bounds.Size);
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
            RectSource = 2

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

        [DllImport ("user32.dll")]
        private static extern bool ScreenToClient (IntPtr hWnd, ref Point lpPoint);

    }

}
