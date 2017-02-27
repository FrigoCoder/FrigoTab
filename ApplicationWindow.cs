using System;
using System.Drawing;

namespace FrigoTab {

    public class ApplicationWindow : IDisposable {

        public readonly WindowHandle Application;
        public readonly int Index;
        private Icon _icon;
        private readonly Thumbnail _thumbnail;
        private readonly Overlay _overlay;
        private Rectangle _bounds;
        private bool _selected;

        public ApplicationWindow (Session session, WindowHandle application, int index) {
            Application = application;
            Index = index;
            _icon = IconManager.IconFromGetClassLongPtr(Application) ?? Program.Icon;
            _thumbnail = new Thumbnail(application, session.Handle);
            _overlay = new Overlay(session, this);
            IconManager.Register(this, Application);
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

        public Icon Icon {
            get { return _icon; }
            set {
                _icon = value;
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

        public Size GetSourceSize () {
            return _thumbnail.GetSourceSize();
        }

    }

}
