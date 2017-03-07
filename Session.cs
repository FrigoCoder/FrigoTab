using System;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Session : FrigoForm, IDisposable {

        private readonly Backgrounds _backgrounds;
        private readonly ScreenForms _screenForms;
        private readonly Applications _applications;
        private ApplicationWindow _selectedWindow;

        public Session (WindowFinder finder) {
            Bounds = Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;

            _backgrounds = new Backgrounds(this);
            _backgrounds.Populate();

            _screenForms = new ScreenForms(this);
            _screenForms.Populate();

            _applications = new Applications(this);
            _applications.Populate();

            Visible = true;
            _screenForms.Visible = true;
            _applications.Visible = true;
            ((WindowHandle) Handle).SetForeground();
        }

        public new void Dispose () {
            _applications.Dispose();
            _screenForms.Dispose();
            _backgrounds.Dispose();
            Close();
        }

        public void HandleKeyEvents (KeyHookEventArgs e) {
            if( ((Keys.D1 <= e.Key) && (e.Key <= Keys.D9)) || ((Keys.NumPad1 <= e.Key) && (e.Key <= Keys.NumPad9)) ) {
                e.Handled = true;
                _applications.SelectByIndex((char) e.Key - '1');
                End();
            }
            if( e.Key == Keys.Escape ) {
                e.Handled = true;
                Dispose();
            }
            if( e.Key == (Keys.Alt | Keys.F4) ) {
                e.Handled = true;
            }
        }

        public void HandleMouseEvents (MouseHookEventArgs e) {
            _applications.SelectByPoint(e.Point);
            if( e.Click ) {
                if( _screenForms.IsOnAToolBar(e.Point) ) {
                    Dispose();
                } else {
                    End();
                }
            }
        }

        private void End () {
            if( _applications.Selected == null ) {
                return;
            }
            _applications.Selected.SetForeground();
            Dispose();
        }

    }

}
