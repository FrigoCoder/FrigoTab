using System;

namespace FrigoTab {

    public struct Property<T> {

        public event Action<T, T> Changed;

        public T Value {
            get => current;
            set {
                if( Equals(current, value) ) {
                    return;
                }
                T oldValue = current;
                current = value;
                Changed?.Invoke(oldValue, value);
            }
        }

        private T current;

    }

}
