using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class ScreenForms : IDisposable {

        private readonly IList<ScreenForm> _forms = new List<ScreenForm>();

        public ScreenForms (Form owner) {
            foreach( Screen screen in Screen.AllScreens ) {
                _forms.Add(new ScreenForm(owner, screen));
            }
        }

        ~ScreenForms () {
            Dispose();
        }

        public bool Visible {
            set {
                foreach( ScreenForm form in _forms ) {
                    form.Visible = value;
                }
            }
        }

        public void Dispose () {
            foreach( ScreenForm form in _forms ) {
                form.Close();
            }
            _forms.Clear();
        }

        public bool IsOnAToolBar (Point point) {
            return _forms.FirstOrDefault(form => form.Bounds.Contains(point)) == null;
        }

    }

}
