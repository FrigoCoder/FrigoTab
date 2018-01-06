using System;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class WindowIcon {

        public event Action Changed;
        public Icon Icon;

        public WindowIcon (WindowHandle handle) {
            Icon = IconFromGetClassLongPtr(handle) ?? Program.Icon;
            RegisterIconCallback(handle, icon => {
                Icon = icon;
                Changed?.Invoke();
            });
        }

        private Icon IconFromGetClassLongPtr (WindowHandle handle) {
            IntPtr icon = GetClassLongPtr(handle, ClassLong.Icon);
            return icon == IntPtr.Zero ? null : Icon.FromHandle(icon);
        }

        private void RegisterIconCallback (WindowHandle handle, Action<Icon> action) =>
            SendMessageCallback(handle, WindowMessages.GetIcon, GetIconSize.Big, (IntPtr) 0, CallbackDelegate,
                GCHandle.ToIntPtr(GCHandle.Alloc(action)));

        private enum ClassLong {

            Icon = -14

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
        private static extern IntPtr GetClassLongPtr (WindowHandle hWnd, ClassLong nIndex);

        [DllImport("user32.dll")]
        private static extern bool SendMessageCallback (WindowHandle hWnd, WindowMessages message, GetIconSize wParam, IntPtr lParam,
            SendMessageDelegate lpCallBack, IntPtr dwData);

    }

}
