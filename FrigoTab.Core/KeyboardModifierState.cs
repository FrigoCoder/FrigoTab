using System.Collections.Generic;

namespace FrigoTab.Core {

    /// <summary>
    /// Derives modifier state from the low-level keyboard event stream.  The
    /// Windows hook is called before GetAsyncKeyState is updated, so sampling
    /// asynchronous state from inside that callback is inherently stale.
    /// </summary>
    public sealed class KeyboardModifierState {

        private readonly object gate = new object();
        private readonly HashSet<KeyboardModifierKey> altKeys = new HashSet<KeyboardModifierKey>();
        private readonly HashSet<KeyboardModifierKey> shiftKeys = new HashSet<KeyboardModifierKey>();

        public bool AltDown {
            get {
                lock( gate ) {
                    return altKeys.Count > 0;
                }
            }
        }

        public bool ShiftDown {
            get {
                lock( gate ) {
                    return shiftKeys.Count > 0;
                }
            }
        }

        public KeyboardInput CreateInput (
            SwitcherKey key,
            KeyTransition transition,
            bool nativeAltDown,
            bool injected) => CreateInput(
                key,
                transition,
                nativeAltDown,
                injected,
                DefaultModifier(key));

        public KeyboardInput CreateInput (
            SwitcherKey key,
            KeyTransition transition,
            bool nativeAltDown,
            bool injected,
            KeyboardModifierKey physicalModifier) {
            lock( gate ) {
                bool isDown = transition == KeyTransition.Down;
                bool alt = nativeAltDown || altKeys.Count > 0 || (IsAlt(physicalModifier) && isDown);
                bool shift = shiftKeys.Count > 0 || (IsShift(physicalModifier) && isDown);

                KeyboardInput input = new KeyboardInput(key, transition, alt, shift, injected);
                if( injected ) {
                    return input;
                }

                // Apply the transition after taking the snapshot. Alt-up therefore
                // still describes the physical state before the release without
                // relying on GetAsyncKeyState inside the low-level hook.
                if( IsAlt(physicalModifier) ) {
                    if( isDown ) {
                        altKeys.Add(physicalModifier);
                    }
                    else {
                        altKeys.Remove(physicalModifier);
                        // Alt is the sentinel used when LLKHF_ALTDOWN recovers a
                        // transition missed during hook installation/reset.
                        altKeys.Remove(KeyboardModifierKey.Alt);
                    }
                }
                else if( IsShift(physicalModifier) ) {
                    Update(shiftKeys, physicalModifier, isDown);
                }
                if( nativeAltDown && altKeys.Count == 0 && !IsAlt(physicalModifier) ) {
                    // LLKHF_ALTDOWN is part of this hook event, not a separately
                    // sampled asynchronous state. Remember it until an Alt-up.
                    altKeys.Add(KeyboardModifierKey.Alt);
                }

                return input;
            }
        }

        public void Reset () {
            lock( gate ) {
                altKeys.Clear();
                shiftKeys.Clear();
            }
        }

        private static void Update (
            ISet<KeyboardModifierKey> keys,
            KeyboardModifierKey modifier,
            bool isDown) {
            if( isDown ) {
                keys.Add(modifier);
            }
            else {
                keys.Remove(modifier);
            }
        }

        private static KeyboardModifierKey DefaultModifier (SwitcherKey key) {
            if( key == SwitcherKey.Alt ) {
                return KeyboardModifierKey.Alt;
            }
            if( key == SwitcherKey.Shift ) {
                return KeyboardModifierKey.Shift;
            }
            return KeyboardModifierKey.None;
        }

        private static bool IsAlt (KeyboardModifierKey modifier) =>
            modifier == KeyboardModifierKey.Alt ||
            modifier == KeyboardModifierKey.LeftAlt ||
            modifier == KeyboardModifierKey.RightAlt;

        private static bool IsShift (KeyboardModifierKey modifier) =>
            modifier == KeyboardModifierKey.Shift ||
            modifier == KeyboardModifierKey.LeftShift ||
            modifier == KeyboardModifierKey.RightShift;

    }

}
