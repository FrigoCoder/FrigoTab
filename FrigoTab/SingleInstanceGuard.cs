using System;
using System.Threading;

namespace FrigoTab {

    /// <summary>
    /// Holds a per-user named mutex for the lifetime of the tray application.
    /// </summary>
    public sealed class SingleInstanceGuard : IDisposable {

        public const string ApplicationMutexName = "Local\\FrigoTab";

        private Mutex mutex;
        private bool ownsMutex;

        private SingleInstanceGuard (Mutex mutex) {
            this.mutex = mutex;
            ownsMutex = true;
        }

        public static bool TryAcquire (string name, out SingleInstanceGuard guard) {
            if( String.IsNullOrWhiteSpace(name) ) {
                throw new ArgumentException("A mutex name is required.", nameof(name));
            }

            guard = null;
            Mutex candidate = new Mutex(false, name);
            bool acquired;
            try {
                acquired = candidate.WaitOne(0);
            }
            catch( AbandonedMutexException ) {
                acquired = true;
            }

            if( !acquired ) {
                candidate.Dispose();
                return false;
            }

            guard = new SingleInstanceGuard(candidate);
            return true;
        }

        public void Dispose () {
            Mutex current = mutex;
            if( current == null ) {
                return;
            }
            mutex = null;
            if( ownsMutex ) {
                ownsMutex = false;
                try {
                    current.ReleaseMutex();
                }
                catch( ApplicationException ) {
                    // Ownership can only be lost during exceptional teardown.
                }
            }
            current.Dispose();
            GC.SuppressFinalize(this);
        }

    }

}
