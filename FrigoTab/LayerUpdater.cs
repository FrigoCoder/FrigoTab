using System;
using System.Drawing;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Windows.Forms;

#pragma warning disable 414

namespace FrigoTab {

    public class LayerUpdater : IDisposable {

        public delegate void Renderer (Graphics graphics);

        private readonly Form form;
        private readonly IntPtr screenDc;
        private readonly IntPtr memDc;
        private readonly IntPtr hBitmap;
        private readonly IntPtr hOldBitmap;
        private bool disposed;

        public LayerUpdater (Form form) {
            this.form = form;

            IntPtr newScreenDc = IntPtr.Zero;
            IntPtr newMemDc = IntPtr.Zero;
            IntPtr newBitmap = IntPtr.Zero;
            IntPtr newOldBitmap = IntPtr.Zero;
            try {
                newScreenDc = GetDC(IntPtr.Zero);
                if( newScreenDc == IntPtr.Zero ) {
                    throw new Win32Exception(Marshal.GetLastWin32Error(), "GetDC failed.");
                }
                newMemDc = CreateCompatibleDC(newScreenDc);
                if( newMemDc == IntPtr.Zero ) {
                    throw new Win32Exception(Marshal.GetLastWin32Error(), "CreateCompatibleDC failed.");
                }
                newBitmap = CreateCompatibleBitmap(newScreenDc, form.Bounds.Width, form.Bounds.Height);
                if( newBitmap == IntPtr.Zero ) {
                    throw new Win32Exception(Marshal.GetLastWin32Error(), "CreateCompatibleBitmap failed.");
                }
                newOldBitmap = SelectObject(newMemDc, newBitmap);
                if( newOldBitmap == IntPtr.Zero || newOldBitmap == new IntPtr(-1) ) {
                    throw new Win32Exception(Marshal.GetLastWin32Error(), "SelectObject failed.");
                }

                screenDc = newScreenDc;
                memDc = newMemDc;
                hBitmap = newBitmap;
                hOldBitmap = newOldBitmap;
            }
            catch {
                if( newOldBitmap != IntPtr.Zero && newOldBitmap != new IntPtr(-1) ) {
                    SelectObject(newMemDc, newOldBitmap);
                }
                if( newBitmap != IntPtr.Zero ) {
                    DeleteObject(newBitmap);
                }
                if( newMemDc != IntPtr.Zero ) {
                    DeleteDC(newMemDc);
                }
                if( newScreenDc != IntPtr.Zero ) {
                    ReleaseDC(IntPtr.Zero, newScreenDc);
                }
                throw;
            }
        }

        ~LayerUpdater () => Dispose();

        public void Dispose () {
            if( disposed ) {
                return;
            }
            if( memDc != IntPtr.Zero && hOldBitmap != IntPtr.Zero && hOldBitmap != new IntPtr(-1) ) {
                SelectObject(memDc, hOldBitmap);
            }
            if( memDc != IntPtr.Zero ) {
                DeleteDC(memDc);
            }
            if( hBitmap != IntPtr.Zero ) {
                DeleteObject(hBitmap);
            }
            if( screenDc != IntPtr.Zero ) {
                ReleaseDC(IntPtr.Zero, screenDc);
            }
            disposed = true;
            GC.SuppressFinalize(this);
        }

        public void Update (Renderer renderer) {
            if( form.IsDisposed ) {
                return;
            }
            using( Graphics graphics = Graphics.FromHdc(memDc) ) {
                graphics.Clear(Color.Empty);
                renderer(graphics);
            }
            UpdateLayeredWindow();
        }

        private void UpdateLayeredWindow () {
            Point pptDst = form.Bounds.Location;
            Size pSize = form.Bounds.Size;
            Point pptSrc = Point.Empty;
            BlendFunction pblend = new BlendFunction {
                BlendOperation = BlendOperation.SourceOver,
                BlendFlags = 0,
                SourceConstantAlpha = 0xff,
                AlphaFormat = AlphaFormat.SourceAlpha
            };
            if( !UpdateLayeredWindow(form.Handle, IntPtr.Zero, ref pptDst, ref pSize, memDc, ref pptSrc, 0, ref pblend, UpdateLayeredWindowFlags.Alpha) ) {
                throw new Win32Exception(Marshal.GetLastWin32Error(), "UpdateLayeredWindow failed.");
            }
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
