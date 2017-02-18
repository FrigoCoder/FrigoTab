using System;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Text;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class Overlay {

        private readonly ApplicationWindow _window;

        public Overlay (ApplicationWindow window) {
            _window = window;
            Draw();
        }

        public void Draw () {
            IntPtr screenDc = GetDC(IntPtr.Zero);
            IntPtr memDc = CreateCompatibleDC(screenDc);
            IntPtr hBitmap = CreateCompatibleBitmap(screenDc, _window.Bounds.Width, _window.Bounds.Height);
            IntPtr hOldBitmap = SelectObject(memDc, hBitmap);
            using( Graphics graphics = Graphics.FromHdc(memDc) ) {
                Render(graphics);
            }
            UpdateLayeredWindow(memDc);
            SelectObject(memDc, hOldBitmap);
            DeleteDC(memDc);
            DeleteObject(hBitmap);
            ReleaseDC(IntPtr.Zero, screenDc);
        }

        private void Render (Graphics graphics) {
            graphics.PixelOffsetMode = PixelOffsetMode.HighQuality;
            graphics.SmoothingMode = SmoothingMode.AntiAlias;
            RenderFrame(graphics);
            RenderTitle(graphics);
            RenderNumber(graphics);
        }

        private void RenderFrame (Graphics graphics) {
            if( _window.Selected ) {
                FillRectangle(graphics, graphics.VisibleClipBounds, Color.FromArgb(128, 0, 0, 255));
            }
        }

        private void RenderTitle (Graphics graphics) {
            const int pad = 8;

            Icon icon = _window.Icon;
            string text = _window.WindowHandle.GetWindowText();

            Font font = new Font("Segoe UI", 11f);
            SizeF textSize = graphics.MeasureString(text, font);

            float width = pad + icon.Width + pad + textSize.Width + pad;
            float height = pad + Math.Max(icon.Height, textSize.Height) + pad;

            RectangleF background = new RectangleF(graphics.VisibleClipBounds.Location, new SizeF(width, height));
            FillRectangle(graphics, background, Color.Black);

            {
                float x = background.X + pad;
                float y = Center(icon.Size, background).Y;
                graphics.DrawIcon(icon, (int) x, (int) y);
            }

            using( Brush brush = new SolidBrush(Color.White) ) {
                float x = background.X + pad + icon.Width + pad;
                float y = Center(textSize, background).Y;
                graphics.DrawString(text, font, brush, x, y);
            }
        }

        private void RenderNumber (Graphics graphics) {
            string text = (_window.Index + 1).ToString();

            Font font = new Font("Segoe UI", 72f, FontStyle.Bold);
            SizeF textSize = graphics.MeasureString(text, font);

            RectangleF background = Center(textSize, graphics.VisibleClipBounds);
            FillRectangle(graphics, background, Color.Black);

            graphics.TextRenderingHint = TextRenderingHint.AntiAliasGridFit;
            using( Brush brush = new SolidBrush(Color.White) ) {
                graphics.DrawString(text, font, brush, background);
            }
        }

        private void UpdateLayeredWindow (IntPtr hdc) {
            Point pptDst = _window.Bounds.Location;
            Size pSize = _window.Bounds.Size;
            Point pptSrc = Point.Empty;
            byte[] pblend = {
                0,
                0,
                0xff,
                1
            };
            UpdateLayeredWindow(_window.Handle, IntPtr.Zero, ref pptDst, ref pSize, hdc, ref pptSrc, 0, pblend, 2);
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

        [DllImport ("user32.dll")]
        private static extern IntPtr GetDC (IntPtr hwnd);

        [DllImport ("gdi32.dll")]
        private static extern IntPtr CreateCompatibleDC (IntPtr hdc);

        [DllImport ("gdi32.dll")]
        private static extern IntPtr CreateCompatibleBitmap (IntPtr hdc, int cx, int cy);

        [DllImport ("gdi32.dll")]
        private static extern IntPtr SelectObject (IntPtr hdc, IntPtr hobj);

        [DllImport ("gdi32.dll")]
        private static extern bool DeleteDC (IntPtr hdc);

        [DllImport ("gdi32.dll")]
        private static extern bool DeleteObject (IntPtr hobj);

        [DllImport ("user32.dll")]
        private static extern int ReleaseDC (IntPtr hwnd, IntPtr hdc);

        [DllImport ("user32.dll")]
        private static extern bool UpdateLayeredWindow (IntPtr hwnd,
            IntPtr hdcDst,
            ref Point pptDst,
            ref Size psize,
            IntPtr hdcSrc,
            ref Point pptSrc,
            int crKey,
            byte[] pblend,
            int dwFlags);

    }

}
