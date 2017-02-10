using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class WindowFinder {

        public readonly IList<WindowHandle> Windows = new List<WindowHandle>();

        public WindowFinder () {
            EnumWindows(EnumWindowCallback, IntPtr.Zero);
            Windows.Remove(GetShellWindow());
            Windows.Remove(GetStartButton());
        }

        private bool EnumWindowCallback (WindowHandle handle, IntPtr lParam) {
            if( handle.IsWindowVisible() && (handle.GetWindowText().Length > 0) ) {
                Windows.Add(handle);
            }
            return true;
        }

        private IntPtr GetStartButton () {
            return FindWindowEx(GetDesktopWindow(), IntPtr.Zero, "Button", "Start");
        }

        private delegate bool EnumWindowsProc (WindowHandle handle, IntPtr lParam);

        [DllImport ("user32.dll")]
        private static extern bool EnumWindows (EnumWindowsProc enumFunc, IntPtr lParam);

        [DllImport ("user32.dll")]
        private static extern IntPtr GetShellWindow ();

        [DllImport ("user32.dll")]
        private static extern IntPtr GetDesktopWindow ();

        [DllImport ("user32.dll")]
        private static extern IntPtr FindWindowEx (IntPtr hwndParent,
            IntPtr hwndChildAfter,
            string lpszClass,
            string lpszWindow);

    }

}
