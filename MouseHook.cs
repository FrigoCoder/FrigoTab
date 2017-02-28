using System;
using System.Diagnostics;
using System.Diagnostics.CodeAnalysis;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class MouseHook {

        [SuppressMessage ("ReSharper", "PrivateFieldCanBeConvertedToLocalVariable")]
        private readonly LowLevelMouseProc _hookProc;

        private readonly IntPtr _hookId;

        public event Action<MouseHookEventArgs> MouseEvent;

        public MouseHook () {
            _hookProc = HookProc;
            using( Process curProcess = Process.GetCurrentProcess() ) {
                using( ProcessModule curModule = curProcess.MainModule ) {
                    _hookId = SetWindowsHookEx(14, _hookProc, GetModuleHandle(curModule.ModuleName), 0);
                }
            }
        }

        public void Dispose () {
            UnhookWindowsHookEx(_hookId);
        }

        private IntPtr HookProc (int nCode, IntPtr wParam, ref LowLevelMouseStruct lParam) {
            if( nCode >= 0 ) {
            }
            return CallNextHookEx(_hookId, nCode, wParam, ref lParam);
        }

        private struct LowLevelMouseStruct {

            public Point Point;
            public int MouseData;
            public int Flags;
            public int Time;
            public IntPtr DwExtraInfo;

        }

        private enum Wm {

            MouseMove = 0x200,
            LButtonDown = 0x201,
            LButtonUp = 0x202,
            RButtonDown = 0x204,
            RButtonUp = 0x205,
            MouseWheel = 0x20a,
            MouseHWheel = 0x20e

        }

        private delegate IntPtr LowLevelMouseProc (int nCode, IntPtr wParam, ref LowLevelMouseStruct lParam);

        [DllImport ("kernel32.dll")]
        private static extern IntPtr GetModuleHandle (string lpModuleName);

        [DllImport ("user32.dll")]
        private static extern IntPtr SetWindowsHookEx (int idHook, LowLevelMouseProc lpfn, IntPtr hMod, int dwThreadId);

        [DllImport ("user32.dll")]
        private static extern bool UnhookWindowsHookEx (IntPtr hhk);

        [DllImport ("user32.dll")]
        private static extern IntPtr CallNextHookEx (IntPtr hhk,
            int nCode,
            IntPtr wParam,
            ref LowLevelMouseStruct lParam);

    }

}
