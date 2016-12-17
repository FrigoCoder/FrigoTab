using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

namespace FastTab {

    public class WindowFinder {

        public IList<IntPtr> getOpenWindows () {
            IList<IntPtr> windows = new List<IntPtr>();
            EnumWindows((hWnd, lParam) => {
                    if( IsWindowVisible(hWnd) && (GetWindowTextLength(hWnd) > 0) ) {
                        windows.Add(hWnd);
                    }
                    return true;
                },
                0);
            windows.Remove(GetShellWindow());
            windows.Remove(getStartButton());
            return windows;
        }

        public string getWindowText (IntPtr hWnd) {
            int length = GetWindowTextLength(hWnd);
            StringBuilder builder = new StringBuilder(length);
            GetWindowText(hWnd, builder, length + 1);
            return builder.ToString();
        }

        private IntPtr getStartButton () {
            return FindWindowEx(GetDesktopWindow(), IntPtr.Zero, "Button", "Start");
        }

        [DllImport ("user32.dll")]
        private static extern bool EnumWindows (EnumWindowsProc enumFunc, int lParam);

        [DllImport ("user32.dll")]
        private static extern int GetWindowText (IntPtr hWnd, StringBuilder lpString, int nMaxCount);

        [DllImport ("user32.dll")]
        private static extern int GetWindowTextLength (IntPtr hWnd);

        [DllImport ("user32.dll")]
        private static extern bool IsWindowVisible (IntPtr hWnd);

        [DllImport ("user32.dll")]
        private static extern IntPtr GetShellWindow ();

        [DllImport ("user32.dll")]
        private static extern IntPtr GetDesktopWindow ();

        [DllImport ("user32.dll")]
        public static extern IntPtr FindWindowEx (IntPtr hwndParent,
            IntPtr hwndChildAfter,
            string lpszClass,
            string lpszWindow);

        private delegate bool EnumWindowsProc (IntPtr hWnd, int lParam);

    }

}
