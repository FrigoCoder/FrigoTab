using System;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Text;
using System.Windows.Forms;

namespace FrigoTab {

    [Flags]
    public enum WindowStyles : long {

        Disabled = 0x8000000,
        Visible = 0x10000000,
        Minimize = 0x20000000

    }

    [Flags]
    public enum WindowExStyles : long {

        Transparent = 0x20,
        ToolWindow = 0x80,
        AppWindow = 0x40000,
        Layered = 0x80000,
        NoActivate = 0x8000000

    }

    public struct WindowHandle {

        public static readonly WindowHandle Null = new WindowHandle(IntPtr.Zero);
        public static bool operator == (WindowHandle h1, WindowHandle h2) => h1.handle == h2.handle;
        public static bool operator != (WindowHandle h1, WindowHandle h2) => h1.handle != h2.handle;

        [DllImport("user32.dll")]
        public static extern WindowHandle GetForegroundWindow ();

        private readonly IntPtr handle;

        public WindowHandle (IntPtr handle) => this.handle = handle;
        public override bool Equals (object obj) => obj != null && GetType() == obj.GetType() && handle == ((WindowHandle) obj).handle;
        public override int GetHashCode () => handle.GetHashCode();
        public Screen GetScreen () => Screen.FromHandle(handle);
        public WindowStyles GetWindowStyles () => (WindowStyles) GetWindowLongPtr(this, WindowLong.Style);
        public WindowExStyles GetWindowExStyles () => (WindowExStyles) GetWindowLongPtr(this, WindowLong.ExStyle);
        public void PostMessage (WindowMessages msg, int wParam, int lParam) => PostMessage(this, msg, (IntPtr) wParam, (IntPtr) lParam);

        public bool SetForeground () {
            if( GetWindowStyles().HasFlag(WindowStyles.Minimize) ) {
                ShowWindow(this, ShowWindowCommand.Restore);
            }

            // FrigoTab historically used this input-queue-independent nudge
            // after AttachThreadInput caused focus and key-state corruption.
            // It gives SetForegroundWindow the same recent-input context as
            // the intercepted gesture without joining another process's input
            // thread or synthesizing a real Alt transition.
            keybd_event(0, 0, 0, UIntPtr.Zero);
            return SetForegroundWindow(this);
        }

        public string GetWindowText () {
            StringBuilder text = new StringBuilder(GetWindowTextLength(this) + 1);
            GetWindowText(this, text, text.Capacity);
            return text.ToString();
        }

        public Rect GetRect () {
            Rectangle rectangle;
            if( !TryGetRect(out rectangle) ) {
                throw new ArgumentException("The window handle no longer has valid screen geometry.", nameof(handle));
            }
            return new Rect(rectangle);
        }

        /// <summary>
        /// Gets the restored/current screen rectangle for a window.
        ///
        /// A window can disappear between enumeration and layout. Native BOOL
        /// results are checked and invalid geometry is reported to the caller
        /// instead of being converted into a partially valid candidate.
        /// </summary>
        public bool TryGetRect (out Rectangle rectangle) {
            rectangle = Rectangle.Empty;
            WindowPlacement placement = new WindowPlacement {
                Length = Marshal.SizeOf<WindowPlacement>()
            };
            if( !GetWindowPlacement(this, ref placement) ) {
                return false;
            }

            NativeRect nativeRect;
            switch( placement.ShowCmd ) {
                case ShowWindowCommand.ShowNormal:
                case ShowWindowCommand.ShowMinimized:
                    nativeRect = placement.NormalPosition;
                    break;
                case ShowWindowCommand.ShowMaximized:
                    if( !GetWindowRect(this, out nativeRect) ) {
                        return false;
                    }
                    break;
                default:
                    return false;
            }

            rectangle = nativeRect.ToRectangle();
            return rectangle.Width > 0 && rectangle.Height > 0;
        }

        private struct WindowPlacement {

            public int Length;
            public int Flags;
            public ShowWindowCommand ShowCmd;
            public Point MinPosition;
            public Point MaxPosition;
            public NativeRect NormalPosition;

        }

        [StructLayout(LayoutKind.Sequential)]
        private struct NativeRect {

            public int Left;
            public int Top;
            public int Right;
            public int Bottom;

            public Rectangle ToRectangle () => Rectangle.FromLTRB(Left, Top, Right, Bottom);

        }

        private enum ShowWindowCommand {

            ShowNormal = 1,
            ShowMinimized = 2,
            ShowMaximized = 3,
            Restore = 9

        }

        private enum WindowLong {

            ExStyle = -20,
            Style = -16

        }

        [DllImport("user32.dll", EntryPoint = "GetWindowTextLengthW", CharSet = CharSet.Unicode,
            ExactSpelling = true, SetLastError = true)]
        private static extern int GetWindowTextLength (WindowHandle hWnd);

        [DllImport("user32.dll", EntryPoint = "GetWindowTextW", CharSet = CharSet.Unicode,
            ExactSpelling = true, SetLastError = true)]
        private static extern int GetWindowText (WindowHandle hWnd, StringBuilder lpString, int nMaxCount);

        [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW", CharSet = CharSet.Unicode,
            ExactSpelling = true, SetLastError = true)]
        private static extern IntPtr GetWindowLongPtr (WindowHandle hWnd, WindowLong nIndex);

        [DllImport("user32.dll")]
        private static extern bool ShowWindow (WindowHandle hWnd, ShowWindowCommand nCmdShow);

        [DllImport("user32.dll")]
        private static extern bool SetForegroundWindow (WindowHandle hWnd);

        [DllImport("user32.dll", ExactSpelling = true)]
        private static extern void keybd_event (byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);

        [DllImport("user32.dll", EntryPoint = "PostMessageW", CharSet = CharSet.Unicode,
            ExactSpelling = true, SetLastError = true)]
        private static extern bool PostMessage (WindowHandle hWnd, WindowMessages msg, IntPtr wParam, IntPtr lParam);

        [DllImport("user32.dll")]
        private static extern bool GetWindowRect (WindowHandle hWnd, out NativeRect rect);

        [DllImport("user32.dll")]
        private static extern bool GetWindowPlacement (WindowHandle hWnd, ref WindowPlacement lpwndpl);

    }

}
