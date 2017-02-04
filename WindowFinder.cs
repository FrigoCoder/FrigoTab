using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

namespace FastTab {

    public class WindowFinder {

        public IList<IntPtr> GetOpenWindows () {
            IList<IntPtr> windows = new List<IntPtr>();
            EnumWindows((hWnd, lParam) => {
                    if( IsWindowVisible(hWnd) && (GetWindowTextLength(hWnd) > 0) ) {
                        windows.Add(hWnd);
                    }
                    return true;
                },
                0);
            windows.Remove(GetShellWindow());
            windows.Remove(GetStartButton());
            return windows;
        }

        public string GetWindowText (IntPtr hWnd) {
            int length = GetWindowTextLength(hWnd);
            StringBuilder builder = new StringBuilder(length);
            GetWindowText(hWnd, builder, length + 1);
            return builder.ToString();
        }

        private IntPtr GetStartButton () {
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
        private static extern IntPtr FindWindowEx (IntPtr hwndParent,
            IntPtr hwndChildAfter,
            string lpszClass,
            string lpszWindow);

        [DllImport ("user32.dll")]
        private static extern bool GetWindowRect (IntPtr hWnd, out LPRECT lpRect);

        [DllImport ("user32.dll")]
        private static extern bool PrintWindow (IntPtr hWnd, IntPtr hdcBlt, int nFlags);

        private delegate bool EnumWindowsProc (IntPtr hWnd, int lParam);

        private struct LPRECT {

            public int left;
            public int top;
            public int right;
            public int bottom;

        }

    }

}
