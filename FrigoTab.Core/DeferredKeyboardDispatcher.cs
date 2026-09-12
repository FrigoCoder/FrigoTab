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

        public bool TryDispatch (KeyboardInput input) {
            if( Interlocked.Increment(ref pending) > capacity ) {
                Interlocked.Decrement(ref pending);
                return false;
            }

            int released = 0;
            Action release = () => {
                if( Interlocked.Exchange(ref released, 1) == 0 ) {
                    Interlocked.Decrement(ref pending);
                }
            };
            Action callback = () => {
                release();
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
