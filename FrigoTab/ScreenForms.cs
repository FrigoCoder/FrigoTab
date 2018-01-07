using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class ScreenForms : IDisposable {

        public Property<bool> Visible;
        private readonly IList<ScreenForm> forms = new List<ScreenForm>();

        public ScreenForms (Form owner) {
            Visible.Changed += (old, value) => {
                foreach( ScreenForm form in forms ) {
                    form.Visible = value;
                }
            };
            foreach( Screen screen in Screen.AllScreens ) {
                forms.Add(new ScreenForm(owner, screen));
            }
        }

        ~ScreenForms () => Dispose();

        public void Dispose () {
            foreach( ScreenForm form in forms ) {
                form.Close();
            }
            forms.Clear();
        }

        public bool IsOnAToolBar (Point point) => forms.FirstOrDefault(form => form.Bounds.Contains(point)) == null;

    }

}
