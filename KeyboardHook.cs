using System;
using System.Diagnostics;
using System.Diagnostics.CodeAnalysis;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace FrigoTab {

    public class KeyboardHook : IDisposable {

        [SuppressMessage ("ReSharper", "PrivateFieldCanBeConvertedToLocalVariable")]
        private readonly LowLevelKeyProc _hookProc;

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

        private IntPtr HookProc (int nCode, IntPtr wParam, ref LowLevelKeyStruct lParam) {
            if( nCode >= 0 ) {
                Wm w = (Wm) wParam;
                if( (w == Wm.KeyDown) || (w == Wm.KeyUp) || (w == Wm.SysKeyDown) || (w == Wm.SysKeyUp) ) {
                    Keys key = (Keys) lParam.VkCode;
                    bool alt = lParam.Flags.HasFlag(LowLevelKeyFlags.AltDown);

                    KeyboardHookEventArgs e = new KeyboardHookEventArgs(key, alt);
                    KeyEvent?.Invoke(this, e);
                    if( e.Handled ) {
                        return (IntPtr) 1;
                    }
                }
            }
            return CallNextHookEx(_hookId, nCode, wParam, ref lParam);
        }

        private struct LowLevelKeyStruct {

            public int VkCode;
            public int ScanCode;
            public LowLevelKeyFlags Flags;
            public int Time;
            public int DwExtraInfo;

        }

        private enum Wm {

            KeyDown = 0x0100,
            KeyUp = 0x0101,
            SysKeyDown = 0x0104,
            SysKeyUp = 0x0105

        }

        [Flags]
        private enum LowLevelKeyFlags {

            AltDown = 32

        }

        private delegate IntPtr LowLevelKeyProc (int nCode, IntPtr wParam, ref LowLevelKeyStruct lParam);

        [DllImport ("kernel32.dll")]
        private static extern IntPtr GetModuleHandle (string lpModuleName);

        [DllImport ("user32.dll")]
        private static extern IntPtr SetWindowsHookEx (int idHook, LowLevelKeyProc lpfn, IntPtr hMod, int dwThreadId);

        [DllImport ("user32.dll")]
        private static extern bool UnhookWindowsHookEx (IntPtr hhk);

        [DllImport ("user32.dll")]
        private static extern IntPtr CallNextHookEx (IntPtr hhk, int nCode, IntPtr wParam, ref LowLevelKeyStruct lParam);

    }

}
