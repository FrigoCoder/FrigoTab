using System;
using System.ComponentModel;
using System.Diagnostics;
using System.Diagnostics.CodeAnalysis;
using System.Runtime.InteropServices;
using System.Windows.Forms;
using FrigoTab.Core;

#pragma warning disable 169
#pragma warning disable 649

namespace FrigoTab {

    public class KeyHookEventArgs {

        public readonly Keys Key;
        public readonly KeyboardInput Input;
        public bool Handled;

        public KeyHookEventArgs (Keys key) :
            this(key, LegacyInput(key)) {
        }

        internal KeyHookEventArgs (Keys key, KeyboardInput input) {
            Key = key;
            Input = input;
        }

        private static KeyboardInput LegacyInput (Keys key) {
            Keys keyCode = key & Keys.KeyCode;
            bool alt = (key & Keys.Alt) == Keys.Alt;
            return new KeyboardInput(MapKey(keyCode), KeyTransition.Down, alt, false, false);
        }

        private static SwitcherKey MapKey (Keys key) {
            switch( key ) {
                case Keys.Tab:
                    return SwitcherKey.Tab;
                case Keys.Escape:
                    return SwitcherKey.Escape;
                case Keys.F4:
                    return SwitcherKey.F4;
                case Keys.Menu:
                case Keys.LMenu:
                case Keys.RMenu:
                    return SwitcherKey.Alt;
                case Keys.D1:
                    return SwitcherKey.D1;
                case Keys.D2:
                    return SwitcherKey.D2;
                case Keys.D3:
                    return SwitcherKey.D3;
                case Keys.D4:
                    return SwitcherKey.D4;
                case Keys.D5:
                    return SwitcherKey.D5;
                case Keys.D6:
                    return SwitcherKey.D6;
                case Keys.D7:
                    return SwitcherKey.D7;
                case Keys.D8:
                    return SwitcherKey.D8;
                case Keys.D9:
                    return SwitcherKey.D9;
                case Keys.NumPad1:
                    return SwitcherKey.NumPad1;
                case Keys.NumPad2:
                    return SwitcherKey.NumPad2;
                case Keys.NumPad3:
                    return SwitcherKey.NumPad3;
                case Keys.NumPad4:
                    return SwitcherKey.NumPad4;
                case Keys.NumPad5:
                    return SwitcherKey.NumPad5;
                case Keys.NumPad6:
                    return SwitcherKey.NumPad6;
                case Keys.NumPad7:
                    return SwitcherKey.NumPad7;
                case Keys.NumPad8:
                    return SwitcherKey.NumPad8;
                case Keys.NumPad9:
                    return SwitcherKey.NumPad9;
                default:
                    return SwitcherKey.Unknown;
            }
        }

    }

    public class KeyHook : IDisposable {

        private const int LowLevelKeyboardHook = 13;

        public event Action<KeyHookEventArgs> KeyEvent;

        [SuppressMessage("ReSharper", "PrivateFieldCanBeConvertedToLocalVariable")]
        private readonly LowLevelKeyProc hookProc;

        private readonly IntPtr hookId;
        private bool disposed;

        public KeyHook () {
            hookProc = HookProc;
            IntPtr moduleHandle = IntPtr.Zero;
            try {
                using( Process curProcess = Process.GetCurrentProcess() ) {
                    using( ProcessModule curModule = curProcess.MainModule ) {
                        if( curModule != null ) {
                            moduleHandle = GetModuleHandle(curModule.ModuleName);
                        }
                    }
                }
            }
            catch( Exception exception ) {
                int error = Marshal.GetLastWin32Error();
                throw new Win32Exception(
                    error == 0 ? 1 : error,
                    "Unable to resolve the FrigoTab module for the global keyboard hook: " + exception.Message);
            }

            hookId = SetWindowsHookEx(LowLevelKeyboardHook, hookProc, moduleHandle, 0);
            if( hookId == IntPtr.Zero ) {
                int error = Marshal.GetLastWin32Error();
                throw new Win32Exception(
                    error == 0 ? 1 : error,
                    "Unable to install the global keyboard hook. FrigoTab cannot intercept Alt+Tab.");
            }
        }

        ~KeyHook () => Dispose();

        public void Dispose () {
            if( disposed ) {
                return;
            }

            disposed = true;
            if( hookId != IntPtr.Zero ) {
                UnhookWindowsHookEx(hookId);
            }
            GC.SuppressFinalize(this);
        }

        private IntPtr HookProc (int nCode, IntPtr wParam, ref LowLevelKeyStruct lParam) {
            bool handled = false;
            try {
                handled = HookProcInner(nCode, (WindowMessages) wParam, ref lParam);
            }
            catch( Exception exception ) {
                // Never allow an exception to cross the unmanaged hook boundary.
                Debug.WriteLine(exception);
            }

            if( handled ) {
                return (IntPtr) 1;
            }
            return CallNextHookEx(hookId, nCode, wParam, ref lParam);
        }

