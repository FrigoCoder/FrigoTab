using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Forms;
using FrigoTab.Core;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [STATestClass]
    [DoNotParallelize]
    [TestCategory("Acceptance")]
    public class T20260914T223000Z_004_RealVisualParityAcceptanceTests {

        [TestInitialize]
        public void UsePerMonitorPhysicalCoordinates () {
            SetThreadDpiAwarenessContext((IntPtr) (-4));
        }

        [TestMethod]
        public void EveryRealPreviewUsesTheOriginalOwnedLayeredWindowTopology () {
            using( LiveVisualSession desktop = new LiveVisualSession() ) {
                desktop.Open();

                Assert.AreEqual(3, desktop.FixtureTiles.Count);
                foreach( ApplicationWindow tile in desktop.FixtureTiles.Values ) {
                    long style = GetWindowLongPtr(tile.Handle, ExtendedStyleIndex).ToInt64();
                    Assert.AreEqual(desktop.Form.Handle, GetWindow(tile.Handle, OwnerWindow));
                    Assert.IsTrue(tile.Visible);
                    AssertStyle(style, ToolWindowStyle, "tool window");
                    AssertStyle(style, TopMostStyle, "topmost");
                    AssertStyle(style, TransparentStyle, "transparent");
                    AssertStyle(style, LayeredStyle, "layered");
                    AssertStyle(style, NoActivateStyle, "no-activate");
                }
            }
        }

        [TestMethod]
        public void DwmPreviewFillsTheRealTileAndSelectedTileKeepsTheBlueAlphaOverlay () {
            using( LiveVisualSession desktop = new LiveVisualSession() ) {
                desktop.Open();
                ApplicationWindow red = desktop.FixtureTiles[Color.Red];
                ApplicationWindow green = desktop.FixtureTiles[Color.Lime];

                desktop.Select(red);
                Rectangle greenBounds = green.Bounds;
                int y = greenBounds.Top + greenBounds.Height * 3 / 4;
                Color leftEdge = desktop.ScreenPixel(greenBounds.Left + 1, y);
                Color rightEdge = desktop.ScreenPixel(greenBounds.Right - 2, y);
                Assert.IsTrue(IsNear(Color.Lime, leftEdge, 55), "The DWM image did not reach the tile's left edge: " + leftEdge);
                Assert.IsTrue(IsNear(Color.Lime, rightEdge, 55), "The DWM image did not reach the tile's right edge: " + rightEdge);

                Rectangle redBounds = red.Bounds;
                Color selected = desktop.ScreenPixel(
                    redBounds.Left + redBounds.Width * 3 / 4,
                    redBounds.Top + redBounds.Height * 3 / 4);
                Color expectedBlueBlend = Color.FromArgb(127, 0, 128);
                Assert.IsTrue(IsNear(expectedBlueBlend, selected, 75),
                    "The selected preview lost the original half-transparent blue fill: " + selected);

                Color unselected = desktop.ScreenPixel(
                    greenBounds.Left + greenBounds.Width * 3 / 4,
                    greenBounds.Top + greenBounds.Height * 3 / 4);
                Assert.IsTrue(IsNear(Color.Lime, unselected, 55),
                    "An unselected preview was unexpectedly tinted: " + unselected);
            }
        }

        [TestMethod]
        public void RealOverlayKeepsTheMeasuredTitleAndLargeCenteredNumber () {
            using( LiveVisualSession desktop = new LiveVisualSession() ) {
                desktop.Open();
                ApplicationWindow tile = desktop.FixtureTiles[Color.Red];
                desktop.Select(tile);

                using( Bitmap image = desktop.Capture(tile.Bounds) ) {
                    Assert.IsTrue(IsNear(Color.Black, image.GetPixel(2, 2), 35),
                        "The measured title backing no longer starts at the tile origin.");
                    Assert.IsFalse(IsNear(Color.Black, image.GetPixel(image.Width - 5, 5), 45),
                        "The title backing was changed into a full-width header.");

                    Rectangle titleArea = new Rectangle(4, 4, Math.Min(220, image.Width - 8), Math.Min(60, image.Height - 8));
                    Assert.IsTrue(CountPixels(image, titleArea, IsWhite) > 8,
                        "The real icon/title overlay did not render in the upper-left corner.");

                    Rectangle numberArea = new Rectangle(
                        image.Width / 4,
                        image.Height / 4,
                        image.Width / 2,
                        image.Height / 2);
                    List<Point> whiteNumberPixels = FindPixels(image, numberArea, IsWhite);
                    Assert.IsTrue(whiteNumberPixels.Count > 20, "The centered number has no visible white glyph.");
                    int glyphHeight = whiteNumberPixels.Max(point => point.Y) - whiteNumberPixels.Min(point => point.Y);
                    Assert.IsTrue(glyphHeight >= 35,
                        "The centered number is no longer rendered with the original large font. Height=" + glyphHeight);
                    Assert.IsTrue(CountPixels(image, numberArea, IsBlack) > 100,
                        "The centered number lost its tight black backing rectangle.");
                }
            }
        }

        private const int ExtendedStyleIndex = -20;
        private const uint OwnerWindow = 4;
        private const long TopMostStyle = 0x00000008;
        private const long TransparentStyle = 0x00000020;
        private const long ToolWindowStyle = 0x00000080;
        private const long LayeredStyle = 0x00080000;
        private const long NoActivateStyle = 0x08000000;

        private static void AssertStyle (long actual, long expected, string name) {
            Assert.AreEqual(expected, actual & expected, "The preview is not a " + name + ".");
        }

        private static bool IsNear (Color expected, Color actual, int tolerance) =>
            Math.Abs(expected.R - actual.R) +
            Math.Abs(expected.G - actual.G) +
            Math.Abs(expected.B - actual.B) <= tolerance;

        private static bool IsWhite (Color color) => color.R >= 210 && color.G >= 210 && color.B >= 210;

        private static bool IsBlack (Color color) => color.R <= 35 && color.G <= 35 && color.B <= 35;

        private static int CountPixels (Bitmap image, Rectangle bounds, Func<Color, bool> predicate) =>
            FindPixels(image, bounds, predicate).Count;

        private static List<Point> FindPixels (Bitmap image, Rectangle bounds, Func<Color, bool> predicate) {
            var result = new List<Point>();
            for( int y = Math.Max(0, bounds.Top); y < Math.Min(image.Height, bounds.Bottom); y++ ) {
                for( int x = Math.Max(0, bounds.Left); x < Math.Min(image.Width, bounds.Right); x++ ) {
                    if( predicate(image.GetPixel(x, y)) ) {
                        result.Add(new Point(x, y));
                    }
                }
            }
            return result;
        }

        private sealed class LiveVisualSession : IDisposable {

            private readonly List<Form> fixtures = new List<Form>();
            private readonly Dictionary<Color, WindowHandle> fixtureHandles = new Dictionary<Color, WindowHandle>();
            private readonly WindowHandle previousForeground;

            public LiveVisualSession () {
                previousForeground = WindowHandle.GetForegroundWindow();
                Form = new SessionForm();
                _ = Form.Handle;
                ShowFixture("R", Color.Red, 80, 80);
                ShowFixture("G", Color.Lime, 760, 120);
                ShowFixture("M", Color.Magenta, 1440, 160);
                Pump();
                Switcher = new SwitcherApplication((ISwitcherSessionPort) Form);
            }

            public SessionForm Form { get; }
            public SwitcherApplication Switcher { get; }

            public Dictionary<Color, ApplicationWindow> FixtureTiles => Application.OpenForms
                .Cast<Form>()
                .OfType<ApplicationWindow>()
                .Where(tile => ReferenceEquals(tile.Owner, Form))
                .Join(
                    fixtureHandles,
                    tile => tile.Application,
                    pair => pair.Value,
                    (tile, pair) => new {pair.Key, Tile = tile})
                .ToDictionary(item => item.Key, item => item.Tile);

            public void Open () {
                Assert.AreEqual(
                    KeyHandling.Consume,
                    Switcher.HandleKeyboard(new KeyboardInput(
                        SwitcherKey.Tab,
                        KeyTransition.Down,
                        alt: true,
                        shift: false,
                        injected: false)));
                WaitUntil(() => Form.Visible && FixtureTiles.Count == 3);
                Pump();
            }

            public void Select (ApplicationWindow tile) {
                Switcher.HandleMouseMove(new ScreenPoint(
                    tile.Bounds.Left + tile.Bounds.Width / 2,
                    tile.Bounds.Top + tile.Bounds.Height / 2));
                Pump();
            }

            public Color ScreenPixel (int x, int y) {
                DwmFlush();
                using( Bitmap pixel = new Bitmap(1, 1) ) {
                    using( Graphics graphics = Graphics.FromImage(pixel) ) {
                        graphics.CopyFromScreen(x, y, 0, 0, new Size(1, 1));
                    }
                    return pixel.GetPixel(0, 0);
                }
            }

            public Bitmap Capture (Rectangle bounds) {
                DwmFlush();
                var image = new Bitmap(bounds.Width, bounds.Height);
                using( Graphics graphics = Graphics.FromImage(image) ) {
                    graphics.CopyFromScreen(bounds.Location, Point.Empty, bounds.Size);
                }
                return image;
            }

            public void Dispose () {
                Switcher.Close();
                Form.Dispose();
                foreach( Form fixture in fixtures ) {
                    fixture.Close();
                    fixture.Dispose();
                }
                if( previousForeground != WindowHandle.Null ) {
                    previousForeground.SetForeground();
                }
                Pump();
            }

            private void ShowFixture (string title, Color color, int left, int top) {
                Form fixture = new Form {
                    Text = title,
                    BackColor = color,
                    Bounds = new Rectangle(left, top, 640, 480),
                    FormBorderStyle = FormBorderStyle.None,
                    StartPosition = FormStartPosition.Manual,
                    ShowInTaskbar = true
                };
                fixture.Show();
                fixture.Refresh();
                fixtures.Add(fixture);
                fixtureHandles.Add(color, new WindowHandle(fixture.Handle));
            }

            private static void Pump () {
                Application.DoEvents();
                Thread.Sleep(40);
                Application.DoEvents();
                DwmFlush();
            }

            private static void WaitUntil (Func<bool> condition) {
                Stopwatch timeout = Stopwatch.StartNew();
                while( timeout.Elapsed < TimeSpan.FromSeconds(5) ) {
                    if( condition() ) {
                        return;
                    }
                    Pump();
                }
                Assert.Fail("Timed out waiting for the real visual desktop state.");
            }

        }

        [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")]
        private static extern IntPtr GetWindowLongPtr (IntPtr window, int index);

        [DllImport("user32.dll")]
        private static extern IntPtr GetWindow (IntPtr window, uint command);

        [DllImport("dwmapi.dll")]
        private static extern int DwmFlush ();

        [DllImport("user32.dll")]
        private static extern IntPtr SetThreadDpiAwarenessContext (IntPtr context);

    }

}
