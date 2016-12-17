using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Imaging;
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

        public Bitmap printWindow (IntPtr hWnd) {
            LPRECT rect;
            GetWindowRect(hWnd, out rect);
            Bitmap bmp = new Bitmap(rect.right - rect.left, rect.bottom - rect.top, PixelFormat.Format32bppArgb);

            using( Graphics gfx = Graphics.FromImage(bmp) ) {
                IntPtr hdc = gfx.GetHdc();
                try {
                    PrintWindow(hWnd, hdc, 0);
                } finally {
                    gfx.ReleaseHdc(hdc);
                }
            }

            return bmp;
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
