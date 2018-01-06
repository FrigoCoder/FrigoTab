using System;

namespace FrigoTab {

    public class BooleanProperty {

        public event Action Changed;
        private bool boolean;

        public bool Get () => boolean;

        public void Set (bool value) {
            if( boolean == value ) {
                return;
            }
            boolean = value;
            Changed?.Invoke();
        }

    }

}
