using System;
using System.Drawing;

namespace FrigoTab {

    public class Thumbnail : IDisposable {

        private IntPtr thumbnail;

        public Thumbnail (WindowHandle source, WindowHandle destination) {
            Dwm.DwmRegisterThumbnail(destination, source, out thumbnail);
        }

        ~Thumbnail () {
            Dispose();
        }

        public void Dispose () {
            if( thumbnail == IntPtr.Zero ) {
                return;
            }
            Dwm.DwmUnregisterThumbnail(thumbnail);
            thumbnail = IntPtr.Zero;
            GC.SuppressFinalize(this);
        }

        public Size GetSourceSize () {
            Dwm.DwmQueryThumbnailSourceSize(thumbnail, out Size size);
            return size;
        }

        public void SetSourceRect (Rect sourceRect) {
            Dwm.ThumbnailProperties properties = new Dwm.ThumbnailProperties {
                Flags = Dwm.ThumbnailFlags.RectSource,
                Source = sourceRect
            };
            Dwm.DwmUpdateThumbnailProperties(thumbnail, ref properties);
        }

        public void SetDestinationRect (Rect destinationRect) {
            Dwm.ThumbnailProperties properties = new Dwm.ThumbnailProperties {
                Flags = Dwm.ThumbnailFlags.RectDestination,
                Destination = destinationRect
            };
            Dwm.DwmUpdateThumbnailProperties(thumbnail, ref properties);
        }

    }

}
