using System;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class Overlay {

        private readonly ApplicationWindow _window;

        public Overlay (ApplicationWindow window) {
            _window = window;
            Draw();
        }

        private void Draw () {
            IntPtr screenDc = GetDC(IntPtr.Zero);
            IntPtr memDc = CreateCompatibleDC(screenDc);
            IntPtr hBitmap = CreateCompatibleBitmap(screenDc, _window.Bounds.Width, _window.Bounds.Height);
            IntPtr hOldBitmap = SelectObject(memDc, hBitmap);
            using( Graphics graphics = Graphics.FromHdc(memDc) ) {
                Render(graphics);
            }
            UpdateLayeredWindow(memDc);
            SelectObject(memDc, hOldBitmap);
            DeleteDC(memDc);
            DeleteObject(hBitmap);
            ReleaseDC(IntPtr.Zero, screenDc);
        }

        private void Render (Graphics graphics) {
        }

        private void UpdateLayeredWindow (IntPtr hdc) {
            Point pptDst = _window.Bounds.Location;
            Size pSize = _window.Bounds.Size;
            Point pptSrc = Point.Empty;
            byte[] pblend = {
                0,
                0,
                0xff,
                1
            };
            UpdateLayeredWindow(_window.Handle, IntPtr.Zero, ref pptDst, ref pSize, hdc, ref pptSrc, 0, pblend, 2);
        }

        [DllImport ("user32.dll")]
        private static extern IntPtr GetDC (IntPtr hwnd);

        [DllImport ("gdi32.dll")]
        private static extern IntPtr CreateCompatibleDC (IntPtr hdc);

        [DllImport ("gdi32.dll")]
        private static extern IntPtr CreateCompatibleBitmap (IntPtr hdc, int cx, int cy);

        [DllImport ("gdi32.dll")]
        private static extern IntPtr SelectObject (IntPtr hdc, IntPtr hobj);

        [DllImport ("gdi32.dll")]
        private static extern bool DeleteDC (IntPtr hdc);

        [DllImport ("gdi32.dll")]
        private static extern bool DeleteObject (IntPtr hobj);

        [DllImport ("user32.dll")]
        private static extern int ReleaseDC (IntPtr hwnd, IntPtr hdc);

        [DllImport ("user32.dll")]
        private static extern bool UpdateLayeredWindow (IntPtr hwnd,
            IntPtr hdcDst,
            ref Point pptDst,
            ref Size psize,
            IntPtr hdcSrc,
            ref Point pptSrc,
            int crKey,
            byte[] pblend,
            int dwFlags);

    }

}
