using System;
using System.Drawing;
using System.Windows.Forms;

namespace FrigoTab {

    public class ApplicationWindow : FrigoForm, IDisposable {

        public readonly int Index;
        private readonly WindowHandle _application;
        private Icon _appIcon;
        private readonly Thumbnail _thumbnail;
        private readonly Overlay _overlay;
        private bool _selected;

        public ApplicationWindow (Session session, WindowHandle application, int index) {
            Owner = session;
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered;
            _application = application;
            Index = index;
            _appIcon = IconManager.IconFromGetClassLongPtr(_application) ?? Program.Icon;
            _thumbnail = new Thumbnail(application, session.Handle);
            _overlay = new Overlay(this);
            IconManager.Register(this, _application);
        }

        public new Rectangle Bounds {
            get { return base.Bounds; }
            set {
                base.Bounds = value;
                _thumbnail.Update(new ScreenRect(value));
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

        public new void Dispose () {
            IconManager.Unregister(this);
            Close();
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
