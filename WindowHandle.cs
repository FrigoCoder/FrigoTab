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

        public delegate void SendMessageDelegate (IntPtr hWnd, int msg, IntPtr dwData, IntPtr lResult);

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

        public Rect GetRestoredWindowRect () {
            if( GetWindowStyles().HasFlag(WindowStyles.Minimize) ) {
                WindowPlacement placement = GetWindowPlacement();
                if( placement.Flags == WindowPlacementFlags.RestoreToMaximized ) {
                    return new Rect(Screen.FromPoint(placement.MaximumPosition).WorkingArea);
                }
                return placement.NormalRectangle;
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

        public Icon IconFromGetClassLongPtr () {
            IntPtr icon = GetClassLongPtr(_handle, ClassLong.Icon);
            return icon != IntPtr.Zero ? Icon.FromHandle(icon) : null;
        }

        public void IconFromCallback (SendMessageDelegate callback) {
            SendMessageCallback(_handle, WindowsMessages.GetIcon, (IntPtr) GetIconSize.Big, (IntPtr) 0, callback, 0);
        }

        public WindowStyles GetWindowStyles () {
            return (WindowStyles) GetWindowLongPtr(_handle, WindowLong.Style);
        }

        public WindowExStyles GetWindowExStyles () {
            return (WindowExStyles) GetWindowLongPtr(_handle, WindowLong.ExStyle);
        }

        private Rect GetWindowRect () {
            Rect lpRect;
            GetWindowRect(_handle, out lpRect);
            return lpRect;
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
            public Rect NormalRectangle;

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

        private enum ClassLong {

            Icon = -14

        }

        private enum WindowsMessages {

            GetIcon = 127

        }

        private enum GetIconSize {

            Big = 1

        }

        [DllImport ("user32.dll")]
        private static extern bool GetWindowRect (IntPtr hWnd, out Rect lpRect);

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
        private static extern IntPtr GetClassLongPtr (IntPtr hWnd, ClassLong nIndex);

        [DllImport ("user32.dll")]
        private static extern bool SendMessageCallback (IntPtr hWnd,
            WindowsMessages message,
            IntPtr wParam,
            IntPtr lParam,
            SendMessageDelegate lpCallBack,
            long dwData);

    }

}
