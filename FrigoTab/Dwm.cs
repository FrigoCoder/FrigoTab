using System.Runtime.InteropServices;

namespace FrigoTab {

    public static class Dwm {

        public enum WindowAttribute {

            ExtendedFrameBounds = 0x9,
            Cloaked = 0xe

        }

        public static bool IsCloaked (WindowHandle window) {
            DwmGetWindowAttribute(window, WindowAttribute.Cloaked, out bool cloaked, Marshal.SizeOf(typeof(bool)));
            return cloaked;
        }

        public static Rect GetExtendedFrameBounds (WindowHandle window) {
            DwmGetWindowAttribute(window, WindowAttribute.ExtendedFrameBounds, out Rect rect, Marshal.SizeOf(typeof(Rect)));
            return rect;
        }

        [DllImport("dwmapi.dll")]
        public static extern int DwmGetWindowAttribute (WindowHandle hWnd, WindowAttribute dwAttribute, out bool pvAttribute, int cbAttribute);

        [DllImport("dwmapi.dll")]
        public static extern int DwmGetWindowAttribute (WindowHandle hWnd, WindowAttribute dwAttribute, out Rect pvAttribute, int cbAttribute);

    }

}
