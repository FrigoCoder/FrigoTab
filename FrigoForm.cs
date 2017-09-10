using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class FrigoForm : Form {

        protected WindowExStyles ExStyle = WindowExStyles.ToolWindow;

        protected FrigoForm () {
            FormBorderStyle = FormBorderStyle.None;
            ShowInTaskbar = false;
            TopMost = true;
            SetStyle(ControlStyles.SupportsTransparentBackColor, false);
            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint, true);
        }

        protected new WindowHandle Handle => base.Handle;

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

        protected override void OnPaint (PaintEventArgs e) {
        }

        protected override void OnPaintBackground (PaintEventArgs e) {
        }

        protected override void SetVisibleCore (bool value) {
            if( IsHandleCreated ) {
                base.SetVisibleCore(value);
                return;
            }
            CreateHandle();
            if( !IsCalledFromRunMessageLoop(new StackTrace()) ) {
                base.SetVisibleCore(value);
            }
        }

        private static bool IsCalledFromRunMessageLoop (StackTrace trace) {
            IEnumerable<string> expected = new[] {
                "SetVisibleCore",
                "RunMessageLoopInner",
                "RunMessageLoop",
                "Main"
            };
            IEnumerable<string> actual = trace.GetFrames()?.Select(frame => frame.GetMethod().Name) ?? new string[0];
            return expected.SequenceEqual(actual);
        }

    }

}
