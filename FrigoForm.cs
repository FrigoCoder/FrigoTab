﻿using System;
using System.Windows.Forms;

namespace FrigoTab {

    public class FrigoForm : Form, IDisposable {

        protected WindowExStyles ExStyle = WindowExStyles.ToolWindow | WindowExStyles.NoActivate;

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

        public FrigoForm () {
            FormBorderStyle = FormBorderStyle.None;
            ShowInTaskbar = false;
            TopMost = true;
            SetStyle(ControlStyles.SupportsTransparentBackColor, false);
            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint, true);
        }

        public new virtual void Dispose () {
            Close();
            base.Dispose();
        }

        protected override void OnPaintBackground (PaintEventArgs e) {
        }

    }

}
