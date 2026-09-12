using System;
using System.ComponentModel;
using System.Diagnostics;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    /// <summary>
    /// Captures the Explorer desktop surface once while the switcher is
    /// hidden.  The source is the shell's desktop view, rather than the
    /// screen DC, so ordinary application windows are never copied into the
    /// switcher's background.
    /// </summary>
    public sealed class ShellDesktopSnapshot : IDisposable {

        private IShellDesktopSnapshotFrame frame;
        private bool disposed;

        public ShellDesktopSnapshot (Rectangle sourceBounds)
            : this(sourceBounds, new GdiShellDesktopSnapshotApi()) {
        }

        public ShellDesktopSnapshot (Rectangle sourceBounds, IShellDesktopSnapshotApi api) {
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
                // Explorer can be restarting, unavailable on a secure/RDP
                // desktop, or have no desktop view.  Keep the hotkey gesture
                // fail-open with an opaque black backdrop.
                Trace.WriteLine("Shell desktop snapshot capture failed: " + exception);
            }
        }

        public bool IsAvailable => frame != null;

        public void Draw (Graphics graphics, Rectangle destinationBounds) {
            if( disposed ) {
                throw new ObjectDisposedException(nameof(ShellDesktopSnapshot));
            }
            if( graphics == null ) {
                throw new ArgumentNullException(nameof(graphics));
            }

            IShellDesktopSnapshotFrame currentFrame = frame;
            if( currentFrame == null ||
                destinationBounds.Width <= 0 || destinationBounds.Height <= 0 ) {
                graphics.Clear(Color.Black);
                return;
            }

            try {
                currentFrame.Draw(graphics, destinationBounds);
            }
            catch( Exception exception ) {
                // A lost shell surface must not escape WM_PAINT.
                Trace.WriteLine("Shell desktop snapshot paint failed: " + exception);
                graphics.Clear(Color.Black);
            }
        }

        public void Dispose () {
            if( disposed ) {
                return;
            }

            disposed = true;
            IShellDesktopSnapshotFrame currentFrame = frame;
            frame = null;
            try {
                currentFrame?.Dispose();
            }
            catch( Exception exception ) {
                Trace.WriteLine("Shell desktop snapshot cleanup failed: " + exception);
            }
            GC.SuppressFinalize(this);
        }

    }

    public interface IShellDesktopSnapshotApi {

        IShellDesktopSnapshotFrame Capture (Rectangle sourceBounds);

    }

    public interface IShellDesktopSnapshotFrame : IDisposable {

        Size Size { get; }

        void Draw (Graphics graphics, Rectangle destinationBounds);

    }

    /// <summary>
    /// Asks Explorer's top-level desktop host to render its complete content
    /// into an off-screen bitmap, then retains a top-down DIB for inexpensive
    /// painting. It deliberately never captures the screen: that would copy
    /// whichever applications happen to cover the wallpaper.
    /// </summary>
    public sealed class GdiShellDesktopSnapshotApi : IShellDesktopSnapshotApi {

        public IShellDesktopSnapshotFrame Capture (Rectangle sourceBounds) =>
            new GdiShellDesktopSnapshotFrame(sourceBounds);

    }

    public sealed class GdiShellDesktopSnapshotFrame : IShellDesktopSnapshotFrame {

        private const uint SourceCopy = 0x00cc0020;
        private const uint Blackness = 0x0042;
        private const uint PrintWindowFullContent = 0x00000002;
        private const uint DibRgbColors = 0;
        private const uint BitmapInfoHeaderSize = 40;
        private const ushort BitmapPlanes = 1;
        private const ushort BitsPerPixel = 32;
        private const int ColorOnColor = 3;

        private IntPtr memoryDc;
        private IntPtr bitmap;
        private IntPtr previousBitmap;
        private bool disposed;

        public GdiShellDesktopSnapshotFrame (Rectangle sourceBounds) {
            if( sourceBounds.Width <= 0 || sourceBounds.Height <= 0 ) {
                throw new ArgumentOutOfRangeException(nameof(sourceBounds));
            }

            Size = sourceBounds.Size;
            IntPtr shellDesktop = ShellDesktopWindowLocator.Find();
            if( shellDesktop == IntPtr.Zero ) {
                throw new InvalidOperationException("The Explorer desktop host is unavailable.");
            }

            IntPtr sourceDc = GetDC(shellDesktop);
            if( sourceDc == IntPtr.Zero ) {
                throw LastWin32Exception("GetDC failed for the Explorer desktop host.");
            }

            IntPtr newMemoryDc = IntPtr.Zero;
            IntPtr newBitmap = IntPtr.Zero;
            IntPtr newPreviousBitmap = IntPtr.Zero;
            try {
                NativeRect clientRect;
                if( !GetClientRect(shellDesktop, out clientRect) ) {
                    throw LastWin32Exception("GetClientRect failed for the Explorer desktop host.");
                }
                if( clientRect.Right <= clientRect.Left || clientRect.Bottom <= clientRect.Top ) {
                    throw new InvalidOperationException("The Explorer desktop host has stale geometry.");
                }

                NativePoint clientOrigin = new NativePoint();
                if( !ClientToScreen(shellDesktop, ref clientOrigin) ) {
                    throw LastWin32Exception("ClientToScreen failed for the Explorer desktop host.");
                }

                int clientWidth = clientRect.Right - clientRect.Left;
                int clientHeight = clientRect.Bottom - clientRect.Top;
                if( clientWidth <= 0 || clientHeight <= 0 ) {
                    throw new InvalidOperationException("The Explorer desktop host has stale geometry.");
                }

                Rectangle desktopClientBounds = new Rectangle(
                    clientOrigin.X,
                    clientOrigin.Y,
                    clientWidth,
                    clientHeight);
                Rectangle copyBounds = Rectangle.Intersect(sourceBounds, desktopClientBounds);
                if( copyBounds.Width <= 0 || copyBounds.Height <= 0 ) {
                    throw new InvalidOperationException("The Explorer desktop host does not intersect the virtual desktop.");
                }

                newMemoryDc = CreateCompatibleDC(sourceDc);
                if( newMemoryDc == IntPtr.Zero ) {
                    throw LastWin32Exception("CreateCompatibleDC failed for the shell snapshot.");
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
                    sourceDc,
                    ref bitmapInfo,
                    DibRgbColors,
                    out bits,
                    IntPtr.Zero,
                    0);
                if( newBitmap == IntPtr.Zero ) {
                    throw LastWin32Exception("CreateDIBSection failed for the shell snapshot.");
                }
                if( bits == IntPtr.Zero ) {
                    throw new InvalidOperationException("CreateDIBSection returned no writable shell-snapshot pixels.");
                }

                newPreviousBitmap = SelectObject(newMemoryDc, newBitmap);
                if( newPreviousBitmap == IntPtr.Zero || newPreviousBitmap == new IntPtr(-1) ) {
                    throw LastWin32Exception("SelectObject failed for the shell snapshot.");
                }

                // The shell view can be smaller than a disconnected monitor,
                // or temporarily report stale bounds during an Explorer
                // restart.  Initialize uncovered portions to black instead
                // of ever reading from a screen DC.
                if( !PatBlt(newMemoryDc, 0, 0, sourceBounds.Width, sourceBounds.Height, Blackness) ) {
                    throw LastWin32Exception("PatBlt failed for the shell snapshot.");
                }

                int sourceX = copyBounds.Left - desktopClientBounds.Left;
                int sourceY = copyBounds.Top - desktopClientBounds.Top;
                int destinationX = copyBounds.Left - sourceBounds.Left;
                int destinationY = copyBounds.Top - sourceBounds.Top;

                // PrintWindow(PW_RENDERFULLCONTENT) asks Explorer to render
                // the shell host and its desktop-view children into an
                // off-screen surface.  Its ordinary mode, DWM thumbnails,
                // and a plain window-DC BitBlt can all expose the applications
                // currently covering the desktop, so none is a valid fallback.
                IntPtr printedDc = IntPtr.Zero;
                IntPtr printedBitmap = IntPtr.Zero;
                IntPtr printedPreviousBitmap = IntPtr.Zero;
                try {
                    printedDc = CreateCompatibleDC(sourceDc);
                    if( printedDc == IntPtr.Zero ) {
                        throw LastWin32Exception("CreateCompatibleDC failed for the shell print surface.");
                    }
                    printedBitmap = CreateCompatibleBitmap(sourceDc, clientWidth, clientHeight);
                    if( printedBitmap == IntPtr.Zero ) {
                        throw LastWin32Exception("CreateCompatibleBitmap failed for the shell print surface.");
                    }
                    printedPreviousBitmap = SelectObject(printedDc, printedBitmap);
                    if( printedPreviousBitmap == IntPtr.Zero || printedPreviousBitmap == new IntPtr(-1) ) {
                        throw LastWin32Exception("SelectObject failed for the shell print surface.");
                    }
                    if( !PatBlt(printedDc, 0, 0, clientWidth, clientHeight, Blackness) ) {
                        throw LastWin32Exception("PatBlt failed for the shell print surface.");
                    }
                    if( !PrintWindow(shellDesktop, printedDc, PrintWindowFullContent) ) {
                        throw LastWin32Exception("PrintWindow failed for the Explorer desktop host.");
                    }
                    if( !BitBlt(
                        newMemoryDc,
                        destinationX,
                        destinationY,
                        copyBounds.Width,
                        copyBounds.Height,
                        printedDc,
                        sourceX,
                        sourceY,
                        SourceCopy) ) {
                        throw LastWin32Exception("BitBlt failed while mapping the shell print surface.");
                    }
                }
                finally {
                    Release(ref printedDc, ref printedBitmap, ref printedPreviousBitmap);
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
                int released = ReleaseDC(shellDesktop, sourceDc);
                if( released == 0 ) {
                    Trace.WriteLine("ReleaseDC failed for the Explorer desktop host.");
                }
            }
        }

        public Size Size { get; }

        ~GdiShellDesktopSnapshotFrame () {
            Dispose(false);
        }

        public void Draw (Graphics graphics, Rectangle destinationBounds) {
            if( disposed ) {
                throw new ObjectDisposedException(nameof(GdiShellDesktopSnapshotFrame));
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
                    throw LastWin32Exception("BitBlt failed while painting the shell snapshot.");
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
            bool bitmapDeselected = true;
            if( currentMemoryDc != IntPtr.Zero &&
                currentPreviousBitmap != IntPtr.Zero &&
                currentPreviousBitmap != new IntPtr(-1) ) {
                IntPtr restored = SelectObject(currentMemoryDc, currentPreviousBitmap);
                bitmapDeselected = restored != IntPtr.Zero && restored != new IntPtr(-1);
                if( !bitmapDeselected ) {
                    Trace.WriteLine("SelectObject failed while releasing the shell snapshot.");
                }
            }
            currentPreviousBitmap = IntPtr.Zero;

            if( currentBitmap != IntPtr.Zero && bitmapDeselected ) {
                if( !DeleteObject(currentBitmap) ) {
                    Trace.WriteLine("DeleteObject failed while releasing the shell snapshot.");
                }
                currentBitmap = IntPtr.Zero;
            }
            if( currentMemoryDc != IntPtr.Zero ) {
                if( !DeleteDC(currentMemoryDc) ) {
                    Trace.WriteLine("DeleteDC failed while releasing the shell snapshot.");
                }
                currentMemoryDc = IntPtr.Zero;
            }
            if( currentBitmap != IntPtr.Zero ) {
                // A selected bitmap cannot be deleted. If restoring the
                // previous object failed, destroying the memory DC releases
                // that selection so deletion can be retried safely.
                if( !DeleteObject(currentBitmap) ) {
                    Trace.WriteLine("DeleteObject failed after releasing the shell snapshot DC.");
                }
                currentBitmap = IntPtr.Zero;
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

        [StructLayout(LayoutKind.Sequential)]
        private struct NativePoint {

            public int X;
            public int Y;

        }

        [StructLayout(LayoutKind.Sequential)]
        private struct NativeRect {

            public int Left;
            public int Top;
            public int Right;
            public int Bottom;

        }

        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
        private static extern IntPtr GetDC (IntPtr window);

        [DllImport("user32.dll", ExactSpelling = true)]
        private static extern int ReleaseDC (IntPtr window, IntPtr dc);

        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
        private static extern bool GetClientRect (IntPtr window, out NativeRect rectangle);

        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
        private static extern bool ClientToScreen (IntPtr window, ref NativePoint point);

        [DllImport("gdi32.dll", ExactSpelling = true, SetLastError = true)]
        private static extern IntPtr CreateCompatibleDC (IntPtr dc);

        [DllImport("gdi32.dll", ExactSpelling = true, SetLastError = true)]
        private static extern IntPtr CreateCompatibleBitmap (IntPtr dc, int width, int height);

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

        [DllImport("gdi32.dll", ExactSpelling = true, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool PatBlt (
            IntPtr destination,
            int destinationX,
            int destinationY,
            int width,
            int height,
            uint operation);

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

        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool PrintWindow (IntPtr window, IntPtr dc, uint flags);

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

    internal static class ShellDesktopWindowLocator {

        public static IntPtr Find () {
            IntPtr shell = GetShellWindow();
            IntPtr desktopView = FindDesktopView(shell);
            if( desktopView != IntPtr.Zero ) {
                return shell;
            }

            IntPtr worker = IntPtr.Zero;
            while( true ) {
                worker = FindWindowEx(IntPtr.Zero, worker, "WorkerW", null);
                if( worker == IntPtr.Zero ) {
                    return IntPtr.Zero;
                }

                desktopView = FindDesktopView(worker);
                if( desktopView != IntPtr.Zero ) {
                    return worker;
                }
            }
        }

        private static IntPtr FindDesktopView (IntPtr parent) {
            if( parent == IntPtr.Zero ) {
                return IntPtr.Zero;
            }
            IntPtr desktopView = FindWindowEx(parent, IntPtr.Zero, "SHELLDLL_DefView", null);
            if( desktopView == IntPtr.Zero ) {
                return IntPtr.Zero;
            }

            // Requiring Explorer's icon list prevents an unrelated window
            // with the same intermediate class from becoming a capture source.
            IntPtr iconView = FindWindowEx(desktopView, IntPtr.Zero, "SysListView32", null);
            return iconView != IntPtr.Zero ? desktopView : IntPtr.Zero;
        }

        [DllImport("user32.dll", ExactSpelling = true)]
        private static extern IntPtr GetShellWindow ();

        [DllImport("user32.dll", EntryPoint = "FindWindowExW", CharSet = CharSet.Unicode, ExactSpelling = true)]
        private static extern IntPtr FindWindowEx (
            IntPtr parent,
            IntPtr after,
            string className,
            string windowName);

    }

}
