using System;
using System.Runtime.InteropServices;
using System.Text;

namespace FrigoTab {

    [Flags]
    public enum WindowStyles : long {

        Disabled = 0x8000000,
        Visible = 0x10000000

    }

    public struct WindowHandle {

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

        public bool IsWindowVisible () {
            return IsWindowVisible(_handle);
        }

        public string GetWindowText () {
            StringBuilder text = new StringBuilder(GetWindowTextLength(_handle) + 1);
            GetWindowText(_handle, text, text.Capacity);
            return text.ToString();
        }

        public WindowStyles GetWindowStyles () {
            return (WindowStyles) GetWindowLongPtr(_handle, WindowLong.Style);
        }

        private enum WindowLong {

            Style = -16

        }

        [DllImport ("user32.dll")]
        private static extern bool IsWindowVisible (IntPtr hWnd);

        [DllImport ("user32.dll")]
        private static extern int GetWindowTextLength (IntPtr hWnd);

        [DllImport ("user32.dll")]
        private static extern int GetWindowText (IntPtr hWnd, StringBuilder lpString, int nMaxCount);

        [DllImport ("user32.dll")]
        private static extern IntPtr GetWindowLongPtr (IntPtr hWnd, WindowLong nIndex);

    }

}
