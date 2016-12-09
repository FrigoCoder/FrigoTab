using System;
using System.Diagnostics;
using System.Windows.Forms;
using System.Runtime.InteropServices;
using System.Text;

namespace FastTab {

    public class KeyboardHook : IDisposable {

        public struct LPARAM {
            public int vkCode;
            public int scanCode;
            public int flags;
            public int time;
            public int dwExtraInfo;
        }

        public delegate bool KeyCallback(int wParam, LPARAM lParam);

        private delegate IntPtr KeyboardProc(int nCode, IntPtr wParam, ref LPARAM lParam);

        private KeyCallback callback;
        private KeyboardProc hookProc;
        private IntPtr hookId;

        public KeyboardHook(KeyCallback callback) {
            this.callback = callback;
            hookProc = new KeyboardProc(HookProc);
            using (Process curProcess = Process.GetCurrentProcess())
            using (ProcessModule curModule = curProcess.MainModule) {
                hookId = SetWindowsHookEx(13, hookProc, GetModuleHandle(curModule.ModuleName), 0);
            }
        }

        private IntPtr HookProc(int nCode, IntPtr wParam, ref LPARAM lParam) {
            bool callNext = nCode < 0 || callback((int)wParam, lParam);
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
