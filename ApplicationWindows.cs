using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class ApplicationWindows : IDisposable {

        private readonly IList<ApplicationWindow> _windows = new List<ApplicationWindow>();
        private ApplicationWindow _selected;

        public ApplicationWindows (Form owner, WindowFinder finder) {
            foreach( WindowHandle window in finder.Windows ) {
                _windows.Add(new ApplicationWindow(owner, window, _windows.Count));
            }
            Layout.LayoutWindows(_windows);
        }

        ~ApplicationWindows () {
            Dispose();
        }

        public ApplicationWindow Selected {
            get => _selected;
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

        public void Dispose () {
            foreach( ApplicationWindow window in _windows ) {
                window.Close();
            }
            _windows.Clear();
        }

        public void SelectByIndex (int index) {
            Selected = index >= 0 && index < _windows.Count ? _windows[index] : null;
        }

        public void SelectByPoint (Point point) {
            Selected = _windows.FirstOrDefault(window => window.Bounds.Contains(point));
        }

    }

}
