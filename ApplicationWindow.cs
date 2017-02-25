using System;
using System.Drawing;

namespace FrigoTab {

    public class ApplicationWindow : IDisposable {

        public readonly WindowHandle Application;
        public readonly Rectangle Bounds;
        public readonly int Index;
        public readonly Icon Icon;
        public readonly Overlay Overlay;
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

        public ApplicationWindow (Session session, WindowHandle application, ScreenRect bounds, int index) {
            Application = application;
            Bounds = bounds.ToRectangle();
            Index = index;

            Icon = IconManager.IconFromSendMessageTimeout(Application);
            Icon = Icon ?? IconManager.IconFromGetClassLongPtr(Application);
            Icon = Icon ?? Program.Icon;

            _thumbnail = new Thumbnail(application, session.Handle, bounds);
            Overlay = new Overlay(session, this);
        }

        public void Dispose () {
            Overlay.Close();
            _thumbnail.Dispose();
        }

    }

}
