using System;
using System.Diagnostics;
using System.Diagnostics.CodeAnalysis;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace FrigoTab {

    public class KeyboardHook : IDisposable {

        [SuppressMessage ("ReSharper", "PrivateFieldCanBeConvertedToLocalVariable")]
        private readonly KeyboardProc _hookProc;

        private readonly IntPtr _hookId;

        public event EventHandler<KeyboardHookEventArgs> KeyEvent;

        public KeyboardHook () {
            _hookProc = HookProc;
            using( Process curProcess = Process.GetCurrentProcess() ) {
                using( ProcessModule curModule = curProcess.MainModule ) {
                    _hookId = SetWindowsHookEx(13, _hookProc, GetModuleHandle(curModule.ModuleName), 0);
                }
            }
        }

        public void Dispose () {
            UnhookWindowsHookEx(_hookId);
        }

        private IntPtr HookProc (int nCode, IntPtr wParam, ref Lparam lParam) {
            if( nCode >= 0 ) {
                Wm w = (Wm) wParam;
                if( (w == Wm.KeyDown) || (w == Wm.KeyUp) || (w == Wm.SysKeyDown) || (w == Wm.SysKeyUp) ) {
                    Keys key = (Keys) lParam.VkCode;
                    bool alt = (lParam.Flags & 32) == 32;

                    KeyboardHookEventArgs e = new KeyboardHookEventArgs(key, alt);
                    KeyEvent?.Invoke(this, e);
                    if( e.Handled ) {
                        return (IntPtr) 1;
                    }
                }
            }
            return CallNextHookEx(_hookId, nCode, wParam, ref lParam);
        }

        private struct Lparam {

            public int VkCode;
            public int ScanCode;
            public int Flags;
            public int Time;
            public int DwExtraInfo;

        }

        private enum Wm {

            KeyDown = 0x0100,
            KeyUp = 0x0101,
            SysKeyDown = 0x0104,
            SysKeyUp = 0x0105

        }

        private delegate IntPtr KeyboardProc (int nCode, IntPtr wParam, ref Lparam lParam);

        [DllImport ("kernel32.dll", CharSet = CharSet.Auto, SetLastError = true)]
        private static extern IntPtr GetModuleHandle (string lpModuleName);

        [DllImport ("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
        private static extern IntPtr SetWindowsHookEx (int idHook, KeyboardProc lpfn, IntPtr hMod, uint dwThreadId);

        [DllImport ("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
        private static extern bool UnhookWindowsHookEx (IntPtr hhk);

        [DllImport ("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
        private static extern IntPtr CallNextHookEx (IntPtr hhk, int nCode, IntPtr wParam, ref Lparam lParam);

    }

}
