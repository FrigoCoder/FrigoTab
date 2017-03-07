using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Applications : IDisposable {

        private readonly Form _owner;
        private readonly IList<ApplicationWindow> _windows = new List<ApplicationWindow>();
        private ApplicationWindow _selected;

        public Applications (Form owner) {
            _owner = owner;
        }

        ~Applications () {
            Dispose();
        }

        public ApplicationWindow Selected {
            get { return _selected; }
            private set {
                if( _selected == value ) {
                    return;
                }
                if( _selected != null ) {
                    _selected.Selected = false;
                }
                _selected = value;
                if( _selected != null ) {
                    _selected.Selected = true;
                }
            }
        }

        public bool Visible {
            set {
                foreach( ApplicationWindow window in _windows ) {
                    window.Visible = value;
                }
            }
        }

        public void Populate () {
            WindowFinder finder = new WindowFinder();
            foreach( WindowHandle window in finder.Windows ) {
                _windows.Add(new ApplicationWindow(_owner, window, _windows.Count));
            }
            Layout();
        }

        public void Layout () {
            FrigoTab.Layout.LayoutWindows(_windows);
        }

        public void Dispose () {
            foreach( ApplicationWindow window in _windows ) {
                window.Dispose();
            }
            _windows.Clear();
        }

        public void Refresh () {
            Dispose();
            Populate();
        }

        public void SelectByIndex (int index) {
            Selected = (index >= 0) && (index < _windows.Count) ? _windows[index] : null;
        }

        public void SelectByPoint (Point point) {
            Selected = _windows.FirstOrDefault(window => window.Bounds.Contains(point));
        }

    }

}
