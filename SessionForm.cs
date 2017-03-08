using System;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

using Microsoft.Win32;

namespace FrigoTab {

    public class SessionForm : FrigoForm {

        private Backgrounds _backgrounds;
        private ScreenForms _screenForms;
        private ApplicationWindows _applications;
        private bool _active;

        public SessionForm () {
            Bounds = Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;
            SystemEvents.DisplaySettingsChanged += RefreshDisplay;
        }

        public void HandleKeyEvents (KeyHookEventArgs e) {
            if( e.Key == (Keys.Alt | Keys.Tab) ) {
                e.Handled = true;
                BeginSession();
            }
            if( !_active ) {
                return;
            }
            if( ((Keys.D1 <= e.Key) && (e.Key <= Keys.D9)) || ((Keys.NumPad1 <= e.Key) && (e.Key <= Keys.NumPad9)) ) {
                e.Handled = true;
                _applications.SelectByIndex((char) e.Key - '1');
                ActivateEndSession();
            }
            if( (e.Key == Keys.Escape) || (e.Key == (Keys.Alt | Keys.F4)) ) {
                e.Handled = true;
                EndSession();
            }
        }

        public void HandleMouseEvents (MouseHookEventArgs e) {
            if( !_active ) {
                return;
            }
            _applications.SelectByPoint(e.Point);
            if( e.Click ) {
                if( _screenForms.IsOnAToolBar(e.Point) ) {
                    EndSession();
                } else {
                    ActivateEndSession();
                }
            }
        }

        protected override void Dispose (bool disposing) {
            EndSession();
            SystemEvents.DisplaySettingsChanged -= RefreshDisplay;
            base.Dispose(disposing);
        }

        protected override void SetVisibleCore (bool value) {
            if( !IsHandleCreated ) {
                CreateHandle();
                value = false;
            }
            base.SetVisibleCore(value);
        }

        private void BeginSession () {
            if( _active ) {
                return;
            }

            WindowFinder finder = new WindowFinder();
            if( finder.Windows.Count == 0 ) {
                return;
            }

            _backgrounds = new Backgrounds(this, finder);
            _screenForms = new ScreenForms(this);
            _applications = new ApplicationWindows(this, finder);

            Visible = true;
            _screenForms.Visible = true;
            _applications.Visible = true;
            ((WindowHandle) Handle).SetForeground();

            _active = true;
        }

        private void EndSession () {
            if( !_active ) {
                return;
            }
            _active = false;

            _applications.Visible = false;
            _screenForms.Visible = false;
            Visible = false;

            _applications.Dispose();
            _screenForms.Dispose();
            _backgrounds.Dispose();
        }

        private void RefreshDisplay (object sender, EventArgs e) {
            Bounds = Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);
            if( !_active ) {
                return;
            }
            EndSession();
            BeginSession();
        }

        private void ActivateEndSession () {
            if( _applications.Selected == null ) {
                return;
            }
            _applications.Selected.SetForeground();
            EndSession();
        }

    }

}
