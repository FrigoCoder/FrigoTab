using System;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Text;
using System.Windows.Forms;

namespace FrigoTab {

    [Flags]
    public enum WindowStyles : long {

        Disabled = 0x8000000,
        Visible = 0x10000000,
        Minimize = 0x20000000

    }

    [Flags]
    public enum WindowExStyles : long {

        Transparent = 0x20,
        ToolWindow = 0x80,
        AppWindow = 0x40000,
        Layered = 0x80000,
        NoActivate = 0x8000000

    }

    public class WindowHandle {

        public static implicit operator WindowHandle (IntPtr handle) {
            return new WindowHandle(handle);
        }

        public static implicit operator IntPtr (WindowHandle handle) {
            return handle._handle;
        }

        private readonly IntPtr _handle;

        private WindowHandle (IntPtr handle) {
            _handle = handle;
        }

        public ScreenRect GetRect () {
            if( GetWindowStyles().HasFlag(WindowStyles.Minimize) ) {
                return GetRestoredRect();
            }
            return GetWindowRect();
        }

        public void SetForeground () {
            if( GetWindowStyles().HasFlag(WindowStyles.Minimize) ) {
                ShowWindow(_handle, ShowWindowCommand.Restore);
            }
            SetForegroundWindow(_handle);
        }

        public string GetWindowText () {
            StringBuilder text = new StringBuilder(GetWindowTextLength(_handle) + 1);
            GetWindowText(_handle, text, text.Capacity);
            return text.ToString();
        }

        public WindowStyles GetWindowStyles () {
            return (WindowStyles) GetWindowLongPtr(_handle, WindowLong.Style);
        }

        public WindowExStyles GetWindowExStyles () {
            return (WindowExStyles) GetWindowLongPtr(_handle, WindowLong.ExStyle);
        }

        private ScreenRect GetWindowRect () {
            ScreenRect lpRect;
            GetWindowRect(_handle, out lpRect);
            return lpRect;
        }

        private ScreenRect GetRestoredRect () {
            WindowPlacement placement = GetWindowPlacement();
            if( placement.Flags.HasFlag(WindowPlacementFlags.RestoreToMaximized) ) {
                return new ScreenRect(GetMaximizedLocation(), GetMaximizedSize());
            }
            return placement.NormalRectangle;
        }

        private Point GetMaximizedLocation () {
            Point[] points = {
                new Point(0, 0)
            };
            int xy = MapWindowPoints(_handle, IntPtr.Zero, points, points.Length);
            return new Point(xy & 0xffff, (xy >> 16) & 0xffff);
        }

        private Size GetMaximizedSize () {
            using( Form form = new Form() ) {
                IntPtr thumbnail;
                DwmRegisterThumbnail(form.Handle, _handle, out thumbnail);
                Size size;
                DwmQueryThumbnailSourceSize(thumbnail, out size);
                DwmUnregisterThumbnail(thumbnail);
                return size;
            }
        }

        private WindowPlacement GetWindowPlacement () {
            WindowPlacement placement = new WindowPlacement {
                Length = Marshal.SizeOf(typeof(WindowPlacement))
            };
            GetWindowPlacement(_handle, ref placement);
            return placement;
        }

        private struct WindowPlacement {

            public int Length;
            public WindowPlacementFlags Flags;
            public ShowWindowCommand ShowCommand;
            public Point MinimumPosition;
            public Point MaximumPosition;
            public ScreenRect NormalRectangle;

        }

        private enum WindowPlacementFlags {

            RestoreToMaximized = 2

        }

        private enum ShowWindowCommand {

            Restore = 9

        }

        private enum WindowLong {

            ExStyle = -20,
            Style = -16

        }

        [DllImport ("user32.dll")]
        private static extern bool GetWindowRect (IntPtr hWnd, out ScreenRect lpRect);

        [DllImport ("user32.dll")]
        private static extern int GetWindowTextLength (IntPtr hWnd);

        [DllImport ("user32.dll")]
        private static extern int GetWindowText (IntPtr hWnd, StringBuilder lpString, int nMaxCount);

        [DllImport ("user32.dll")]
        private static extern IntPtr GetWindowLongPtr (IntPtr hWnd, WindowLong nIndex);

        [DllImport ("user32.dll")]
        private static extern bool ShowWindow (IntPtr hWnd, ShowWindowCommand nCmdShow);

        [DllImport ("user32.dll")]
        private static extern bool SetForegroundWindow (IntPtr hwnd);

        [DllImport ("user32.dll")]
        private static extern bool GetWindowPlacement (IntPtr hWnd, ref WindowPlacement lpwndpl);

        [DllImport ("user32.dll")]
        private static extern int MapWindowPoints (IntPtr hWndFrom, IntPtr hWndTo, Point[] lpPoints, int cPoints);

        [DllImport ("dwmapi.dll")]
        private static extern int DwmRegisterThumbnail (IntPtr dest, IntPtr src, out IntPtr thumb);

        [DllImport ("dwmapi.dll")]
        private static extern int DwmUnregisterThumbnail (IntPtr thumb);

        [DllImport ("dwmapi.dll")]
        private static extern int DwmQueryThumbnailSourceSize (IntPtr thumb, out Size pSize);

    }

}
