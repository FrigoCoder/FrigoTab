using System.Windows.Forms;

namespace FrigoTab {

    public class FrigoForm : Form {

        public WindowHandle WindowHandle => new WindowHandle(Handle);
        protected WindowExStyles ExStyle = WindowExStyles.ToolWindow;

        protected override CreateParams CreateParams {
            get {
                CreateParams createParams = base.CreateParams;
                createParams.ExStyle |= (int) ExStyle;
                createParams.X = Bounds.X;
                createParams.Y = Bounds.Y;
                createParams.Width = Bounds.Width;
                createParams.Height = Bounds.Height;
                return createParams;
            }
        }

        protected FrigoForm () {
            FormBorderStyle = FormBorderStyle.None;
            ShowInTaskbar = false;
            TopMost = true;
            SetStyle(ControlStyles.SupportsTransparentBackColor, false);
            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint, true);
        }

        protected override void OnPaint (PaintEventArgs e) {
        }

        protected override void OnPaintBackground (PaintEventArgs e) {
        }

    }

}
