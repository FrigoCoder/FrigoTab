using System;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public static class IconManager {

        public delegate void SendMessageDelegate (IntPtr hWnd, int msg, IntPtr dwData, IntPtr lResult);

        public static Icon IconFromGetClassLongPtr (WindowHandle handle) {
            IntPtr icon = GetClassLongPtr(handle, ClassLong.Icon);
            return icon != IntPtr.Zero ? Icon.FromHandle(icon) : null;
        }

        public static Icon IconFromSendMessageTimeout (WindowHandle handle) {
            IntPtr icon;
            SendMessageTimeout(handle,
                WindowMessages.GetIcon,
                (IntPtr) GetIconSize.Big,
                (IntPtr) 0,
                SendMessageTimeoutFlags.AbortIfHung | SendMessageTimeoutFlags.Block,
                500,
                out icon);
            return icon != IntPtr.Zero ? Icon.FromHandle(icon) : null;
        }

        public static void IconFromSendMessageCallback (WindowHandle handle, SendMessageDelegate callback) {
            SendMessageCallback(handle, WindowMessages.GetIcon, (IntPtr) GetIconSize.Big, (IntPtr) 0, callback, 0);
        }

        private enum ClassLong {

            Icon = -14

        }

        private enum WindowMessages {

            GetIcon = 127

        }

        private enum GetIconSize {

            Big = 1

        }

        [Flags]
        private enum SendMessageTimeoutFlags {

            Block = 1,
            AbortIfHung = 2

        }

        [DllImport ("user32.dll")]
        private static extern IntPtr GetClassLongPtr (IntPtr hWnd, ClassLong nIndex);

        [DllImport ("user32.dll")]
        private static extern IntPtr SendMessageTimeout (IntPtr hwnd,
            WindowMessages message,
            IntPtr wparam,
            IntPtr lparam,
            SendMessageTimeoutFlags flags,
            int timeout,
            out IntPtr result);

        [DllImport ("user32.dll")]
        private static extern bool SendMessageCallback (IntPtr hWnd,
            WindowMessages message,
            IntPtr wParam,
            IntPtr lParam,
            SendMessageDelegate lpCallBack,
            long dwData);

    }

}
