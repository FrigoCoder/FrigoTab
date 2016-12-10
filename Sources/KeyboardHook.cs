using System;
using System.Diagnostics;
using System.Windows.Forms;
using System.Runtime.InteropServices;
using System.Text;
using System.Collections.Generic;
using System.Collections.ObjectModel;

namespace FastTab {

    public class KeyboardHook : IDisposable {

        public struct LPARAM {
            public int vkCode;
            public int scanCode;
            public int flags;
            public int time;
            public int dwExtraInfo;
        }

        public delegate bool KeyCallback(IReadOnlyDictionary<Keys, bool> keys, int wParam, LPARAM lParam);
        private delegate IntPtr KeyboardProc(int nCode, IntPtr wParam, ref LPARAM lParam);

        private const int WM_KEYDOWN = 0x0100;
        private const int WM_KEYUP = 0x0101;
        private const int WM_SYSKEYDOWN = 0x0104;
        private const int WM_SYSKEYUP = 0x0105;

        private KeyCallback callback;
        private KeyboardProc hookProc;
        private IDictionary<Keys, bool> keys;
        private IReadOnlyDictionary<Keys, bool> readOnlyKeys;
        private IntPtr hookId;

        public KeyboardHook(KeyCallback callback) {
            this.callback = callback;
            hookProc = new KeyboardProc(HookProc);

            keys = new Dictionary<Keys, bool>();
            foreach (Keys key in Enum.GetValues(typeof(Keys))) {
                keys[key] = false;
            }
            readOnlyKeys = new ReadOnlyDictionary<Keys, bool>(keys);

            using (Process curProcess = Process.GetCurrentProcess())
            using (ProcessModule curModule = curProcess.MainModule) {
                hookId = SetWindowsHookEx(13, hookProc, GetModuleHandle(curModule.ModuleName), 0);
            }
        }

        private IntPtr HookProc(int nCode, IntPtr wParam, ref LPARAM lParam) {
            if (nCode < 0) {
                return CallNextHookEx(hookId, nCode, wParam, ref lParam);
            }

            int w = (int)wParam;
            Keys key = (Keys)lParam.vkCode;
            if ((w == WM_KEYDOWN) || (w == WM_SYSKEYDOWN)) {
                keys[key] = true;
            }
            if ((w == WM_KEYUP) || (w == WM_SYSKEYUP)) {
                keys[key] = false;
            }

            bool callNext = callback(readOnlyKeys, w, lParam);
            return callNext ? CallNextHookEx(hookId, nCode, wParam, ref lParam) : (IntPtr)1;
        }

        public void Dispose() {
            UnhookWindowsHookEx(hookId);
        }

        [DllImport("kernel32.dll", CharSet = CharSet.Auto, SetLastError = true)]
        private static extern IntPtr GetModuleHandle(string lpModuleName);

        [DllImport("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
        private static extern IntPtr SetWindowsHookEx(int idHook, KeyboardProc lpfn, IntPtr hMod, uint dwThreadId);

        [DllImport("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
        private static extern bool UnhookWindowsHookEx(IntPtr hhk);

        [DllImport("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
        private static extern IntPtr CallNextHookEx(IntPtr hhk, int nCode, IntPtr wParam, ref LPARAM lParam);

    }

}
