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

        public static WindowHandle GetForegroundWindowHandle () {
            return GetForegroundWindow();
        }

        public static implicit operator WindowHandle (IntPtr handle) {
            return new WindowHandle(handle);
        }

        public static implicit operator IntPtr (WindowHandle handle) {
            return handle._handle;
        }

        private readonly IntPtr _handle;

        private WindowHandle (IntPtr handle) {
            _handle = handle;
        }

        public Screen GetScreen () {
            return Screen.FromHandle(_handle);
        }

        public void SetForeground () {
            if( GetWindowStyles().HasFlag(WindowStyles.Minimize) ) {
                ShowWindow(_handle, ShowWindowCommand.Restore);
            }

            byte[] keyState = new byte[256];
            GetKeyboardState(keyState);
            bool altPressed = (keyState[(int) Keys.Menu] & 0x80) != 0;

            if( !altPressed ) {
                keybd_event((int) Keys.Menu, 0, (int) KeyEventF.ExtendedKey, 0);
            }

            SetForegroundWindow(_handle);

            if( !altPressed ) {
                keybd_event((int) Keys.Menu, 0, (int) (KeyEventF.ExtendedKey | KeyEventF.KeyUp), 0);
            }
        }

        public string GetWindowText () {
            StringBuilder text = new StringBuilder(GetWindowTextLength(_handle) + 1);
            GetWindowText(_handle, text, text.Capacity);
            return text.ToString();
        }

        public WindowStyles GetWindowStyles () {
            return (WindowStyles) GetWindowLongPtr(_handle, WindowLong.Style);
        }

        public WindowExStyles GetWindowExStyles () {
            return (WindowExStyles) GetWindowLongPtr(_handle, WindowLong.ExStyle);
        }

        public Icon IconFromGetClassLongPtr () {
            IntPtr icon = GetClassLongPtr(_handle, ClassLong.Icon);
            return icon == IntPtr.Zero ? null : Icon.FromHandle(icon);
        }

        public void RegisterIconCallback (Action<Icon> action) {
            IntPtr actionHandle = GCHandle.ToIntPtr(GCHandle.Alloc(action));
            SendMessageCallback(_handle, WindowMessages.GetIcon, GetIconSize.Big, (IntPtr) 0, _callback, actionHandle);
        }

        public void SendMessage (WindowMessages msg, int wParam, int lParam) {
            SendMessage(_handle, msg, (IntPtr) wParam, (IntPtr) lParam);
        }

        public void PostMessage (WindowMessages msg, int wParam, int lParam) {
            PostMessage(_handle, msg, (IntPtr) wParam, (IntPtr) lParam);
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

        [Flags]
        private enum KeyEventF {

            ExtendedKey = 1,
            KeyUp = 2

        }

        private delegate void SendMessageDelegate (IntPtr hWnd, int msg, IntPtr dwData, IntPtr lResult);

        private static readonly SendMessageDelegate _callback = Callback;

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
        private static extern bool SendMessageCallback (IntPtr hWnd,
            WindowMessages message,
            GetIconSize wParam,
            IntPtr lParam,
            SendMessageDelegate lpCallBack,
            IntPtr dwData);

        [DllImport("user32.dll")]
        private static extern bool GetKeyboardState (byte[] lpKeyState);

        [DllImport("user32.dll")]
        private static extern void keybd_event (byte bVk, byte bScan, int dwFlags, int dwExtraInfo);

        [DllImport("user32.dll")]
        private static extern bool SendMessage (IntPtr hWnd, WindowMessages msg, IntPtr wParam, IntPtr lParam);

        [DllImport("user32.dll")]
        private static extern bool PostMessage (IntPtr hWnd, WindowMessages msg, IntPtr wParam, IntPtr lParam);

        [DllImport("user32.dll")]
        private static extern IntPtr GetForegroundWindow ();

    }

}
