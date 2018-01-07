using System;

namespace FrigoTab {

    public struct Property<T> {

        public event Action<T, T> Changed;
        private T current;

        public Property (T defaultValue) => current = defaultValue;

        public T Get () => current;

        public void Set (T value) {
            if( Equals(current, value) ) {
                return;
            }
            T oldValue = current;
            current = value;
            Changed?.Invoke(oldValue, value);
        }

    }

}
