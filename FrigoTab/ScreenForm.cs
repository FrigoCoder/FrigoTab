using System.Windows.Forms;

namespace FrigoTab {

    public class ScreenForm : FrigoForm {

        public ScreenForm (Form owner, Screen screen) {
            Owner = owner;
            Bounds = screen.WorkingArea;
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;
        }

    }

}
