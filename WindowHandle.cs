using System;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Text;

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

        public ScreenRect GetWindowRect () {
            ScreenRect lpRect;
            GetWindowRect(_handle, out lpRect);
            return lpRect;
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

        public Icon IconFromGetClassLongPtr () {
            IntPtr icon = GetClassLongPtr(_handle, ClassLong.Icon);
            return icon == IntPtr.Zero ? null : Icon.FromHandle(icon);
        }

        private enum ClassLong {

            Icon = -14

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
        private static extern IntPtr GetClassLongPtr (IntPtr hWnd, ClassLong nIndex);

    }

}
