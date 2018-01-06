using System;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class BackgroundWindow : IDisposable {

        private readonly Thumbnail thumbnail;
        private bool disposed;

        public BackgroundWindow (FrigoForm owner, WindowHandle window) {
            thumbnail = new Thumbnail(window, owner.WindowHandle);
            thumbnail.SetSourceRect(GetExtendedFrameBounds(window).ScreenToClient(window));
            thumbnail.SetDestinationRect(GetExtendedFrameBounds(window).ScreenToClient(owner.WindowHandle));
        }

        ~BackgroundWindow () => Dispose();

        public void Dispose () {
            if( disposed ) {
                return;
            }
            thumbnail.Dispose();
            disposed = true;
            GC.SuppressFinalize(this);
        }

        private enum WindowAttribute {

            ExtendedFrameBounds = 0x9

        }

        private static Rect GetExtendedFrameBounds (WindowHandle window) {
            DwmGetWindowAttribute(window, WindowAttribute.ExtendedFrameBounds, out Rect rect, Marshal.SizeOf(typeof(Rect)));
            return rect;
        }

        [DllImport("dwmapi.dll")]
        private static extern int DwmGetWindowAttribute (WindowHandle hWnd, WindowAttribute dwAttribute, out Rect pvAttribute, int cbAttribute);

    }

}
