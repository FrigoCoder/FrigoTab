using System;
using System.ComponentModel;
using System.Diagnostics;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    /// <summary>
    /// Captures the visible virtual desktop once while the switcher is hidden
    /// and retains that opaque frame for inexpensive native repainting.
    /// </summary>
    public sealed class DesktopSnapshot : IDisposable {

        private IDesktopSnapshotFrame frame;
        private bool disposed;

        public DesktopSnapshot (Rectangle sourceBounds)
            : this(sourceBounds, new GdiDesktopSnapshotApi()) {
        }

        public DesktopSnapshot (Rectangle sourceBounds, IDesktopSnapshotApi api) {
            if( api == null ) {
                throw new ArgumentNullException(nameof(api));
            }
            if( sourceBounds.Width <= 0 || sourceBounds.Height <= 0 ) {
                return;
            }

            try {
                frame = api.Capture(sourceBounds);
            }
            catch( Exception exception ) {
                // Screen capture can be unavailable on protected or remote
                // desktops. Opening remains fail-open with an opaque black
                // backdrop instead of rejecting the keyboard gesture.
                Trace.WriteLine("Desktop snapshot capture failed: " + exception);
            }
        }

        public bool IsAvailable => frame != null;

        public void Draw (Graphics graphics, Rectangle destinationBounds) {
            if( disposed ) {
                throw new ObjectDisposedException(nameof(DesktopSnapshot));
            }
            if( graphics == null ) {
                throw new ArgumentNullException(nameof(graphics));
            }

            IDesktopSnapshotFrame currentFrame = frame;
            if( currentFrame == null ||
                destinationBounds.Width <= 0 || destinationBounds.Height <= 0 ) {
                graphics.Clear(Color.Black);
                return;
            }

            try {
                currentFrame.Draw(graphics, destinationBounds);
            }
            catch( Exception exception ) {
                // A lost display surface must not escape WM_PAINT.
                Trace.WriteLine("Desktop snapshot paint failed: " + exception);
                graphics.Clear(Color.Black);
            }
        }

        public void Dispose () {
            if( disposed ) {
                return;
            }

            disposed = true;
            IDesktopSnapshotFrame currentFrame = frame;
            frame = null;
            try {
                currentFrame?.Dispose();
            }
            catch( Exception exception ) {
                Trace.WriteLine("Desktop snapshot cleanup failed: " + exception);
            }
            GC.SuppressFinalize(this);
        }

    }

    public interface IDesktopSnapshotApi {

        IDesktopSnapshotFrame Capture (Rectangle sourceBounds);

    }

    public interface IDesktopSnapshotFrame : IDisposable {

        Size Size { get; }

        void Draw (Graphics graphics, Rectangle destinationBounds);

    }

    /// <summary>
    /// Uses one native screen-to-memory BitBlt and retains its DIB/DC. This
    /// avoids GDI+ pixel conversion during capture and DrawImage scaling or
    /// allocation during every owner repaint.
    /// </summary>
    public sealed class GdiDesktopSnapshotApi : IDesktopSnapshotApi {

        public IDesktopSnapshotFrame Capture (Rectangle sourceBounds) =>
            new GdiDesktopSnapshotFrame(sourceBounds);

    }

    public sealed class GdiDesktopSnapshotFrame : IDesktopSnapshotFrame {

        private const uint SourceCopy = 0x00cc0020;
        private const uint DibRgbColors = 0;
        private const uint BitmapInfoHeaderSize = 40;
        private const ushort BitmapPlanes = 1;
        private const ushort BitsPerPixel = 32;
        private const int ColorOnColor = 3;

        private IntPtr memoryDc;
        private IntPtr bitmap;
        private IntPtr previousBitmap;
        private bool disposed;

        public GdiDesktopSnapshotFrame (Rectangle sourceBounds) {
            if( sourceBounds.Width <= 0 || sourceBounds.Height <= 0 ) {
                throw new ArgumentOutOfRangeException(nameof(sourceBounds));
            }

            Size = sourceBounds.Size;
            IntPtr screenDc = GetDC(IntPtr.Zero);
            if( screenDc == IntPtr.Zero ) {
                throw LastWin32Exception("GetDC failed while capturing the desktop.");
            }

            IntPtr newMemoryDc = IntPtr.Zero;
            IntPtr newBitmap = IntPtr.Zero;
            IntPtr newPreviousBitmap = IntPtr.Zero;
            try {
                newMemoryDc = CreateCompatibleDC(screenDc);
                if( newMemoryDc == IntPtr.Zero ) {
                    throw LastWin32Exception("CreateCompatibleDC failed while capturing the desktop.");
                }

                BitmapInfo bitmapInfo = new BitmapInfo {
                    Header = new BitmapInfoHeader {
                        Size = BitmapInfoHeaderSize,
                        Width = sourceBounds.Width,
                        // A negative height creates a top-down DIB whose first
                        // scan line is the top of the virtual desktop.
                        Height = -sourceBounds.Height,
                        Planes = BitmapPlanes,
                        BitCount = BitsPerPixel
                    }
                };
                IntPtr bits;
                newBitmap = CreateDIBSection(
                    screenDc,
                    ref bitmapInfo,
                    DibRgbColors,
                    out bits,
                    IntPtr.Zero,
                    0);
                if( newBitmap == IntPtr.Zero ) {
                    throw LastWin32Exception("CreateDIBSection failed while capturing the desktop.");
                }

                newPreviousBitmap = SelectObject(newMemoryDc, newBitmap);
                if( newPreviousBitmap == IntPtr.Zero || newPreviousBitmap == new IntPtr(-1) ) {
                    throw LastWin32Exception("SelectObject failed while capturing the desktop.");
                }

                // SourceCopy deliberately matches the previous
                // Graphics.CopyFromScreen behavior. CAPTUREBLT is slower and
                // would change which transient layered windows are included.
                if( !BitBlt(
                    newMemoryDc,
                    0,
                    0,
                    sourceBounds.Width,
                    sourceBounds.Height,
                    screenDc,
                    sourceBounds.Left,
                    sourceBounds.Top,
                    SourceCopy) ) {
                    throw LastWin32Exception("BitBlt failed while capturing the desktop.");
                }

                memoryDc = newMemoryDc;
                bitmap = newBitmap;
                previousBitmap = newPreviousBitmap;
                newMemoryDc = IntPtr.Zero;
                newBitmap = IntPtr.Zero;
                newPreviousBitmap = IntPtr.Zero;
            }
            catch {
                Release(ref newMemoryDc, ref newBitmap, ref newPreviousBitmap);
                throw;
            }
            finally {
                ReleaseDC(IntPtr.Zero, screenDc);
            }
        }

        public Size Size { get; }

        ~GdiDesktopSnapshotFrame () {
            Dispose(false);
        }

        public void Draw (Graphics graphics, Rectangle destinationBounds) {
            if( disposed ) {
                throw new ObjectDisposedException(nameof(GdiDesktopSnapshotFrame));
            }
            if( graphics == null ) {
                throw new ArgumentNullException(nameof(graphics));
            }
            if( destinationBounds.Width <= 0 || destinationBounds.Height <= 0 ) {
                return;
            }

            IntPtr destinationDc = graphics.GetHdc();
            try {
                bool painted;
                if( destinationBounds.Size == Size ) {
                    painted = BitBlt(
                        destinationDc,
                        destinationBounds.Left,
                        destinationBounds.Top,
                        Size.Width,
                        Size.Height,
                        memoryDc,
                        0,
                        0,
                        SourceCopy);
                }
                else {
                    int previousMode = SetStretchBltMode(destinationDc, ColorOnColor);
                    painted = StretchBlt(
                        destinationDc,
                        destinationBounds.Left,
                        destinationBounds.Top,
                        destinationBounds.Width,
                        destinationBounds.Height,
                        memoryDc,
                        0,
                        0,
                        Size.Width,
                        Size.Height,
                        SourceCopy);
                    if( previousMode != 0 ) {
                        SetStretchBltMode(destinationDc, previousMode);
                    }
                }

                if( !painted ) {
                    throw LastWin32Exception("BitBlt failed while painting the desktop snapshot.");
                }
            }
            finally {
                graphics.ReleaseHdc(destinationDc);
            }
        }

        public void Dispose () {
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        private void Dispose (bool disposing) {
            if( disposed ) {
                return;
            }

            disposed = true;
            Release(ref memoryDc, ref bitmap, ref previousBitmap);
        }

        private static void Release (
            ref IntPtr currentMemoryDc,
            ref IntPtr currentBitmap,
            ref IntPtr currentPreviousBitmap) {
            if( currentMemoryDc != IntPtr.Zero &&
                currentPreviousBitmap != IntPtr.Zero &&
                currentPreviousBitmap != new IntPtr(-1) ) {
                SelectObject(currentMemoryDc, currentPreviousBitmap);
            }
            currentPreviousBitmap = IntPtr.Zero;

            if( currentBitmap != IntPtr.Zero ) {
                DeleteObject(currentBitmap);
                currentBitmap = IntPtr.Zero;
            }
            if( currentMemoryDc != IntPtr.Zero ) {
                DeleteDC(currentMemoryDc);
                currentMemoryDc = IntPtr.Zero;
            }
        }

        private static Win32Exception LastWin32Exception (string message) {
            int error = Marshal.GetLastWin32Error();
            return new Win32Exception(error == 0 ? 1 : error, message);
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct BitmapInfo {

            public BitmapInfoHeader Header;
            public uint RedMask;
            public uint GreenMask;
            public uint BlueMask;

        }

        [StructLayout(LayoutKind.Sequential)]
        private struct BitmapInfoHeader {

            public uint Size;
            public int Width;
            public int Height;
            public ushort Planes;
            public ushort BitCount;
            public uint Compression;
            public uint SizeImage;
            public int XPelsPerMeter;
            public int YPelsPerMeter;
            public uint ColorsUsed;
            public uint ColorsImportant;

        }

        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
        private static extern IntPtr GetDC (IntPtr window);

        [DllImport("user32.dll", ExactSpelling = true)]
        private static extern int ReleaseDC (IntPtr window, IntPtr dc);

        [DllImport("gdi32.dll", ExactSpelling = true, SetLastError = true)]
        private static extern IntPtr CreateCompatibleDC (IntPtr dc);

        [DllImport("gdi32.dll", ExactSpelling = true, SetLastError = true)]
        private static extern IntPtr CreateDIBSection (
            IntPtr dc,
            ref BitmapInfo bitmapInfo,
            uint usage,
            out IntPtr bits,
            IntPtr section,
            uint offset);

        [DllImport("gdi32.dll", ExactSpelling = true, SetLastError = true)]
        private static extern IntPtr SelectObject (IntPtr dc, IntPtr value);

        [DllImport("gdi32.dll", ExactSpelling = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool DeleteObject (IntPtr value);

        [DllImport("gdi32.dll", ExactSpelling = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool DeleteDC (IntPtr dc);

        [DllImport("gdi32.dll", ExactSpelling = true, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool BitBlt (
            IntPtr destination,
            int destinationX,
            int destinationY,
            int width,
            int height,
            IntPtr source,
            int sourceX,
            int sourceY,
            uint operation);

        [DllImport("gdi32.dll", ExactSpelling = true, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool StretchBlt (
            IntPtr destination,
            int destinationX,
            int destinationY,
            int destinationWidth,
            int destinationHeight,
            IntPtr source,
            int sourceX,
            int sourceY,
            int sourceWidth,
            int sourceHeight,
            uint operation);

        [DllImport("gdi32.dll", ExactSpelling = true)]
        private static extern int SetStretchBltMode (IntPtr dc, int mode);

    }

}
