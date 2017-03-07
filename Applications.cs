using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Applications : IDisposable {

        public readonly IList<ApplicationWindow> Windows = new List<ApplicationWindow>();
        private readonly Form _owner;
        private ApplicationWindow _selected;

        public Applications (Form owner) {
            _owner = owner;
        }

        ~Applications () {
            Dispose();
        }

        public ApplicationWindow Selected {
            get { return _selected; }
            set {
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
                foreach( ApplicationWindow window in Windows ) {
                    window.Visible = value;
                }
            }
        }

        public void Populate () {
            WindowFinder finder = new WindowFinder();
            foreach( WindowHandle window in finder.Windows ) {
                Windows.Add(new ApplicationWindow(_owner, window, Windows.Count));
            }
            Layout();
        }

        public void Layout () {
            FrigoTab.Layout.LayoutWindows(Windows);
        }

        public void Dispose () {
            foreach( ApplicationWindow window in Windows ) {
                window.Dispose();
            }
            Windows.Clear();
        }

        public void Refresh () {
            Dispose();
            Populate();
        }

        public void SelectByIndex (int index) {
            Selected = (index >= 0) && (index < Windows.Count) ? Windows[index] : null;
        }

        public void SelectByPoint (Point point) {
            Selected = Windows.FirstOrDefault(window => window.Bounds.Contains(point));
        }

    }

}
