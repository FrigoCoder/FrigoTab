﻿using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace FastTab {

    public class KeyboardHook : IDisposable {

        public delegate bool KeyCallback (IDictionary<Keys, bool> keys, int wParam, Lparam lParam);

        public struct Lparam {

            public int VkCode;
            public int ScanCode;
            public int Flags;
            public int Time;
            public int DwExtraInfo;

        }

        private readonly KeyCallback _callback;
        private readonly IntPtr _hookId;
        private readonly KeyboardProc _hookProc;
        private readonly IDictionary<Keys, bool> _keys;

        public KeyboardHook (KeyCallback callback) {
            _callback = callback;
            _hookProc = HookProc;

            _keys = new Dictionary<Keys, bool>();
            foreach( Keys key in Enum.GetValues(typeof(Keys)) ) {
                _keys[key] = false;
            }

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
            if( nCode < 0 ) {
                return CallNextHookEx(_hookId, nCode, wParam, ref lParam);
            }

            int w = (int) wParam;
            Keys key = (Keys) lParam.VkCode;
            if( (w == (int) WindowsMessages.KeyDown) || (w == (int) WindowsMessages.SysKeyDown) ) {
                _keys[key] = true;
            }
            if( (w == (int) WindowsMessages.KeyUp) || (w == (int) WindowsMessages.SysKeyUp) ) {
                _keys[key] = false;
            }

            bool callNext = _callback(_keys, w, lParam);
            return callNext ? CallNextHookEx(_hookId, nCode, wParam, ref lParam) : (IntPtr) 1;
        }

        private enum WindowsMessages {

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