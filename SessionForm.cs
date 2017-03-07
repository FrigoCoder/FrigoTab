using System;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

using Microsoft.Win32;

namespace FrigoTab {

    public class SessionForm : FrigoForm {

        private readonly Backgrounds _backgrounds;
        private readonly ScreenForms _screenForms;
        private readonly Applications _applications;
        private bool _active;

        public SessionForm () {
            Bounds = Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;

            _backgrounds = new Backgrounds(this);
            _screenForms = new ScreenForms(this);
            _applications = new Applications(this);

            SystemEvents.DisplaySettingsChanged += RefreshSession;
        }

        public void BeginSession () {
            if( _active ) {
                return;
            }

            _backgrounds.Populate();
            _screenForms.Populate();
            _applications.Populate();

            Visible = true;
            _screenForms.Visible = true;
            _applications.Visible = true;
            ((WindowHandle) Handle).SetForeground();

            _active = true;
        }

        public void EndSession () {
            _active = false;

            _applications.Visible = false;
            _screenForms.Visible = false;
            Visible = false;

            _applications.Dispose();
            _screenForms.Dispose();
            _backgrounds.Dispose();
        }

        public void RefreshSession (object sender, EventArgs e) {
            if( _active ) {
                EndSession();
                BeginSession();
            }
        }

        public void HandleKeyEvents (KeyHookEventArgs e) {
            if( e.Key == (Keys.Alt | Keys.Tab) ) {
                e.Handled = true;
                BeginSession();
            }
            if( _active ) {
                if( ((Keys.D1 <= e.Key) && (e.Key <= Keys.D9)) || ((Keys.NumPad1 <= e.Key) && (e.Key <= Keys.NumPad9)) ) {
                    e.Handled = true;
                    _applications.SelectByIndex((char) e.Key - '1');
                    ActivateEndSession();
                }
                if( e.Key == Keys.Escape ) {
                    e.Handled = true;
                    EndSession();
                }
                if( e.Key == (Keys.Alt | Keys.F4) ) {
                    e.Handled = true;
                }
            }
        }

        public void HandleMouseEvents (MouseHookEventArgs e) {
            if( _active ) {
                _applications.SelectByPoint(e.Point);
                if( e.Click ) {
                    if( _screenForms.IsOnAToolBar(e.Point) ) {
                        EndSession();
                    } else {
                        ActivateEndSession();
                    }
                }
            }
        }

        protected override void Dispose (bool disposing) {
            EndSession();
            SystemEvents.DisplaySettingsChanged -= RefreshSession;
            base.Dispose(disposing);
        }

        protected override void SetVisibleCore (bool value) {
            if( !IsHandleCreated ) {
                CreateHandle();
                value = false;
            }
            base.SetVisibleCore(value);
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
