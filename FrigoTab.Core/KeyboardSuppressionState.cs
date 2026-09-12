using System.Collections.Generic;

namespace FrigoTab.Core {

    /// <summary>
    /// Makes the bounded suppression decision required by LowLevelKeyboardProc.
    /// Heavy session work is intentionally not part of this class.
    /// </summary>
    public sealed class KeyboardSuppressionState {

        private readonly object gate = new object();
        private readonly HashSet<SwitcherKey> consumedKeys = new HashSet<SwitcherKey>();
        private bool sessionActiveOrPending;
        private long nextAdmissionToken;
        private long pendingAdmissionToken;

        public bool SessionActiveOrPending {
            get {
                lock( gate ) {
                    return sessionActiveOrPending;
                }
            }
        }

        public bool AdmissionPending {
            get {
                lock( gate ) {
                    return pendingAdmissionToken != 0;
                }
            }
        }

        public bool ShouldConsume (KeyboardInput input) =>
            ShouldConsume(input, out _);

        /// <summary>
        /// Returns the non-zero token only for the event that changes the
        /// switcher from idle to pending. Repeated Tab events deliberately get
        /// no token, so a failed repeat delivery cannot cancel an admission
        /// that was already accepted by the UI queue.
        /// </summary>
        public bool ShouldConsume (KeyboardInput input, out long admissionToken) {
            lock( gate ) {
                admissionToken = 0;
                if( input.Injected ) {
                    return false;
                }

                if( input.IsUp && consumedKeys.Remove(input.Key) ) {
                    return true;
                }

                if( !input.IsDown ) {
                    return false;
                }

                bool consume;
                if( !sessionActiveOrPending ) {
                    consume = input.Key == SwitcherKey.Tab && input.Alt;
                    if( consume ) {
                        // Admission is marked immediately so repeats arriving before
                        // the UI thread drains the first event remain balanced.
                        sessionActiveOrPending = true;
                        pendingAdmissionToken = NextAdmissionToken();
                        admissionToken = pendingAdmissionToken;
                    }
                }
                else {
                    consume = (input.Key == SwitcherKey.Tab && input.Alt) ||
                        input.Key == SwitcherKey.Escape ||
                        (input.Key == SwitcherKey.F4 && input.Alt) ||
                        IsDigit(input.Key);
                }

                if( consume ) {
                    consumedKeys.Add(input.Key);
                }
                return consume;
            }
        }

        public void SetSessionVisible (bool visible) {
            lock( gate ) {
                sessionActiveOrPending = visible;
                pendingAdmissionToken = 0;
            }
        }

        /// <summary>
        /// Cancels only an admission that has not reached the UI.  This lets a
        /// failed UI post recover the original Alt+Tab gesture without clearing
        /// the consumed-key ledger of an already visible session.
        /// </summary>
        public bool AbortPendingAdmission (long admissionToken) {
            lock( gate ) {
                if( admissionToken == 0 || admissionToken != pendingAdmissionToken ) {
                    return false;
                }

                pendingAdmissionToken = 0;
                sessionActiveOrPending = false;
                consumedKeys.Remove(SwitcherKey.Tab);
                return true;
            }
        }

        public void Reset () {
            lock( gate ) {
                sessionActiveOrPending = false;
                pendingAdmissionToken = 0;
                consumedKeys.Clear();
            }
        }

        private long NextAdmissionToken () {
            ++nextAdmissionToken;
            if( nextAdmissionToken == 0 ) {
                ++nextAdmissionToken;
            }
            return nextAdmissionToken;
        }

        private static bool IsDigit (SwitcherKey key) =>
            (key >= SwitcherKey.D1 && key <= SwitcherKey.D9) ||
            (key >= SwitcherKey.NumPad1 && key <= SwitcherKey.NumPad9);

    }

}
