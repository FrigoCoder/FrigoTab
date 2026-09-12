using System;

namespace FrigoTab.Core {

    /// <summary>
    /// Deterministic application boundary for an Alt-Tab replacement.
    ///
    /// This class deliberately knows nothing about HWNDs, WinForms, DWM, or
    /// drawing.  Those operations are supplied by ISwitcherSessionPort so the
    /// acceptance tests can exercise the complete interaction contract with a
    /// fake desktop and the production app can use a Win32 adapter.
    /// </summary>
    public sealed class SwitcherApplication {

        private readonly ISwitcherSessionPort port;
        private SwitcherState state;
        private int candidateCount;
        private int? selectedIndex;

        public SwitcherApplication (ISwitcherSessionPort port) {
            if( port == null ) {
                throw new ArgumentNullException(nameof(port));
            }
            this.port = port;
            state = SwitcherState.Idle;
        }

        public SwitcherState State => state;
        public int CandidateCount => candidateCount;
        public int? SelectedIndex => selectedIndex;

        /// <summary>
        /// Handles a normalized keyboard event and returns whether the global
        /// hook should consume it.  In particular, the first Alt+Tab is only
        /// consumed after the session has opened successfully.
        /// </summary>
        public KeyHandling HandleKeyboard (KeyboardInput input) {
            if( input.Injected ) {
                return KeyHandling.PassThrough;
            }

            return state == SwitcherState.Idle ? HandleIdleKeyboard(input) : HandleVisibleKeyboard(input);
        }

        /// <summary>
        /// Updates selection from a pointer move over the session surface.
        /// </summary>
        public void HandleMouseMove (ScreenPoint point) {
            if( state != SwitcherState.Visible ) {
                return;
            }

            int? index;
            try {
                index = port.HitTest(point);
            }
            catch (Exception) {
                CloseAfterPortFailure();
                return;
            }

            if( index.HasValue ) {
                if( !IsValidIndex(index.Value) ) {
                    CloseAfterPortFailure();
                    return;
                }
                TrySelect(index.Value);
            }
            else {
                TryClearSelection();
            }
        }

        /// <summary>
        /// Selects and activates the candidate under a pointer click.
        /// </summary>
        public void HandleMouseClick (ScreenPoint point) {
            if( state != SwitcherState.Visible ) {
                return;
            }

            int? index;
            try {
                index = port.HitTest(point);
            }
            catch (Exception) {
                CloseAfterPortFailure();
                return;
            }

            if( !index.HasValue ) {
                TryClearSelection();
                return;
            }

            if( !IsValidIndex(index.Value) ) {
                CloseAfterPortFailure();
                return;
            }

            if( !TrySelect(index.Value) ) {
                return;
            }
            TryCommitSelection();
        }

        /// <summary>
        /// Closes the active session.  It is also safe to call while idle.
        /// </summary>
        public void Close () {
            try {
                port.Close();
            }
            catch (Exception) {
                // Closing is best effort; the state must never remain active
                // merely because native cleanup failed.
            }
            ResetState();
        }

        /// <summary>
        /// Handles workstation lock, desktop interruption, or another event
        /// that makes the current overlay unusable.
        /// </summary>
        public void Interrupt () => Close();

        /// <summary>
        /// Recalculates the visible session after a display/DPI topology change.
        /// A failed relayout closes the session to avoid stale UI and stale
        /// native resources.
        /// </summary>
        public void Relayout () {
            if( state != SwitcherState.Visible ) {
                return;
            }

            try {
                port.Relayout();
            }
            catch (Exception) {
                CloseAfterPortFailure();
            }
        }

        private KeyHandling HandleIdleKeyboard (KeyboardInput input) {
            if( !input.IsDown || input.Key != SwitcherKey.Tab || !input.Alt ) {
                return KeyHandling.PassThrough;
            }

            return TryOpen() ? KeyHandling.Consume : KeyHandling.PassThrough;
        }

