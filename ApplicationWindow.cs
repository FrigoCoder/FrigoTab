using System;
using System.Drawing;

namespace FrigoTab {

    public class ApplicationWindow : IDisposable {

        public readonly WindowHandle Application;
        public readonly int Index;
        public readonly Icon Icon;
        public readonly Overlay Overlay;
        private Rectangle _bounds;
        private readonly Thumbnail _thumbnail;
        private bool _selected;

        public bool Selected {
            get { return _selected; }
            set {
                if( _selected == value ) {
                    return;
                }
                _selected = value;
                Overlay.Draw();
            }
        }

        public Rectangle Bounds {
            get { return _bounds; }
            set {
                _bounds = value;
                _thumbnail.SetDestinationRect(new ScreenRect(value));
                Overlay.Bounds = value;
                Overlay.Draw();
            }
        }

        public ApplicationWindow (Session session, WindowHandle application, int index) {
            Application = application;
            Index = index;

            Icon = IconManager.IconFromSendMessageTimeout(Application);
            Icon = Icon ?? IconManager.IconFromGetClassLongPtr(Application);
            Icon = Icon ?? Program.Icon;

            _thumbnail = new Thumbnail(application, session.Handle);
            Overlay = new Overlay(session, this);
        }

        public void Dispose () {
            Overlay.Close();
            _thumbnail.Dispose();
        }

        public Size GetSourceSize () {
            return _thumbnail.GetSourceSize();
        }

    }

}
