using System;
using System.Diagnostics.CodeAnalysis;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class WindowIcon {

        public event Action Changed;
        public Icon Icon;

        [SuppressMessage("ReSharper", "PrivateFieldCanBeConvertedToLocalVariable")]
        private readonly SendMessageDelegate callback;

        public WindowIcon (WindowHandle handle) {
            IntPtr icon = GetClassLongPtr(handle, ClassLong.Icon);
            Icon = icon == IntPtr.Zero ? Program.Icon : Icon.FromHandle(icon);
            callback = Callback;
            SendMessageCallback(handle, WindowMessages.GetIcon, GetIconSize.Big, (IntPtr) 0, callback, (IntPtr) 0);
        }

        private void Callback (WindowHandle hWnd, int msg, IntPtr dwData, IntPtr lResult) {
            if( lResult == IntPtr.Zero ) {
                return;
            }
            Icon = Icon.FromHandle(lResult);
            Changed?.Invoke();
        }

        private enum ClassLong {

            Icon = -14

        }

        private enum GetIconSize {

            Big = 1

        }

        private delegate void SendMessageDelegate (WindowHandle hWnd, int msg, IntPtr dwData, IntPtr lResult);

        [DllImport("user32.dll")]
        private static extern IntPtr GetClassLongPtr (WindowHandle hWnd, ClassLong nIndex);

        [DllImport("user32.dll")]
        private static extern bool SendMessageCallback (WindowHandle hWnd, WindowMessages message, GetIconSize wParam, IntPtr lParam,
            SendMessageDelegate lpCallBack, IntPtr dwData);

    }

}