        private bool HookProcInner (int nCode, WindowMessages wParam, ref LowLevelKeyStruct lParam) {
            if( nCode < 0 || disposed ) {
                return false;
            }

            KeyTransition transition;
            if( !TryGetTransition(wParam, out transition) ) {
                return false;
            }

            Keys keyCode = lParam.VkCode & Keys.KeyCode;
            bool keyIsAlt = IsAltKey(keyCode);
            bool keyIsShift = IsShiftKey(keyCode);
            bool alt = lParam.Flags.HasFlag(LowLevelKeyFlags.AltDown) ||
                IsModifierDown(Keys.Menu) || (keyIsAlt && transition == KeyTransition.Down);
            bool shift = IsModifierDown(Keys.ShiftKey) || (keyIsShift && transition == KeyTransition.Down);
            bool injected = lParam.Flags.HasFlag(LowLevelKeyFlags.Injected) ||
                lParam.Flags.HasFlag(LowLevelKeyFlags.LowerIntegrityInjected);
            SwitcherKey key = MapKey(keyCode);

            // Report modifier state before applying this transition so Alt-up
            // remains a recognizable commit gesture.
            KeyboardInput input = new KeyboardInput(key, transition, alt, shift, injected);
            Keys legacyKey = alt ? keyCode | Keys.Alt : keyCode;
            KeyHookEventArgs e = new KeyHookEventArgs(legacyKey, input);

            try {
                KeyEvent?.Invoke(e);
            }
            catch( Exception exception ) {
                // Subscriber failures must not disable the global hook or block
                // input in another application.
                Debug.WriteLine(exception);
                return false;
            }
            return e.Handled;
        }

        private static bool TryGetTransition (WindowMessages message, out KeyTransition transition) {
            switch( message ) {
                case WindowMessages.KeyDown:
                case WindowMessages.SysKeyDown:
                    transition = KeyTransition.Down;
                    return true;
                case WindowMessages.KeyUp:
                case WindowMessages.SysKeyUp:
                    transition = KeyTransition.Up;
                    return true;
                default:
                    transition = KeyTransition.Down;
                    return false;
            }
        }

        private static bool IsAltKey (Keys key) =>
            key == Keys.Menu || key == Keys.LMenu || key == Keys.RMenu;

        private static bool IsShiftKey (Keys key) =>
            key == Keys.ShiftKey || key == Keys.LShiftKey || key == Keys.RShiftKey;

        private static bool IsModifierDown (Keys key) =>
            (GetAsyncKeyState((int) key) & 0x8000) != 0;

        private static SwitcherKey MapKey (Keys key) {
            switch( key ) {
                case Keys.Tab:
                    return SwitcherKey.Tab;
                case Keys.Escape:
                    return SwitcherKey.Escape;
                case Keys.F4:
                    return SwitcherKey.F4;
                case Keys.Menu:
                case Keys.LMenu:
                case Keys.RMenu:
                    return SwitcherKey.Alt;
                case Keys.D1:
                    return SwitcherKey.D1;
                case Keys.D2:
                    return SwitcherKey.D2;
                case Keys.D3:
                    return SwitcherKey.D3;
                case Keys.D4:
                    return SwitcherKey.D4;
                case Keys.D5:
                    return SwitcherKey.D5;
                case Keys.D6:
                    return SwitcherKey.D6;
                case Keys.D7:
                    return SwitcherKey.D7;
                case Keys.D8:
                    return SwitcherKey.D8;
                case Keys.D9:
                    return SwitcherKey.D9;
                case Keys.NumPad1:
                    return SwitcherKey.NumPad1;
                case Keys.NumPad2:
                    return SwitcherKey.NumPad2;
                case Keys.NumPad3:
                    return SwitcherKey.NumPad3;
                case Keys.NumPad4:
                    return SwitcherKey.NumPad4;
                case Keys.NumPad5:
                    return SwitcherKey.NumPad5;
                case Keys.NumPad6:
                    return SwitcherKey.NumPad6;
                case Keys.NumPad7:
                    return SwitcherKey.NumPad7;
                case Keys.NumPad8:
                    return SwitcherKey.NumPad8;
                case Keys.NumPad9:
                    return SwitcherKey.NumPad9;
                default:
                    return SwitcherKey.Unknown;
            }
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct LowLevelKeyStruct {

            public Keys VkCode;
            public uint ScanCode;
            public LowLevelKeyFlags Flags;
            public uint Time;
            public UIntPtr DwExtraInfo;

        }

        [Flags]
        private enum LowLevelKeyFlags : uint {

            Extended = 1,
            LowerIntegrityInjected = 2,
            Injected = 16,
            AltDown = 32

        }

        [UnmanagedFunctionPointer(CallingConvention.Winapi)]
        private delegate IntPtr LowLevelKeyProc (int nCode, IntPtr wParam, ref LowLevelKeyStruct lParam);

        [DllImport("kernel32.dll", EntryPoint = "GetModuleHandleW", CharSet = CharSet.Unicode,
            ExactSpelling = true, SetLastError = true)]
        private static extern IntPtr GetModuleHandle (string lpModuleName);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern IntPtr SetWindowsHookEx (int idHook, LowLevelKeyProc lpfn, IntPtr hMod, int dwThreadId);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern bool UnhookWindowsHookEx (IntPtr hhk);

        [DllImport("user32.dll")]
        private static extern IntPtr CallNextHookEx (IntPtr hhk, int nCode, IntPtr wParam, ref LowLevelKeyStruct lParam);

        [DllImport("user32.dll")]
        private static extern short GetAsyncKeyState (int vKey);

    }

}
