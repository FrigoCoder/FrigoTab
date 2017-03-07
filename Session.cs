using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;

namespace FrigoTab {

    public class Session : FrigoForm, IDisposable {

        private readonly Backgrounds _backgrounds;
        private readonly ScreenForms _screenForms;
        private readonly IList<ApplicationWindow> _applications = new List<ApplicationWindow>();
        private ApplicationWindow _selectedWindow;

        public Session (WindowFinder finder) {
            Bounds = Screen.AllScreens.Select(screen => screen.Bounds).Aggregate(Rectangle.Union);
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;

            _backgrounds = new Backgrounds(this);
            _backgrounds.Populate();

            _screenForms = new ScreenForms(this);
            _screenForms.Populate();

            foreach( WindowHandle window in finder.Windows ) {
                _applications.Add(new ApplicationWindow(this, window, _applications.Count));
            }

            FrigoTab.Layout.LayoutWindows(_applications);

            Visible = true;
            _screenForms.Visible = true;
            foreach( ApplicationWindow window in _applications ) {
                window.Visible = true;
            }
            ((WindowHandle) Handle).SetForeground();
        }

        private ApplicationWindow SelectedWindow {
            get { return _selectedWindow; }
            set {
                if( _selectedWindow == value ) {
                    return;
                }
                if( _selectedWindow != null ) {
                    _selectedWindow.Selected = false;
                }
                _selectedWindow = value;
                if( _selectedWindow != null ) {
                    _selectedWindow.Selected = true;
                }
            }
        }

        public new void Dispose () {
            foreach( ApplicationWindow window in _applications ) {
                window.Dispose();
            }
            _screenForms.Dispose();
            _backgrounds.Dispose();
            Close();
        }

        public void HandleKeyEvents (KeyHookEventArgs e) {
            if( ((Keys.D1 <= e.Key) && (e.Key <= Keys.D9)) || ((Keys.NumPad1 <= e.Key) && (e.Key <= Keys.NumPad9)) ) {
                int index = (char) e.Key - '1';
                if( (index >= 0) && (index < _applications.Count) ) {
                    e.Handled = true;
                    SelectedWindow = _applications[index];
                    End();
                }
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
            SelectedWindow = _applications.FirstOrDefault(window => window.Bounds.Contains(e.Point));
            if( e.Click ) {
                if( _screenForms.IsOnAToolBar(e.Point) ) {
                    Dispose();
                } else {
                    End();
                }
            }
        }

        private void End () {
            if( SelectedWindow == null ) {
                return;
            }
            SelectedWindow.SetForeground();
            Dispose();
        }

    }

}
