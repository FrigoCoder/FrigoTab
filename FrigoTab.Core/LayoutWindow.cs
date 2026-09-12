using System;

namespace FrigoTab.Core {

    /// <summary>
    /// A window candidate and the monitor to which its layout belongs.
    /// </summary>
    public sealed class LayoutWindow {

        public LayoutWindow (string id, string monitorId, ScreenRectangle bounds) {
            if( String.IsNullOrEmpty(id) ) {
                throw new ArgumentException("A layout window needs an id.", nameof(id));
            }
            if( String.IsNullOrEmpty(monitorId) ) {
                throw new ArgumentException("A layout window needs a monitor id.", nameof(monitorId));
            }
            Id = id;
            MonitorId = monitorId;
            Bounds = bounds;
        }

        public string Id { get; }
        public string MonitorId { get; }
        public ScreenRectangle Bounds { get; }

    }

}
