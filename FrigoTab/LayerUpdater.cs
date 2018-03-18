using System;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Windows.Forms;

#pragma warning disable 414

namespace FrigoTab {

    public static class LayerUpdater {

        public delegate void Renderer (Graphics graphics);

        public static void Update (Form form, Renderer renderer) {
            if( form.IsDisposed ) {
                return;
            }
            IntPtr screenDc = GetDC(IntPtr.Zero);
            IntPtr memDc = CreateCompatibleDC(screenDc);
            IntPtr hBitmap = CreateCompatibleBitmap(screenDc, form.Bounds.Width, form.Bounds.Height);
            IntPtr hOldBitmap = SelectObject(memDc, hBitmap);
            using( Graphics graphics = Graphics.FromHdc(memDc) ) {
                renderer(graphics);
            }
            UpdateLayeredWindow(form, memDc);
            SelectObject(memDc, hOldBitmap);
            DeleteDC(memDc);
            DeleteObject(hBitmap);
            ReleaseDC(IntPtr.Zero, screenDc);
        }

        private struct BlendFunction {

            public BlendOperation BlendOperation;
            public byte BlendFlags;
            public byte SourceConstantAlpha;
            public AlphaFormat AlphaFormat;

        }

        private enum BlendOperation : byte {

            SourceOver = 0

        }

        private enum AlphaFormat : byte {

            SourceAlpha = 1

        }

        [Flags]
        private enum UpdateLayeredWindowFlags {

            Alpha = 2

        }

        private static void UpdateLayeredWindow (Control form, IntPtr hdc) {
            Point pptDst = form.Bounds.Location;
            Size pSize = form.Bounds.Size;
            Point pptSrc = Point.Empty;
            BlendFunction pblend = new BlendFunction {
                BlendOperation = BlendOperation.SourceOver,
                BlendFlags = 0,
                SourceConstantAlpha = 0xff,
                AlphaFormat = AlphaFormat.SourceAlpha
            };
            UpdateLayeredWindow(form.Handle, IntPtr.Zero, ref pptDst, ref pSize, hdc, ref pptSrc, 0, ref pblend, UpdateLayeredWindowFlags.Alpha);
        }

        [DllImport("user32.dll")]
        private static extern IntPtr GetDC (IntPtr hwnd);

        [DllImport("gdi32.dll")]
        private static extern IntPtr CreateCompatibleDC (IntPtr hdc);

        [DllImport("gdi32.dll")]
        private static extern IntPtr CreateCompatibleBitmap (IntPtr hdc, int cx, int cy);

        [DllImport("gdi32.dll")]
        private static extern IntPtr SelectObject (IntPtr hdc, IntPtr hobj);

        [DllImport("gdi32.dll")]
        private static extern bool DeleteDC (IntPtr hdc);

        [DllImport("gdi32.dll")]
        private static extern bool DeleteObject (IntPtr hobj);

        [DllImport("user32.dll")]
        private static extern int ReleaseDC (IntPtr hwnd, IntPtr hdc);

        [DllImport("user32.dll")]
        private static extern bool UpdateLayeredWindow (IntPtr hwnd, IntPtr hdcDst, ref Point pptDst, ref Size psize, IntPtr hdcSrc, ref Point pptSrc,
            int crKey, ref BlendFunction pblend, UpdateLayeredWindowFlags dwFlags);

    }

}
