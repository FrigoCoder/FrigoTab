using System;
using System.Collections.Generic;
using System.Drawing;
using System.Windows.Forms;

namespace FrigoTab {

    public class LayoutScreen {

        private readonly Screen screen;
        private readonly IList<ApplicationWindow> windows;
        private readonly int columns;
        private readonly int rows;

        public LayoutScreen (Screen screen, IList<ApplicationWindow> windows) {
            this.screen = screen;
            this.windows = windows;
            columns = (int) Math.Ceiling(Math.Sqrt(windows.Count));
            rows = windows.Count == 0 ? 0 : (int) Math.Ceiling((double) windows.Count / columns);
        }

        public void Layout () {
            for( int i = 0; i < windows.Count; i++ ) {
                float xMargin = screen.Bounds.Width * 0.005f;
                float yMargin = screen.Bounds.Height * 0.005f;
                RectangleF cell = GetCellBounds(i % columns, i / columns, xMargin, yMargin);
                RectangleF bounds = CenterWithin(windows[i].GetSourceSize(), cell);
                bounds.Offset(screen.WorkingArea.Location);
                windows[i].Bounds = Rectangle.Round(bounds);
            }
        }

        private RectangleF GetCellBounds (int column, int row, float xMargin, float yMargin) {
            SizeF size = new SizeF((float) screen.WorkingArea.Width / columns, (float) screen.WorkingArea.Height / rows);
            PointF location = new PointF(column * size.Width + xMargin, row * size.Height + yMargin);
            return new RectangleF(location, new SizeF(size.Width - 2 * xMargin, size.Height - 2 * yMargin));
        }

        private static RectangleF CenterWithin (Size rectSize, RectangleF bounds) {
            SizeF size = ScaleWithin(rectSize, bounds.Size);
            SizeF margin = bounds.Size - size;
            PointF location = new PointF(bounds.X + margin.Width / 2, bounds.Y + margin.Height / 2);
            return new RectangleF(location, size);
        }

        private static SizeF ScaleWithin (SizeF size, SizeF bounds) {
            size = Scale(size, Math.Min(bounds.Width, size.Width) / size.Width);
            size = Scale(size, Math.Min(bounds.Height, size.Height) / size.Height);
            return size;
        }

        private static SizeF Scale (SizeF size, float scale) => new SizeF(size.Width * scale, size.Height * scale);

    }

}
