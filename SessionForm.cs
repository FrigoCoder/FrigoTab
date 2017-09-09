using System;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

using Microsoft.Win32;

namespace FrigoTab {

    public class SessionForm : FrigoForm {

        private BackgroundWindows _backgrounds;
        private ScreenForms _screenForms;
        private ApplicationWindows _applications;
        private bool _active;

        public SessionForm () {
            Bounds = GetScreenBounds();
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;
            SystemEvents.DisplaySettingsChanged += RefreshDisplay;
        }

        public void HandleKeyEvents (KeyHookEventArgs e) {
            if( e.Key == (Keys.Alt | Keys.Tab) ) {
                e.Handled = true;
                PostMessage(Wm.BeginSession, 0, 0);
            }
            if( !_active ) {
                return;
            }
            if( e.Key == Keys.Escape || e.Key == (Keys.Alt | Keys.F4) ) {
                e.Handled = true;
                PostMessage(Wm.EndSession, 0, 0);
            }
            if( Keys.D1 <= e.Key && e.Key <= Keys.D9 || Keys.NumPad1 <= e.Key && e.Key <= Keys.NumPad9 ) {
                e.Handled = true;
                PostMessage(Wm.KeyPressed, (int) e.Key, 0);
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

        protected override void WndProc (ref Message m) {
            switch( m.Msg ) {
                case (int) Wm.BeginSession:
                    BeginSession();
                    break;
                case (int) Wm.EndSession:
                    EndSession();
                    break;
                case (int) Wm.KeyPressed:
                    Keys key = (Keys) m.WParam;
                    _applications.SelectByIndex((char) key - '1');
                    ActivateEndSession();
                    break;
                default:
                    base.WndProc(ref m);
                    break;
            }
        }

        private void PostMessage (Wm wm, int wParam, int lParam) {
            ((WindowHandle) Handle).PostMessage((int) wm, wParam, lParam);
        }

        private void BeginSession () {
            if( _active ) {
                return;
            }

            WindowFinder finder = new WindowFinder();
            if( finder.Windows.Count == 0 ) {
                return;
            }

            _backgrounds = new BackgroundWindows(this, finder);
            _screenForms = new ScreenForms(this);
            _applications = new ApplicationWindows(this, finder);

            _applications.SelectByIndex(0);

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

        private void ActivateEndSession () {
            if( _applications.Selected == null ) {
                return;
            }
            WindowHandle selected = _applications.Selected.Application;
            EndSession();
            selected.SetForeground();
        }

        private void RefreshDisplay (object sender, EventArgs e) {
            if( !_active ) {
                return;
            }
            if( Bounds != GetScreenBounds() ) {
                EndSession();
                Bounds = GetScreenBounds();
                BeginSession();
            }
        }

        private enum Wm {

            User = 0x4000,
            BeginSession = User + 1,
            EndSession = User + 2,
            KeyPressed = User + 3

        }

        private static Rectangle GetScreenBounds () {
            return Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);
        }

    }

}
