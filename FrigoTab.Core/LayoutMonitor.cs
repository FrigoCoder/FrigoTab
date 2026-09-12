using System;

namespace FrigoTab.Core {

    /// <summary>
    /// Display bounds and usable working area for one monitor.
    /// </summary>
    public sealed class LayoutMonitor {

        public LayoutMonitor (string id, ScreenRectangle bounds, ScreenRectangle workingArea) {
            if( String.IsNullOrEmpty(id) ) {
                throw new ArgumentException("A layout monitor needs an id.", nameof(id));
            }
            Id = id;
            Bounds = bounds;
            WorkingArea = workingArea;
        }

        public string Id { get; }
        public ScreenRectangle Bounds { get; }
        public ScreenRectangle WorkingArea { get; }

    }

}
