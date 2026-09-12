namespace FrigoTab.Core {

    /// <summary>
    /// Keys understood by the switcher boundary.  The Win32 adapter translates
    /// virtual-key codes into this small, platform-independent set.
    /// </summary>
    public enum SwitcherKey {

        Unknown,
        Alt,
        Tab,
        Escape,
        F4,
        D1,
        D2,
        D3,
        D4,
        D5,
        D6,
        D7,
        D8,
        D9,
        NumPad1,
        NumPad2,
        NumPad3,
        NumPad4,
        NumPad5,
        NumPad6,
        NumPad7,
        NumPad8,
        NumPad9

    }

    /// <summary>
    /// Whether a keyboard event represents a press or a release.
    /// </summary>
    public enum KeyTransition {

        Down,
        Up

    }

    /// <summary>
    /// A normalized keyboard event supplied by the platform adapter.
    /// </summary>
    public struct KeyboardInput {

        public KeyboardInput (SwitcherKey key, KeyTransition transition, bool alt, bool shift, bool injected) {
            Key = key;
            Transition = transition;
            Alt = alt;
            Shift = shift;
            Injected = injected;
        }

        public SwitcherKey Key { get; }
        public KeyTransition Transition { get; }
        public bool Alt { get; }
        public bool Shift { get; }
        public bool Injected { get; }

        public bool IsDown => Transition == KeyTransition.Down;
        public bool IsUp => Transition == KeyTransition.Up;

    }

}
