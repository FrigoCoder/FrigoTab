using System;
using System.Runtime.InteropServices;
using System.Text;

namespace FrigoTab {

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

        [DllImport ("user32.dll")]
        private static extern bool IsWindowVisible (IntPtr hWnd);

        [DllImport ("user32.dll")]
        private static extern int GetWindowTextLength (IntPtr hWnd);

        [DllImport ("user32.dll")]
        private static extern int GetWindowText (IntPtr hWnd, StringBuilder lpString, int nMaxCount);

    }

}