        private KeyHandling HandleVisibleKeyboard (KeyboardInput input) {
            if( input.IsDown && input.Key == SwitcherKey.Tab && input.Alt ) {
                MoveSelection(input.Shift ? -1 : 1);
                return KeyHandling.Consume;
            }

            if( input.IsDown && input.Key == SwitcherKey.Escape ) {
                Close();
                return KeyHandling.Consume;
            }

            if( input.IsDown && input.Key == SwitcherKey.F4 && input.Alt ) {
                Close();
                return KeyHandling.Consume;
            }

            if( input.IsDown ) {
                int digitIndex = GetDigitIndex(input.Key);
                if( digitIndex >= 0 ) {
                    // An out-of-range digit is still consumed while the
                    // session is visible, but it must not clear the selection.
                    if( IsValidIndex(digitIndex) ) {
                        TrySelect(digitIndex);
                        TryCommitSelection();
                    }
                    return KeyHandling.Consume;
                }
            }

            if( input.IsUp && input.Key == SwitcherKey.Alt ) {
                TryCommitSelection();
                // The physical Alt-down event was allowed through before the
                // switcher opened.  Pass its release through as well so the
                // foreground application cannot be left with a stuck modifier.
                return KeyHandling.PassThrough;
            }

            return KeyHandling.PassThrough;
        }

        private bool TryOpen () {
            int count;
            try {
                if( !port.TryOpen(out count) || count <= 0 ) {
                    CloseAfterPortFailure();
                    return false;
                }

                // Keep the state private until the initial selection succeeds;
                // a partially constructed native session is never published.
                port.Select(0);
                state = SwitcherState.Visible;
                candidateCount = count;
                selectedIndex = 0;
                return true;
            }
            catch (Exception) {
                CloseAfterPortFailure();
                return false;
            }
        }

        private void MoveSelection (int direction) {
            if( candidateCount <= 0 ) {
                return;
            }

            // Pointer movement outside every tile deliberately clears the
            // selection.  The keyboard must still be able to take control
            // again: forward traversal starts at the first candidate and
            // reverse traversal starts at the last candidate.
            if( !selectedIndex.HasValue ) {
                TrySelect(direction < 0 ? candidateCount - 1 : 0);
                return;
            }

            int next = selectedIndex.Value + direction;
            if( next < 0 ) {
                next = candidateCount - 1;
            }
            else if( next >= candidateCount ) {
                next = 0;
            }
            TrySelect(next);
        }

        private bool TrySelect (int index) {
            if( state != SwitcherState.Visible || !IsValidIndex(index) ) {
                return false;
            }

            try {
                port.Select(index);
                selectedIndex = index;
                return true;
            }
            catch (Exception) {
                CloseAfterPortFailure();
                return false;
            }
        }

        private void TryClearSelection () {
            try {
                port.ClearSelection();
                selectedIndex = null;
            }
            catch (Exception) {
                CloseAfterPortFailure();
            }
        }

        private void TryCommitSelection () {
            if( state != SwitcherState.Visible || !selectedIndex.HasValue ) {
                return;
            }

            bool activated;
            try {
                activated = port.TryActivateSelected();
            }
            catch (Exception) {
                // An activation exception is equivalent to a failed
                // activation: leave the session available for retry/cancel.
                return;
            }

            if( activated ) {
                Close();
            }
        }

        private void CloseAfterPortFailure () {
            try {
                port.Close();
            }
            catch (Exception) {
                // Native cleanup remains best effort; ResetState is mandatory.
            }
            ResetState();
        }

        private bool IsValidIndex (int index) => index >= 0 && index < candidateCount;

        private static int GetDigitIndex (SwitcherKey key) {
            switch( key ) {
                case SwitcherKey.D1:
                case SwitcherKey.NumPad1:
                    return 0;
                case SwitcherKey.D2:
                case SwitcherKey.NumPad2:
                    return 1;
                case SwitcherKey.D3:
                case SwitcherKey.NumPad3:
                    return 2;
                case SwitcherKey.D4:
                case SwitcherKey.NumPad4:
                    return 3;
                case SwitcherKey.D5:
                case SwitcherKey.NumPad5:
                    return 4;
                case SwitcherKey.D6:
                case SwitcherKey.NumPad6:
                    return 5;
                case SwitcherKey.D7:
                case SwitcherKey.NumPad7:
                    return 6;
                case SwitcherKey.D8:
                case SwitcherKey.NumPad8:
                    return 7;
                case SwitcherKey.D9:
                case SwitcherKey.NumPad9:
                    return 8;
                default:
                    return -1;
            }
        }

        private void ResetState () {
            state = SwitcherState.Idle;
            candidateCount = 0;
            selectedIndex = null;
        }

    }

}
