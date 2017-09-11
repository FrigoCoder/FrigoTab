using System;
using System.Diagnostics.CodeAnalysis;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class Thumbnail : IDisposable {

        static Thumbnail () {
            DwmEnableComposition(DwmEnableCompositionConstants.EnableComposition);
        }

        private readonly WindowHandle _source;
        private readonly WindowHandle _destination;
        private readonly IntPtr _thumbnail;
        private bool _disposed;

        public Thumbnail (WindowHandle source, WindowHandle destination) {
            _source = source;
            _destination = destination;
            DwmRegisterThumbnail(destination, source, out _thumbnail);
        }

        public Thumbnail (WindowHandle source, WindowHandle destination, ScreenRect bounds) :
            this(source, destination) {
            Update(bounds);
        }

        ~Thumbnail () {
            Dispose();
        }

        public void Dispose () {
            if( _disposed ) {
                return;
            }
            DwmUnregisterThumbnail(_thumbnail);
            _disposed = true;
            GC.SuppressFinalize(this);
        }

        public Size GetSourceSize () {
            return _source.GetWindowRect().Size;
        }

        public void Update (ScreenRect destinationRect) {
            DwmThumbnailProperties properties = new DwmThumbnailProperties {
                Flags = DwmThumbnailFlags.RectSource | DwmThumbnailFlags.RectDestination,
                Source = new ClientRect(Point.Empty, GetSourceSize()),
                Destination = destinationRect.ScreenToClient(_destination)
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

        [Flags, SuppressMessage("ReSharper", "UnusedMember.Local")]
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

        [DllImport("dwmapi.dll")]
        private static extern int DwmEnableComposition (DwmEnableCompositionConstants uCompositionAction);

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
