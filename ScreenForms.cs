using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class ScreenForms : IDisposable {

        private readonly Form _owner;
        private readonly IList<ScreenForm> _screenForms = new List<ScreenForm>();

        public ScreenForms (Form owner) {
            _owner = owner;
        }

        ~ScreenForms () {
            Dispose();
        }

        public bool Visible {
            set {
                foreach( ScreenForm screenForm in _screenForms ) {
                    screenForm.Visible = value;
                }
            }
        }

        public void Populate () {
            foreach( Screen screen in Screen.AllScreens ) {
                _screenForms.Add(new ScreenForm(_owner, screen));
            }
        }

        public void Dispose () {
            foreach( ScreenForm screenForm in _screenForms ) {
                screenForm.Dispose();
            }
            _screenForms.Clear();
        }

        public void Refresh () {
            Dispose();
            Populate();
        }

        public bool IsOnAToolBar (Point point) {
            return _screenForms.FirstOrDefault(form => form.Bounds.Contains(point)) == null;
        }

    }

}
