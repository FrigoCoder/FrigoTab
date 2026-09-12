using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Windows.Forms;

namespace FrigoTab {

    /// <summary>
    /// Captures the visible virtual desktop once, before the switcher is
    /// shown, and paints that image as the switcher background.
    /// </summary>
    public sealed class DesktopSnapshot : IDisposable {

        private readonly Rectangle sourceBounds;
        private Bitmap image;
        private bool disposed;

        public DesktopSnapshot (Rectangle sourceBounds) {
            this.sourceBounds = sourceBounds;
            Capture();
        }

        public void Draw (Graphics graphics, Rectangle destinationBounds) {
            if( disposed ) {
                throw new ObjectDisposedException(nameof(DesktopSnapshot));
            }

            graphics.Clear(Color.Black);
            if( image == null || destinationBounds.Width <= 0 || destinationBounds.Height <= 0 ) {
                return;
            }

            if( destinationBounds.Size == image.Size ) {
                graphics.DrawImageUnscaled(image, destinationBounds.Location);
                return;
            }

            graphics.DrawImage(
                image,
                destinationBounds,
                0,
                0,
                image.Width,
                image.Height,
                GraphicsUnit.Pixel);
        }

        public void Dispose () {
            if( disposed ) {
                return;
            }

            disposed = true;
            Bitmap currentImage = image;
            image = null;
            currentImage?.Dispose();
            GC.SuppressFinalize(this);
        }

        private void Capture () {
            if( sourceBounds.Width <= 0 || sourceBounds.Height <= 0 ) {
                return;
            }

            Bitmap captured = null;
            try {
                captured = new Bitmap(
                    sourceBounds.Width,
                    sourceBounds.Height,
                    PixelFormat.Format32bppPArgb);

                using( Graphics graphics = Graphics.FromImage(captured) ) {
                    graphics.CopyFromScreen(
                        sourceBounds.Left,
                        sourceBounds.Top,
                        0,
                        0,
                        sourceBounds.Size,
                        CopyPixelOperation.SourceCopy);
                }

                image = captured;
                captured = null;
            }
            catch {
                // Some desktops expose protected or unavailable surfaces.
                // A missing snapshot is intentionally rendered as solid black
                // so opening the switcher remains fail-open.
            }
            finally {
                captured?.Dispose();
            }
        }

    }

}
