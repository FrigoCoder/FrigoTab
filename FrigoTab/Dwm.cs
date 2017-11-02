using System;
using System.Diagnostics.CodeAnalysis;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public static class Dwm {

        public struct ThumbnailProperties {

            public ThumbnailFlags Flags;
            public Rect Destination;
            public Rect Source;
            public byte Opacity;
            public bool Visible;
            public bool SourceClientAreaOnly;

        }

        [Flags]
        [SuppressMessage("ReSharper", "UnusedMember.Local")]
        public enum ThumbnailFlags {

            RectDestination = 1,
            RectSource = 2,
            Opacity = 4,
            Visible = 8,
            SourceClientAreaOnly = 16

        }

        public enum WindowAttribute {

            ExtendedFrameBounds = 0x9,
            Cloaked = 0xe

        }

        public static bool IsCloaked (WindowHandle window) {
            bool cloaked;
            DwmGetWindowAttribute(window, WindowAttribute.Cloaked, out cloaked, Marshal.SizeOf(typeof(bool)));
            return cloaked;
        }

        public static Rect GetExtendedFrameBounds (WindowHandle window) {
            Rect rect;
            DwmGetWindowAttribute(window, WindowAttribute.ExtendedFrameBounds, out rect, Marshal.SizeOf(typeof(Rect)));
            return rect;
        }

        [DllImport("dwmapi.dll")]
        public static extern int DwmRegisterThumbnail (IntPtr dest, IntPtr src, out IntPtr thumb);

        [DllImport("dwmapi.dll")]
        public static extern int DwmUnregisterThumbnail (IntPtr thumb);

        [DllImport("dwmapi.dll")]
        public static extern int DwmUpdateThumbnailProperties (IntPtr thumb, ref ThumbnailProperties props);

        [DllImport("dwmapi.dll")]
        public static extern int DwmQueryThumbnailSourceSize (IntPtr thumb, out Size pSize);

        [DllImport("dwmapi.dll")]
        public static extern int DwmGetWindowAttribute (IntPtr hWnd, WindowAttribute dwAttribute, out bool pvAttribute, int cbAttribute);

        [DllImport("dwmapi.dll")]
        public static extern int DwmGetWindowAttribute (IntPtr hWnd, WindowAttribute dwAttribute, out Rect pvAttribute, int cbAttribute);

    }

}
