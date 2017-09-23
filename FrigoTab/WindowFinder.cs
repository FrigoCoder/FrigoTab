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

        private bool EnumWindowCallback (IntPtr handle, IntPtr lParam) {
            switch( GetWindowType(handle) ) {
                case WindowType.AppWindow:
                    Windows.Add(handle);
                    break;
                case WindowType.ToolWindow:
                    ToolWindows.Add(handle);
                    break;
                case WindowType.Hidden:
                    break;
                default:
                    throw new ArgumentOutOfRangeException();
            }
            return true;
        }

        private enum WindowType {

            Hidden,
            AppWindow,
            ToolWindow

        }

        private delegate bool EnumWindowsProc (IntPtr handle, IntPtr lParam);

        private static WindowType GetWindowType (WindowHandle handle) {
            if( handle.GetWindowRect().IsEmpty ) {
                return WindowType.Hidden;
            }

            if( Dwm.IsCloaked(handle) ) {
                return WindowType.Hidden;
            }

            WindowStyles style = handle.GetWindowStyles();
            if( style.HasFlag(WindowStyles.Disabled) ) {
                return WindowType.Hidden;
            }
            if( !style.HasFlag(WindowStyles.Visible) ) {
                return WindowType.Hidden;
            }

            WindowExStyles ex = handle.GetWindowExStyles();
            if( ex.HasFlag(WindowExStyles.NoActivate) ) {
                return WindowType.Hidden;
            }
            if( ex.HasFlag(WindowExStyles.AppWindow) ) {
                return WindowType.AppWindow;
            }
            if( ex.HasFlag(WindowExStyles.ToolWindow) ) {
                return WindowType.ToolWindow;
            }

            return WindowType.AppWindow;
        }

        [DllImport("user32.dll")]
        private static extern bool EnumWindows (EnumWindowsProc enumFunc, IntPtr lParam);

    }

}
