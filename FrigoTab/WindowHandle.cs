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

    public class WindowHandle {

        public static WindowHandle GetForegroundWindowHandle () => GetForegroundWindow();
        public static implicit operator WindowHandle (IntPtr handle) => new WindowHandle(handle);
        public static implicit operator IntPtr (WindowHandle handle) => handle.handle;
        public static bool operator == (WindowHandle h1, WindowHandle h2) => h1 == null ? h2 == null : h1.handle == h2.handle;
        public static bool operator != (WindowHandle h1, WindowHandle h2) => !(h1 == h2);

        private readonly IntPtr handle;

        private WindowHandle (IntPtr handle) {
            this.handle = handle;
        }

        public override bool Equals (object obj) => obj != null && GetType() == obj.GetType() && handle == ((WindowHandle) obj).handle;
        public override int GetHashCode () => handle.GetHashCode();
        public Screen GetScreen () => Screen.FromHandle(handle);
        public WindowStyles GetWindowStyles () => (WindowStyles) GetWindowLongPtr(handle, WindowLong.Style);
        public WindowExStyles GetWindowExStyles () => (WindowExStyles) GetWindowLongPtr(handle, WindowLong.ExStyle);

        public void SetForeground () {
            if( GetWindowStyles().HasFlag(WindowStyles.Minimize) ) {
                ShowWindow(handle, ShowWindowCommand.Restore);
            }
            keybd_event(0, 0, 0, 0);
            SetForegroundWindow(handle);
        }

        public Rect GetWindowRect () {
            GetWindowRect(handle, out Rect rect);
            return rect;
        }

        public string GetWindowText () {
            StringBuilder text = new StringBuilder(GetWindowTextLength(handle) + 1);
            GetWindowText(handle, text, text.Capacity);
            return text.ToString();
        }

        public Icon IconFromGetClassLongPtr () {
            IntPtr icon = GetClassLongPtr(handle, ClassLong.Icon);
            return icon == IntPtr.Zero ? null : Icon.FromHandle(icon);
        }

        public void RegisterIconCallback (Action<Icon> action) {
            IntPtr actionHandle = GCHandle.ToIntPtr(GCHandle.Alloc(action));
            SendMessageCallback(handle, WindowMessages.GetIcon, GetIconSize.Big, (IntPtr) 0, CallbackDelegate, actionHandle);
        }

        public void PostMessage (WindowMessages msg, int wParam, int lParam) {
            PostMessage(handle, msg, (IntPtr) wParam, (IntPtr) lParam);
        }

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

        private delegate void SendMessageDelegate (IntPtr hWnd, int msg, IntPtr dwData, IntPtr lResult);

        private static readonly SendMessageDelegate CallbackDelegate = Callback;

        private static void Callback (IntPtr hWnd, int msg, IntPtr dwData, IntPtr lResult) {
            GCHandle handle = GCHandle.FromIntPtr(dwData);
            Action<Icon> action = (Action<Icon>) handle.Target;
            handle.Free();

            if( lResult != IntPtr.Zero ) {
                action(Icon.FromHandle(lResult));
            }
        }

        [DllImport("user32.dll")]
        private static extern int GetWindowTextLength (IntPtr hWnd);

        [DllImport("user32.dll")]
        private static extern int GetWindowText (IntPtr hWnd, StringBuilder lpString, int nMaxCount);

        [DllImport("user32.dll")]
        private static extern IntPtr GetWindowLongPtr (IntPtr hWnd, WindowLong nIndex);

        [DllImport("user32.dll")]
        private static extern bool ShowWindow (IntPtr hWnd, ShowWindowCommand nCmdShow);

        [DllImport("user32.dll")]
        private static extern bool SetForegroundWindow (IntPtr hwnd);

        [DllImport("user32.dll")]
        private static extern IntPtr GetClassLongPtr (IntPtr hWnd, ClassLong nIndex);

        [DllImport("user32.dll")]
        private static extern bool SendMessageCallback (IntPtr hWnd, WindowMessages message, GetIconSize wParam, IntPtr lParam,
            SendMessageDelegate lpCallBack, IntPtr dwData);

        [DllImport("user32.dll")]
        private static extern void keybd_event (byte bVk, byte bScan, int dwFlags, int dwExtraInfo);

        [DllImport("user32.dll")]
        private static extern bool PostMessage (IntPtr hWnd, WindowMessages msg, IntPtr wParam, IntPtr lParam);

        [DllImport("user32.dll")]
        private static extern IntPtr GetForegroundWindow ();

        [DllImport("user32.dll")]
        private static extern bool GetWindowRect (IntPtr hWnd, out Rect lpRect);

    }

}
