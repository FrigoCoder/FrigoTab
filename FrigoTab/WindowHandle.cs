using System;
using System.Diagnostics.Contracts;
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
        public WindowStyles GetWindowStyles () => (WindowStyles) GetWindowLongPtr(this, WindowLong.Style);
        public WindowExStyles GetWindowExStyles () => (WindowExStyles) GetWindowLongPtr(this, WindowLong.ExStyle);

        [Pure]
        public Screen GetScreen () => Screen.FromHandle(handle);

        public void SetForeground () {
            if( GetWindowStyles().HasFlag(WindowStyles.Minimize) ) {
                ShowWindow(this, ShowWindowCommand.Restore);
            }
            keybd_event(0, 0, 0, 0);
            SetForegroundWindow(this);
        }

        [Pure]
        public string GetWindowText () {
            StringBuilder text = new StringBuilder(GetWindowTextLength(this) + 1);
            GetWindowText(this, text, text.Capacity);
            return text.ToString();
        }

        [Pure]
        public Icon IconFromGetClassLongPtr () {
            IntPtr icon = GetClassLongPtr(this, ClassLong.Icon);
            return icon == IntPtr.Zero ? null : Icon.FromHandle(icon);
        }

        public void RegisterIconCallback (Action<Icon> action) {
            IntPtr actionHandle = GCHandle.ToIntPtr(GCHandle.Alloc(action));
            SendMessageCallback(this, WindowMessages.GetIcon, GetIconSize.Big, (IntPtr) 0, CallbackDelegate, actionHandle);
        }

        public void PostMessage (WindowMessages msg, int wParam, int lParam) => PostMessage(this, msg, (IntPtr) wParam, (IntPtr) lParam);

        private enum ClassLong {

            Icon = -14

        }

        private enum ShowWindowCommand {

            Restore = 9

        }

        private enum WindowLong {

            ExStyle = -20,
            Style = -16

        }

        private enum GetIconSize {

            Big = 1

        }

        private delegate void SendMessageDelegate (WindowHandle hWnd, int msg, IntPtr dwData, IntPtr lResult);

        private static readonly SendMessageDelegate CallbackDelegate = Callback;

        private static void Callback (WindowHandle hWnd, int msg, IntPtr dwData, IntPtr lResult) {
            GCHandle handle = GCHandle.FromIntPtr(dwData);
            Action<Icon> action = (Action<Icon>) handle.Target;
            handle.Free();

            if( lResult != IntPtr.Zero ) {
                action(Icon.FromHandle(lResult));
            }
        }

        [DllImport("user32.dll")]
        private static extern int GetWindowTextLength (WindowHandle hWnd);

        [DllImport("user32.dll")]
        private static extern int GetWindowText (WindowHandle hWnd, StringBuilder lpString, int nMaxCount);

        [DllImport("user32.dll")]
        private static extern IntPtr GetWindowLongPtr (WindowHandle hWnd, WindowLong nIndex);

        [DllImport("user32.dll")]
        private static extern bool ShowWindow (WindowHandle hWnd, ShowWindowCommand nCmdShow);

        [DllImport("user32.dll")]
        private static extern bool SetForegroundWindow (WindowHandle hWnd);

        [DllImport("user32.dll")]
        private static extern IntPtr GetClassLongPtr (WindowHandle hWnd, ClassLong nIndex);

        [DllImport("user32.dll")]
        private static extern bool SendMessageCallback (WindowHandle hWnd, WindowMessages message, GetIconSize wParam, IntPtr lParam,
            SendMessageDelegate lpCallBack, IntPtr dwData);

        [DllImport("user32.dll")]
        private static extern void keybd_event (byte bVk, byte bScan, int dwFlags, int dwExtraInfo);

        [DllImport("user32.dll")]
        private static extern bool PostMessage (WindowHandle hWnd, WindowMessages msg, IntPtr wParam, IntPtr lParam);

    }

}
