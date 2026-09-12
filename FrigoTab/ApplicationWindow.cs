using System;
using System.Drawing;
using System.Diagnostics;
using System.Drawing.Drawing2D;
using System.Drawing.Text;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class ApplicationWindow : FrigoForm {

        private const int NonClientHitTestMessage = 0x0084;
        private static readonly IntPtr TransparentHitTest = new IntPtr(-1);

        public readonly WindowHandle Application;
        public Property<bool> Selected;
        private readonly int index;
        private readonly Thumbnail thumbnail;
        private readonly WindowIcon windowIcon;
        private readonly LayerUpdater layerUpdater;
        private bool disposed;

        public ApplicationWindow (FrigoForm owner, WindowHandle application, int index, Rectangle bounds) {
            Bounds = bounds;
            Owner = owner;
            ExStyle |= WindowExStyles.Transparent | WindowExStyles.Layered | WindowExStyles.NoActivate;
            Application = application;
            Selected.Changed += (x, y) => RenderOverlay();
            this.index = index;
            Thumbnail newThumbnail = null;
            LayerUpdater newLayerUpdater = null;
            WindowIcon newWindowIcon = null;
            try {
                newThumbnail = TryCreateThumbnail(application, owner.WindowHandle, Bounds);
                newLayerUpdater = new LayerUpdater(this);
                newWindowIcon = new WindowIcon(application);

                thumbnail = newThumbnail;
                layerUpdater = newLayerUpdater;
                windowIcon = newWindowIcon;
                windowIcon.Changed += RenderOverlay;
                RenderOverlay();
            }
            catch {
                // The form handle can already exist because LayerUpdater asks
                // for form.Handle. A constructor that fails after that point
                // has no caller-owned instance to dispose, so release both the
                // local native resources and the partially built Form here.
                try {
                    newWindowIcon?.Dispose();
                }
                catch {
                }
                try {
                    newLayerUpdater?.Dispose();
                }
                catch {
                }
                try {
                    newThumbnail?.Dispose();
                }
                catch {
                }
                try {
                    base.Dispose(true);
                }
                catch {
                }
                throw;
            }
        }

        protected override void Dispose (bool disposing) {
            if( disposed ) {
                base.Dispose(disposing);
                return;
            }
            disposed = true;
            if( windowIcon != null ) {
                windowIcon.Changed -= RenderOverlay;
                windowIcon.Dispose();
            }
            layerUpdater?.Dispose();
            thumbnail?.Dispose();
            base.Dispose(disposing);
        }

        public void SetSelected (bool value) => Selected.Value = value;

        public bool TryActivate () => Application.SetForeground();

        protected override void WndProc (ref System.Windows.Forms.Message m) {
            if( m.Msg == NonClientHitTestMessage ) {
                // Tiles are visual overlays owned by SessionForm. Returning
                // HTTRANSPARENT routes pointer input to that same-thread owner,
                // where the application-level hit test selects the tile.
                m.Result = TransparentHitTest;
                return;
            }
            base.WndProc(ref m);
        }

        private void RenderOverlay () => layerUpdater.Update(RenderOverlay);

        private void RenderOverlay (Graphics graphics) {
            graphics.PixelOffsetMode = PixelOffsetMode.HighQuality;
            graphics.SmoothingMode = SmoothingMode.AntiAlias;
            RenderFrame(graphics);
            RenderTitle(graphics);
            RenderNumber(graphics);
        }

        private void RenderFrame (Graphics graphics) {
            if( Selected.Value ) {
                FillRectangle(graphics, graphics.VisibleClipBounds, Color.FromArgb(128, 0, 0, 255));
            }
        }

        private void RenderTitle (Graphics graphics) {
            const int Pad = 8;

            Icon icon = windowIcon.Icon;
            string text = Application.GetWindowText();

            using( Font font = new Font("Segoe UI", 11f) ) {
                SizeF textSize = graphics.MeasureString(text, font);

                float width = Pad + icon.Width + Pad + textSize.Width + Pad;
                float height = Pad + Math.Max(icon.Height, textSize.Height) + Pad;

                RectangleF background = new RectangleF(graphics.VisibleClipBounds.Location, new SizeF(width, height));
                FillRectangle(graphics, background, Color.Black);

                {
                    float x = background.X + Pad;
                    float y = Center(icon.Size, background).Y;
                    graphics.DrawIcon(icon, (int) x, (int) y);
                }

                using( Brush brush = new SolidBrush(Color.White) ) {
                    float x = background.X + Pad + icon.Width + Pad;
                    float y = Center(textSize, background).Y;
                    graphics.DrawString(text, font, brush, x, y);
                }
            }
        }

        private void RenderNumber (Graphics graphics) {
            string text = (index + 1).ToString();

            using( Font font = new Font("Segoe UI", 72f, FontStyle.Bold) ) {
                SizeF textSize = graphics.MeasureString(text, font);

                RectangleF background = Center(textSize, graphics.VisibleClipBounds);
                FillRectangle(graphics, background, Color.Black);

                graphics.TextRenderingHint = TextRenderingHint.AntiAliasGridFit;
                using( Brush brush = new SolidBrush(Color.White) ) {
                    graphics.DrawString(text, font, brush, background);
                }
            }
        }

        private static Thumbnail TryCreateThumbnail (
            WindowHandle application,
            WindowHandle owner,
            Rectangle bounds) {
            Thumbnail result = null;
            try {
                result = new Thumbnail(application, owner);
                result.SetDestinationRect(new Rect(bounds).ScreenToClient(owner));
                return result;
            }
            catch( Exception exception ) when(
                exception is ExternalException ||
                exception is DllNotFoundException ||
                exception is EntryPointNotFoundException ) {
                result?.Dispose();
                Trace.WriteLine("DWM thumbnail unavailable; using the icon/title overlay fallback: " + exception);
                return null;
            }
        }

        private static void FillRectangle (Graphics graphics, RectangleF bounds, Color color) {
            PointF[] points = new PointF[5];
            points[0] = new PointF(bounds.Left, bounds.Top);
            points[1] = new PointF(bounds.Left, bounds.Top);
            points[2] = new PointF(bounds.Right, bounds.Top);
            points[3] = new PointF(bounds.Right, bounds.Bottom);
            points[4] = new PointF(bounds.Left, bounds.Bottom);
            using( Brush brush = new SolidBrush(color) ) {
                graphics.FillPolygon(brush, points);
            }
        }

        private static RectangleF Center (SizeF rect, RectangleF bounds) {
            SizeF margins = bounds.Size - rect;
            PointF location = new PointF(bounds.X + margins.Width / 2, bounds.Y + margins.Height / 2);
            return new RectangleF(location, rect);
        }

    }

}
