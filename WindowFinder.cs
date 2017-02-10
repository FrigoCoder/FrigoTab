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
            if( IsWindow(handle) ) {
                Windows.Add(handle);
            }
            return true;
        }

        private delegate bool EnumWindowsProc (WindowHandle handle, IntPtr lParam);

        private static bool IsWindow (WindowHandle handle) {
            return handle.IsWindowVisible() && (handle.GetWindowText().Length > 0);
        }

        private static WindowHandle GetStartButton () {
            return FindWindowEx(GetDesktopWindow(), IntPtr.Zero, "Button", "Start");
        }

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
