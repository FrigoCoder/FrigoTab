using System;
using System.Diagnostics;
using System.Diagnostics.CodeAnalysis;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class MouseHookEventArgs {

        public readonly Point Point;
        public readonly bool Click;
        public bool Handled;

        public MouseHookEventArgs (Point point, bool click) {
            Point = point;
            Click = click;
        }

    }

    public class MouseHook : IDisposable {

        [SuppressMessage("ReSharper", "PrivateFieldCanBeConvertedToLocalVariable")]
        private readonly LowLevelMouseProc _hookProc;

        private readonly IntPtr _hookId;
        private bool _disposed;

        public event Action<MouseHookEventArgs> MouseEvent;

        public MouseHook () {
            _hookProc = HookProc;
            using( Process curProcess = Process.GetCurrentProcess() ) {
                using( ProcessModule curModule = curProcess.MainModule ) {
                    _hookId = SetWindowsHookEx(14, _hookProc, GetModuleHandle(curModule.ModuleName), 0);
                }
            }
        }

        ~MouseHook () {
            Dispose();
        }

        public void Dispose () {
            if( _disposed ) {
                return;
            }
            UnhookWindowsHookEx(_hookId);
            _disposed = true;
            GC.SuppressFinalize(this);
        }

        private IntPtr HookProc (int nCode, IntPtr wParam, ref LowLevelMouseStruct lParam) {
            if( nCode >= 0 ) {
                Wm w = (Wm) wParam;
                if( Enum.IsDefined(typeof(Wm), w) ) {
                    Point point = lParam.Point;
                    bool click = w == Wm.LeftDown || w == Wm.LeftUp || w == Wm.RightDown || w == Wm.RightUp;

                    MouseHookEventArgs e = new MouseHookEventArgs(point, click);
                    MouseEvent?.Invoke(e);
                    if( e.Handled ) {
                        return (IntPtr) 1;
                    }
                }
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

        [SuppressMessage("ReSharper", "UnusedMember.Local")]
        private enum Wm {

            MouseMove = 0x200,
            LeftDown = 0x201,
            LeftUp = 0x202,
            RightDown = 0x204,
            RightUp = 0x205,
            MouseWheel = 0x20a,
            MouseHWheel = 0x20e

        }

        private delegate IntPtr LowLevelMouseProc (int nCode, IntPtr wParam, ref LowLevelMouseStruct lParam);

        [DllImport("kernel32.dll")]
        private static extern IntPtr GetModuleHandle (string lpModuleName);

        [DllImport("user32.dll")]
        private static extern IntPtr SetWindowsHookEx (int idHook, LowLevelMouseProc lpfn, IntPtr hMod, int dwThreadId);

        [DllImport("user32.dll")]
        private static extern bool UnhookWindowsHookEx (IntPtr hhk);

        [DllImport("user32.dll")]
        private static extern IntPtr CallNextHookEx (IntPtr hhk,
            int nCode,
            IntPtr wParam,
            ref LowLevelMouseStruct lParam);

    }

}
