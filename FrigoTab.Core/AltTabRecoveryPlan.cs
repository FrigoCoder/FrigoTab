using System.Collections.Generic;

namespace FrigoTab.Core {

    /// <summary>
    /// Describes the synthetic gesture needed when an initially suppressed
    /// Alt+Tab cannot open the switcher. The plan remains valid even when the
    /// user released Alt before the UI message queue processed the request.
    /// </summary>
    public static class AltTabRecoveryPlan {

        public static KeyboardInput[] Create (
            bool altStillDown,
            bool reverse = false,
            bool shiftStillDown = false) {
            var result = new List<KeyboardInput>();
            bool syntheticAlt = !altStillDown;
            bool syntheticShift = reverse && !shiftStillDown;
            bool temporarilyReleaseShift = !reverse && shiftStillDown;

            if( syntheticAlt ) {
                result.Add(Input(SwitcherKey.Alt, KeyTransition.Down));
            }
            if( syntheticShift ) {
                result.Add(Input(SwitcherKey.Shift, KeyTransition.Down));
            }
            else if( temporarilyReleaseShift ) {
                result.Add(Input(SwitcherKey.Shift, KeyTransition.Up));
            }

            result.Add(Input(SwitcherKey.Tab, KeyTransition.Down));
            result.Add(Input(SwitcherKey.Tab, KeyTransition.Up));

            if( syntheticShift ) {
                result.Add(Input(SwitcherKey.Shift, KeyTransition.Up));
            }
            else if( temporarilyReleaseShift ) {
                result.Add(Input(SwitcherKey.Shift, KeyTransition.Down));
            }
            if( syntheticAlt ) {
                result.Add(Input(SwitcherKey.Alt, KeyTransition.Up));
            }
            return result.ToArray();
        }

        private static KeyboardInput Input (SwitcherKey key, KeyTransition transition) =>
            new KeyboardInput(key, transition, true, false, true);

    }

}
