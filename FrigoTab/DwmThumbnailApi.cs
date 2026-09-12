using System;
using System.Runtime.InteropServices;

namespace FrigoTab {

    /// <summary>
    /// Native DWM thumbnail operations.  The interface keeps the production
    /// thumbnail wrapper testable without requiring a live desktop compositor.
    /// </summary>
    public interface IDwmThumbnailApi {

        int Register (WindowHandle destination, WindowHandle source, out IntPtr thumbnail);
        int Unregister (IntPtr thumbnail);
        int Update (IntPtr thumbnail, ref DwmThumbnailProperties properties);

    }

    [Flags]
    public enum DwmThumbnailFlags : uint {

        RectDestination = 0x00000001,
        RectSource = 0x00000002,
        Opacity = 0x00000004,
        Visible = 0x00000008,
        SourceClientAreaOnly = 0x00000010

    }

    [StructLayout(LayoutKind.Sequential)]
    public struct DwmThumbnailProperties {

        public DwmThumbnailFlags Flags;
        public Rect Destination;
        public Rect Source;
        public byte Opacity;
        [MarshalAs(UnmanagedType.Bool)]
        public bool Visible;
        [MarshalAs(UnmanagedType.Bool)]
        public bool SourceClientAreaOnly;

    }

    /// <summary>
    /// Direct P/Invoke implementation used by the application.
    /// </summary>
    public sealed class DwmThumbnailApi : IDwmThumbnailApi {

        public int Register (WindowHandle destination, WindowHandle source, out IntPtr thumbnail) =>
            DwmRegisterThumbnail(destination, source, out thumbnail);

        public int Unregister (IntPtr thumbnail) => DwmUnregisterThumbnail(thumbnail);

        public int Update (IntPtr thumbnail, ref DwmThumbnailProperties properties) =>
            DwmUpdateThumbnailProperties(thumbnail, ref properties);

        [DllImport("dwmapi.dll")]
        private static extern int DwmRegisterThumbnail (WindowHandle destination, WindowHandle source, out IntPtr thumbnail);

        [DllImport("dwmapi.dll")]
        private static extern int DwmUnregisterThumbnail (IntPtr thumbnail);

        [DllImport("dwmapi.dll")]
        private static extern int DwmUpdateThumbnailProperties (IntPtr thumbnail, ref DwmThumbnailProperties properties);

    }

}
