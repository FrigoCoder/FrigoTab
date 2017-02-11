using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class WindowFinder {

        public readonly IList<WindowHandle> Windows = new List<WindowHandle>();
        public readonly IList<WindowHandle> ToolWindows = new List<WindowHandle>();

        public WindowFinder () {
            EnumWindows(EnumWindowCallback, IntPtr.Zero);
        }

        private bool EnumWindowCallback (WindowHandle handle, IntPtr lParam) {
            if( IsWindow(handle) ) {
                Windows.Add(handle);
            }
            if( IsToolWindow(handle) ) {
                ToolWindows.Add(handle);
            }
            return true;
        }

        private delegate bool EnumWindowsProc (WindowHandle handle, IntPtr lParam);

        private static bool IsWindow (WindowHandle handle) {
            WindowStyles style = handle.GetWindowStyles();
            if( style.HasFlag(WindowStyles.Disabled) ) {
                return false;
            }
            if( !style.HasFlag(WindowStyles.Visible) ) {
                return false;
            }

            WindowExStyles ex = handle.GetWindowExStyles();
            if( ex.HasFlag(WindowExStyles.NoActivate) ) {
                return false;
            }
            if( ex.HasFlag(WindowExStyles.AppWindow) ) {
                return true;
            }
            if( ex.HasFlag(WindowExStyles.ToolWindow) ) {
                return false;
            }
            return true;
        }

        private static bool IsToolWindow (WindowHandle handle) {
            WindowStyles style = handle.GetWindowStyles();
            if( style.HasFlag(WindowStyles.Disabled) ) {
                return false;
            }
            if( !style.HasFlag(WindowStyles.Visible) ) {
                return false;
            }

            WindowExStyles ex = handle.GetWindowExStyles();
            if( ex.HasFlag(WindowExStyles.NoActivate) ) {
                return false;
            }
            if( ex.HasFlag(WindowExStyles.AppWindow) ) {
                return false;
            }
            if( ex.HasFlag(WindowExStyles.ToolWindow) ) {
                return true;
            }
            return false;
        }

        [DllImport ("user32.dll")]
        private static extern bool EnumWindows (EnumWindowsProc enumFunc, IntPtr lParam);

    }

}
