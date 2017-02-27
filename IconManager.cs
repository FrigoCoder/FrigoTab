using System;
using System.Collections.Generic;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public static class IconManager {

        public static void Register (ApplicationWindow window, WindowHandle application) {
            _registered.Add(window);
            IntPtr handle = GCHandle.ToIntPtr(GCHandle.Alloc(window));
            SendMessageCallback(application, WindowMessages.GetIcon, GetIconSize.Big, (IntPtr) 0, Callback, handle);
        }

        public static void Unregister (ApplicationWindow window) {
            _registered.Remove(window);
        }

        public static Icon IconFromGetClassLongPtr (WindowHandle handle) {
            IntPtr icon = GetClassLongPtr(handle, ClassLong.Icon);
            return icon != IntPtr.Zero ? Icon.FromHandle(icon) : null;
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

        private delegate void SendMessageDelegate (IntPtr hWnd, int msg, IntPtr dwData, IntPtr lResult);

        private static readonly IList<ApplicationWindow> _registered = new List<ApplicationWindow>();

        private static void Callback (IntPtr hWnd, int msg, IntPtr dwData, IntPtr lResult) {
            GCHandle handle = GCHandle.FromIntPtr(dwData);
            ApplicationWindow window = (ApplicationWindow) handle.Target;
            handle.Free();
            if( _registered.Contains(window) ) {
                if( lResult != IntPtr.Zero ) {
                    window.AppIcon = Icon.FromHandle(lResult);
                }
            }
            Unregister(window);
        }

        [DllImport ("user32.dll")]
        private static extern IntPtr GetClassLongPtr (IntPtr hWnd, ClassLong nIndex);

        [DllImport ("user32.dll")]
        private static extern bool SendMessageCallback (IntPtr hWnd,
            WindowMessages message,
            GetIconSize wParam,
            IntPtr lParam,
            SendMessageDelegate lpCallBack,
            IntPtr dwData);

    }

}
