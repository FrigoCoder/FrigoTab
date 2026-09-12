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
        private const int MaxPendingKeyboardEvents = 64;

        public event Action<KeyHookEventArgs> KeyEvent;

        [SuppressMessage("ReSharper", "PrivateFieldCanBeConvertedToLocalVariable")]
        private readonly LowLevelKeyProc hookProc;
        private readonly Control dispatcherControl;
        private readonly KeyboardModifierState modifiers = new KeyboardModifierState();
        private readonly KeyboardSuppressionState suppression = new KeyboardSuppressionState();
        private readonly DeferredKeyboardDispatcher deferredDispatcher;
        private readonly IntPtr hookId;
        private bool disposed;

        public KeyHook (Control dispatcherControl) {
            if( dispatcherControl == null ) {
                throw new ArgumentNullException(nameof(dispatcherControl));
            }
            if( !dispatcherControl.IsHandleCreated ) {
                throw new InvalidOperationException("The keyboard dispatcher must have a native handle before installing the hook.");
            }
            this.dispatcherControl = dispatcherControl;
            deferredDispatcher = new DeferredKeyboardDispatcher(
                PostToUiThread,
                DispatchOnUiThread,
                MaxPendingKeyboardEvents);
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
            suppression.Reset();
            modifiers.Reset();
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
            bool injected = lParam.Flags.HasFlag(LowLevelKeyFlags.Injected) ||
                lParam.Flags.HasFlag(LowLevelKeyFlags.LowerIntegrityInjected);
            SwitcherKey key = MapKey(keyCode);
            if( key == SwitcherKey.Unknown || injected ) {
                return false;
            }

            KeyboardInput input = modifiers.CreateInput(
                key,
                transition,
                lParam.Flags.HasFlag(LowLevelKeyFlags.AltDown),
                false,
                MapModifierKey(keyCode));

            // BeginInvoke only posts a Windows message.  Window enumeration,
            // DWM setup, rendering, and activation run later on the UI thread,
            // after LowLevelKeyboardProc has returned.
            if( !deferredDispatcher.TryDispatch(input) ) {
                return false;
            }
            return suppression.ShouldConsume(input);
        }

        public void SetSessionVisible (bool visible) => suppression.SetSessionVisible(visible);

        public void ResetInputState () {
            modifiers.Reset();
            suppression.Reset();
        }

        private bool PostToUiThread (Action action) {
            if( disposed || dispatcherControl.IsDisposed || dispatcherControl.Disposing ) {
                return false;
            }
            dispatcherControl.BeginInvoke(action);
            return true;
        }

        private void DispatchOnUiThread (KeyboardInput input) {
            if( disposed ) {
                return;
            }

            Keys keyCode = MapLegacyKey(input.Key);
            Keys legacyKey = input.Alt ? keyCode | Keys.Alt : keyCode;
            KeyHookEventArgs e = new KeyHookEventArgs(legacyKey, input);
            try {
                KeyEvent?.Invoke(e);
            }
            catch( Exception exception ) {
                // Subscriber failures are contained on the managed UI thread.
                Debug.WriteLine(exception);
            }

            if( !e.Handled && input.IsDown && input.Key == SwitcherKey.Tab && input.Alt ) {
                try {
                    ReplayNativeAltTab(modifiers.AltDown, input.Shift, modifiers.ShiftDown);
                }
                catch( Exception exception ) {
                    // Recovery is best effort; a missing User32 entry point or
                    // marshaling failure must not terminate the message loop.
                    Debug.WriteLine(exception);
                }
            }
        }

        private static void ReplayNativeAltTab (bool altStillDown, bool reverse, bool shiftStillDown) {
            KeyboardInput[] recovery = AltTabRecoveryPlan.Create(altStillDown, reverse, shiftStillDown);
            NativeInput[] inputs = new NativeInput[recovery.Length];
            for( int index = 0; index < recovery.Length; index++ ) {
                KeyboardInput input = recovery[index];
                inputs[index] = NativeInput.Keyboard(
                    (ushort) MapLegacyKey(input.Key),
                    input.IsUp ? KeyboardEventFlags.KeyUp : 0);
            }
            uint sent = SendInput((uint) inputs.Length, inputs, Marshal.SizeOf<NativeInput>());
            if( sent != (uint) inputs.Length ) {
                Debug.WriteLine("Unable to replay Alt+Tab after switcher admission failed. Win32 error: " +
                    Marshal.GetLastWin32Error());
            }
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
                case Keys.ShiftKey:
                case Keys.LShiftKey:
                case Keys.RShiftKey:
                    return SwitcherKey.Shift;
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

        private static Keys MapLegacyKey (SwitcherKey key) {
            switch( key ) {
                case SwitcherKey.Alt:
                    return Keys.Menu;
                case SwitcherKey.Shift:
                    return Keys.ShiftKey;
                case SwitcherKey.Tab:
                    return Keys.Tab;
                case SwitcherKey.Escape:
                    return Keys.Escape;
                case SwitcherKey.F4:
                    return Keys.F4;
                case SwitcherKey.D1:
                case SwitcherKey.D2:
                case SwitcherKey.D3:
                case SwitcherKey.D4:
                case SwitcherKey.D5:
                case SwitcherKey.D6:
                case SwitcherKey.D7:
                case SwitcherKey.D8:
                case SwitcherKey.D9:
                    return (Keys) ((int) Keys.D1 + (int) key - (int) SwitcherKey.D1);
                case SwitcherKey.NumPad1:
                case SwitcherKey.NumPad2:
                case SwitcherKey.NumPad3:
                case SwitcherKey.NumPad4:
                case SwitcherKey.NumPad5:
                case SwitcherKey.NumPad6:
                case SwitcherKey.NumPad7:
                case SwitcherKey.NumPad8:
                case SwitcherKey.NumPad9:
                    return (Keys) ((int) Keys.NumPad1 + (int) key - (int) SwitcherKey.NumPad1);
                default:
                    return Keys.None;
            }
        }

        private static KeyboardModifierKey MapModifierKey (Keys key) {
            switch( key ) {
                case Keys.Menu:
                    return KeyboardModifierKey.Alt;
                case Keys.LMenu:
                    return KeyboardModifierKey.LeftAlt;
                case Keys.RMenu:
                    return KeyboardModifierKey.RightAlt;
                case Keys.ShiftKey:
                    return KeyboardModifierKey.Shift;
                case Keys.LShiftKey:
                    return KeyboardModifierKey.LeftShift;
                case Keys.RShiftKey:
                    return KeyboardModifierKey.RightShift;
                default:
                    return KeyboardModifierKey.None;
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

        [StructLayout(LayoutKind.Sequential)]
        private struct NativeInput {

            public uint Type;
            public NativeInputUnion Data;

            public static NativeInput Keyboard (ushort virtualKey, KeyboardEventFlags flags) => new NativeInput {
                Type = 1,
                Data = new NativeInputUnion {
                    Keyboard = new NativeKeyboardInput {
                        VirtualKey = virtualKey,
                        Flags = flags
                    }
                }
            };

        }

        [StructLayout(LayoutKind.Explicit)]
        private struct NativeInputUnion {

            [FieldOffset(0)]
            public NativeKeyboardInput Keyboard;

            // INPUT's native union is sized by MOUSEINPUT, not KEYBDINPUT.
            // These unused members keep cbSize correct (40 bytes on x64 and
            // 28 on x86), which SendInput validates strictly.
            [FieldOffset(0)]
            public NativeMouseInput Mouse;

            [FieldOffset(0)]
            public NativeHardwareInput Hardware;

        }

        [StructLayout(LayoutKind.Sequential)]
        private struct NativeMouseInput {

            public int X;
            public int Y;
            public uint MouseData;
            public uint Flags;
            public uint Time;
            public UIntPtr ExtraInfo;

        }

        [StructLayout(LayoutKind.Sequential)]
        private struct NativeHardwareInput {

            public uint Message;
            public ushort ParameterLow;
            public ushort ParameterHigh;

        }

        [StructLayout(LayoutKind.Sequential)]
        private struct NativeKeyboardInput {

            public ushort VirtualKey;
            public ushort ScanCode;
            public KeyboardEventFlags Flags;
            public uint Time;
            public UIntPtr ExtraInfo;

        }

        [Flags]
        private enum KeyboardEventFlags : uint {

            KeyUp = 0x0002

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

        [DllImport("user32.dll", SetLastError = true)]
        private static extern uint SendInput (uint inputCount, NativeInput[] inputs, int inputSize);

    }

}
