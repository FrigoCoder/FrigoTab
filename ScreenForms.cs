using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class ScreenForms : IDisposable {

        private readonly Form _owner;
        private readonly IList<FrigoForm> _forms = new List<FrigoForm>();

        public ScreenForms (Form owner) {
            _owner = owner;
        }

        ~ScreenForms () {
            Dispose();
        }

        public bool Visible {
            set {
                foreach( FrigoForm form in _forms ) {
                    form.Visible = value;
                }
            }
        }

        public void Populate () {
            foreach( Screen screen in Screen.AllScreens ) {
                _forms.Add(new FrigoForm {
                    Owner = _owner,
                    Bounds = screen.WorkingArea
                });
            }
        }

        public void Dispose () {
            foreach( FrigoForm form in _forms ) {
                form.Close();
            }
            _forms.Clear();
        }

        public void Refresh () {
            Dispose();
            Populate();
        }

        public bool IsOnAToolBar (Point point) {
            return _forms.FirstOrDefault(form => form.Bounds.Contains(point)) == null;
        }

    }

}
