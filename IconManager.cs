using System;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public static class IconManager {

        public static void RegisterIconCallback (WindowHandle window, Action<Icon> callback) {
            IntPtr handle = GCHandle.ToIntPtr(GCHandle.Alloc(callback));
            SendMessageCallback(window, WindowMessages.GetIcon, GetIconSize.Big, (IntPtr) 0, _callback, handle);
        }

        private enum WindowMessages {

            GetIcon = 127

        }

        private enum GetIconSize {

            Big = 1

        }

        private delegate void SendMessageDelegate (IntPtr hWnd, int msg, IntPtr dwData, IntPtr lResult);

        private static readonly SendMessageDelegate _callback = Callback;

        private static void Callback (IntPtr hWnd, int msg, IntPtr dwData, IntPtr lResult) {
            GCHandle handle = GCHandle.FromIntPtr(dwData);
            Action<Icon> callback = (Action<Icon>) handle.Target;
            handle.Free();

            if( lResult != IntPtr.Zero ) {
                callback(Icon.FromHandle(lResult));
            }
        }

        [DllImport ("user32.dll")]
        private static extern bool SendMessageCallback (IntPtr hWnd,
            WindowMessages message,
            GetIconSize wParam,
            IntPtr lParam,
            SendMessageDelegate lpCallBack,
            IntPtr dwData);

    }

}
