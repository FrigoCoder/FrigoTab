using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class ScreenForms : IDisposable {

        private readonly Form _owner;
        private readonly IList<ScreenForm> _forms = new List<ScreenForm>();

        public ScreenForms (Form owner) {
            _owner = owner;
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

        public void Populate () {
            foreach( Screen screen in Screen.AllScreens ) {
                _forms.Add(new ScreenForm(_owner, screen));
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
