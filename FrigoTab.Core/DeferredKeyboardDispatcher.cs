using System;
using System.Threading;

namespace FrigoTab.Core {

    /// <summary>
    /// Bounds and defers keyboard delivery.  The supplied post operation must
    /// enqueue the callback rather than execute application work inline.
    /// </summary>
    public sealed class DeferredKeyboardDispatcher {

        private readonly Func<Action, bool> post;
        private readonly Action<KeyboardInput> handler;
        private readonly int capacity;
        private int pending;
        private int generation;

        public DeferredKeyboardDispatcher (
            Func<Action, bool> post,
            Action<KeyboardInput> handler,
            int capacity = 64) {
            if( post == null ) {
                throw new ArgumentNullException(nameof(post));
            }
            if( handler == null ) {
                throw new ArgumentNullException(nameof(handler));
            }
            if( capacity <= 0 ) {
                throw new ArgumentOutOfRangeException(nameof(capacity));
            }
            this.post = post;
            this.handler = handler;
            this.capacity = capacity;
        }

        public int PendingCount => Volatile.Read(ref pending);

        /// <summary>
        /// Prevents callbacks posted before a session/desktop interruption
        /// from reaching the handler after input state has been reset.
        /// Already-posted callbacks still release their bounded queue slots.
        /// </summary>
        public void InvalidatePending () => Interlocked.Increment(ref generation);

        public bool TryDispatch (KeyboardInput input) => TryDispatch(input, capacity);

        /// <summary>
        /// Uses one reserved slot beyond the ordinary queue capacity. Session
        /// termination must survive a burst of repeat events while the UI
        /// thread is constructing or rendering the overlay.
        /// </summary>
        public bool TryDispatchCritical (KeyboardInput input) => TryDispatch(input, capacity + 1);

        private bool TryDispatch (KeyboardInput input, int limit) {
            if( Interlocked.Increment(ref pending) > limit ) {
                Interlocked.Decrement(ref pending);
                return false;
            }

            int dispatchGeneration = Volatile.Read(ref generation);
            int released = 0;
            Action release = () => {
                if( Interlocked.Exchange(ref released, 1) == 0 ) {
                    Interlocked.Decrement(ref pending);
                }
            };
            Action callback = () => {
                release();
                if( dispatchGeneration != Volatile.Read(ref generation) ) {
                    return;
                }
                handler(input);
            };

            try {
                if( post(callback) ) {
                    return true;
                }
            }
            catch {
                // Posting can fail while the UI handle is being destroyed.
            }

            release();
            return false;
        }

    }

}
