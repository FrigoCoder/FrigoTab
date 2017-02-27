using System;
using System.Drawing;
using System.Windows.Forms;

namespace FrigoTab {

    public class ApplicationWindow : IDisposable {

        public readonly int Index;
        private readonly WindowHandle _application;
        private Icon _appIcon;
        private readonly Thumbnail _thumbnail;
        private readonly Overlay _overlay;
        private Rectangle _bounds;
        private bool _selected;

        public ApplicationWindow (Session session, WindowHandle application, int index) {
            _application = application;
            Index = index;
            _appIcon = IconManager.IconFromGetClassLongPtr(_application) ?? Program.Icon;
            _thumbnail = new Thumbnail(application, session.Handle);
            _overlay = new Overlay(session, this);
            IconManager.Register(this, _application);
        }

        public Rectangle Bounds {
            get { return _bounds; }
            set {
                _bounds = value;
                _thumbnail.Update(new ScreenRect(value));
                _overlay.Bounds = value;
                _overlay.Draw();
            }
        }

        public Icon AppIcon {
            get { return _appIcon; }
            set {
                _appIcon = value;
                _overlay.Draw();
            }
        }

        public bool Selected {
            get { return _selected; }
            set {
                if( _selected == value ) {
                    return;
                }
                _selected = value;
                _overlay.Draw();
            }
        }

        public bool Visible {
            set { _overlay.Visible = value; }
        }

        public void Dispose () {
            IconManager.Unregister(this);
            _overlay.Close();
            _thumbnail.Dispose();
        }

        public Screen GetScreen () {
            return Screen.FromHandle(_application);
        }

        public string GetWindowText () {
            return _application.GetWindowText();
        }

        public void SetForeground () {
            _application.SetForeground();
        }

        public Size GetSourceSize () {
            return _thumbnail.GetSourceSize();
        }

    }

}
