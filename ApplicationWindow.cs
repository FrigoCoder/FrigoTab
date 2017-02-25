using System;
using System.Drawing;

namespace FrigoTab {

    public class ApplicationWindow : IDisposable {

        public readonly WindowHandle Application;
        public readonly int Index;
        public readonly Icon Icon;
        private Rectangle _bounds;
        private readonly Thumbnail _thumbnail;
        private readonly Overlay _overlay;
        private bool _selected;

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

        public Rectangle Bounds {
            get { return _bounds; }
            set {
                _bounds = value;
                _thumbnail.SetDestinationRect(new ScreenRect(value));
                _overlay.Bounds = value;
                _overlay.Draw();
            }
        }

        public bool Visible {
            set { _overlay.Visible = value; }
        }

        public ApplicationWindow (Session session, WindowHandle application, int index) {
            Application = application;
            Index = index;

            Icon = IconManager.IconFromSendMessageTimeout(Application);
            Icon = Icon ?? IconManager.IconFromGetClassLongPtr(Application);
            Icon = Icon ?? Program.Icon;

            _thumbnail = new Thumbnail(application, session.Handle);
            _overlay = new Overlay(session, this);
        }

        public void Dispose () {
            _overlay.Close();
            _thumbnail.Dispose();
        }

        public Size GetSourceSize () {
            return _thumbnail.GetSourceSize();
        }

    }

}
