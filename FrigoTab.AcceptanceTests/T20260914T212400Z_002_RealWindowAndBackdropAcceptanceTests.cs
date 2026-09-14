using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Drawing;
using System.Drawing.Imaging;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Forms;
using FrigoTab;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [STATestClass]
    [DoNotParallelize]
    [TestCategory("Acceptance")]
    public sealed class T20260914T212400Z_002_RealWindowAndBackdropAcceptanceTests {

        [STATestMethod]
        public void WindowFinderIncludesARealNormalWindowAndExcludesUnsupportedWindows () {
            string suffix = Guid.NewGuid().ToString("N");
            using( AcceptanceWindow normal = CreateWindow("FrigoTab.Acceptance.Normal." + suffix, Color.CornflowerBlue) )
            using( AcceptanceWindow hidden = CreateWindow("FrigoTab.Acceptance.Hidden." + suffix, Color.Gray) )
            using( AcceptanceWindow tool = CreateWindow("FrigoTab.Acceptance.Tool." + suffix, Color.Gray, toolWindow: true) )
            using( AcceptanceWindow noActivate = CreateWindow(
                "FrigoTab.Acceptance.NoActivate." + suffix,
                Color.Gray,
                noActivate: true) ) {
                normal.Show();
                hidden.Show();
                tool.Show();
                noActivate.Show();
                Pump();
                hidden.Hide();
                Pump();

                WindowHandle normalHandle = new WindowHandle(normal.Handle);
                WindowHandle hiddenHandle = new WindowHandle(hidden.Handle);
                WindowHandle toolHandle = new WindowHandle(tool.Handle);
                WindowHandle noActivateHandle = new WindowHandle(noActivate.Handle);
                WindowFinder finder = new WindowFinder();

                Assert.IsTrue(Contains(finder, normalHandle), "The visible normal Form was not enumerated.");
                Assert.IsFalse(Contains(finder, hiddenHandle), "A hidden Form was enumerated as an application.");
                Assert.IsFalse(Contains(finder, toolHandle), "A tool window was enumerated as an application.");
                Assert.IsFalse(Contains(finder, noActivateHandle), "A no-activate window was enumerated as an application.");
            }
        }

        [STATestMethod]
        public void WindowHandlePreservesAUnicodeTitleFromARealWindow () {
            const string title = "FrigoTab.Acceptance.Ω窗";
            using( AcceptanceWindow window = CreateWindow(title, Color.CornflowerBlue) ) {
                window.Show();
                Pump();

                string actual = new WindowHandle(window.Handle).GetWindowText();

                Assert.AreEqual(title, actual);
            }
        }

        [STATestMethod]
        public void LayoutKeepsALiveWindowAndSkipsAClosedWindowHandle () {
            using( AcceptanceWindow live = CreateWindow(
                "FrigoTab.Acceptance.Layout.Live." + Guid.NewGuid().ToString("N"),
                Color.CornflowerBlue) )
            using( AcceptanceWindow closed = CreateWindow(
                "FrigoTab.Acceptance.Layout.Closed." + Guid.NewGuid().ToString("N"),
                Color.Gray) ) {
                live.Show();
                closed.Show();
                Pump();
                WindowHandle liveHandle = new WindowHandle(live.Handle);
                WindowHandle closedHandle = new WindowHandle(closed.Handle);

                closed.Close();
                closed.Dispose();
                Pump();

                Layout layout = new Layout(new List<WindowHandle> {liveHandle, closedHandle});

                Rectangle tile;
                Assert.IsTrue(layout.Bounds.TryGetValue(liveHandle, out tile));
                Assert.IsTrue(tile.Width > 0 && tile.Height > 0);
                Assert.IsFalse(layout.Bounds.ContainsKey(closedHandle));
            }
        }

        [STATestMethod]
        public void MinimizedWindowUsesItsRestoredMonitorWhenMultipleScreensExist () {
            Screen target = Screen.AllScreens.FirstOrDefault(screen => !screen.Primary);
            if( target == null ) {
                Assert.Inconclusive("The restored-monitor acceptance case requires at least two monitors.");
            }

            Rectangle bounds = Rectangle.Inflate(target.WorkingArea, -40, -40);
            using( AcceptanceWindow window = CreateWindow(
                "FrigoTab.Acceptance.Layout.Minimized." + Guid.NewGuid().ToString("N"),
                Color.CornflowerBlue,
                bounds: bounds) ) {
                window.Show();
                Pump();
                window.WindowState = FormWindowState.Minimized;
                Pump();

                WindowHandle handle = new WindowHandle(window.Handle);
                Rectangle restored;
                Assert.IsTrue(handle.TryGetRect(out restored), "The real minimized window has no restored placement.");
                Assert.AreEqual(target.DeviceName, Screen.FromRectangle(restored).DeviceName);

                Layout layout = new Layout(new List<WindowHandle> {handle});
                Rectangle tile;
                Assert.IsTrue(layout.Bounds.TryGetValue(handle, out tile));
                Assert.IsTrue(target.WorkingArea.Contains(tile), "The tile was assigned outside the restored monitor.");
            }
        }

        [STATestMethod]
        public void ShellSnapshotDrawsDesktopPixelsWithoutCopyingAVisibleApplication () {
            Rectangle desktop = GetVirtualDesktopBounds();
            if( desktop.Width < 160 || desktop.Height < 160 ) {
                Assert.Inconclusive("The desktop is too small for a stable shell-snapshot pixel check.");
            }

            Rectangle coveredArea = new Rectangle(desktop.Left + 32, desktop.Top + 32, 120, 120);
            using( AcceptanceWindow covered = CreateWindow(
                "FrigoTab.Acceptance.ShellSurface." + Guid.NewGuid().ToString("N"),
                Color.Magenta,
                bounds: coveredArea,
                topMost: true) ) {
                covered.Show();
                covered.BringToFront();
                Pump();

                using( ShellDesktopSnapshot snapshot = new ShellDesktopSnapshot(desktop) ) {
                    if( !snapshot.IsAvailable ) {
                        Assert.Inconclusive("Explorer did not expose a capturable desktop surface.");
                    }

                    using( Bitmap bitmap = new Bitmap(
                        desktop.Width,
                        desktop.Height,
                        PixelFormat.Format32bppArgb) )
                    using( Graphics graphics = Graphics.FromImage(bitmap) ) {
                        graphics.Clear(Color.Black);
                        snapshot.Draw(graphics, new Rectangle(0, 0, desktop.Width, desktop.Height));

                        int sampleX = coveredArea.Left - desktop.Left + coveredArea.Width / 2;
                        int sampleY = coveredArea.Top - desktop.Top + coveredArea.Height / 2;
                        Color sample = bitmap.GetPixel(sampleX, sampleY);
                        Assert.IsFalse(
                            IsMagenta(sample),
                            "The shell snapshot copied the visible application's solid-magenta surface.");

                        if( IsUniformBlack(bitmap) ) {
                            Assert.Inconclusive("The desktop surface rendered uniformly black on this desktop.");
                        }
                    }
                }
            }
        }

        [STATestMethod]
        public void DwmThumbnailAppearsOnlyDuringItsVisiblePhase () {
            using( AcceptanceWindow source = CreateWindow(
                "FrigoTab.Acceptance.Thumbnail.Source." + Guid.NewGuid().ToString("N"),
                Color.Magenta,
                bounds: new Rectangle(120, 120, 180, 120)) )
            using( AcceptanceWindow destination = CreateWindow(
                "FrigoTab.Acceptance.Thumbnail.Destination." + Guid.NewGuid().ToString("N"),
                Color.Black,
                bounds: new Rectangle(360, 120, 240, 160),
                topMost: true) ) {
                source.Show();
                destination.Show();
                destination.BringToFront();
                Pump();

                try {
                    using( Thumbnail thumbnail = new Thumbnail(
                        new WindowHandle(source.Handle),
                        new WindowHandle(destination.Handle)) ) {
                        thumbnail.SetDestinationRect(new Rect(new Rectangle(
                            0,
                            0,
                            destination.ClientSize.Width,
                            destination.ClientSize.Height)));
                        thumbnail.SetVisible(false);
                        RequirePixel(destination, IsBlack, "DWM did not keep the thumbnail hidden.");

                        thumbnail.SetVisible(true);
                        RequirePixel(destination, IsMagenta, "DWM did not show the real source window.");

                        thumbnail.SetVisible(false);
                        RequirePixel(destination, IsBlack, "DWM did not hide the real source window again.");
                    }
                }
                catch( Exception exception ) when( IsDwmUnavailable(exception) ) {
                    Assert.Inconclusive("DWM thumbnails are unavailable on this desktop: " + exception.Message);
                }
            }
        }

        [STATestMethod]
        public void RepeatedDwmThumbnailDisposalLeavesTheDestinationClear () {
            using( AcceptanceWindow source = CreateWindow(
                "FrigoTab.Acceptance.Thumbnail.Repeat.Source." + Guid.NewGuid().ToString("N"),
                Color.Magenta,
                bounds: new Rectangle(120, 320, 180, 120)) )
            using( AcceptanceWindow destination = CreateWindow(
                "FrigoTab.Acceptance.Thumbnail.Repeat.Destination." + Guid.NewGuid().ToString("N"),
                Color.Black,
                bounds: new Rectangle(360, 320, 240, 160),
                topMost: true) ) {
                source.Show();
                destination.Show();
                destination.BringToFront();
                Pump();

                try {
                    for( int iteration = 0; iteration < 5; iteration++ ) {
                        using( Thumbnail thumbnail = new Thumbnail(
                            new WindowHandle(source.Handle),
                            new WindowHandle(destination.Handle)) ) {
                            thumbnail.SetDestinationRect(new Rect(new Rectangle(
                                0,
                                0,
                                destination.ClientSize.Width,
                                destination.ClientSize.Height)));
                            thumbnail.SetVisible(true);
                            RequirePixel(destination, IsMagenta, "The real thumbnail did not become visible.");
                            thumbnail.SetVisible(false);
                        }

                        RequirePixel(destination, IsBlack, "A disposed thumbnail remained visible.");
                    }
                }
                catch( Exception exception ) when( IsDwmUnavailable(exception) ) {
                    Assert.Inconclusive("DWM thumbnails are unavailable on this desktop: " + exception.Message);
                }
            }
        }

        private static AcceptanceWindow CreateWindow (
            string title,
            Color color,
            bool toolWindow = false,
            bool noActivate = false,
            Rectangle? bounds = null,
            bool topMost = false) {
            return new AcceptanceWindow(title, color, toolWindow, noActivate, bounds, topMost);
        }

        private static bool Contains (WindowFinder finder, WindowHandle expected) {
            return finder.Windows.Any(window => window == expected);
        }

        private static Rectangle GetVirtualDesktopBounds () {
            Rectangle result = Rectangle.Empty;
            bool found = false;
            foreach( Screen screen in Screen.AllScreens ) {
                result = found ? Rectangle.Union(result, screen.Bounds) : screen.Bounds;
                found = true;
            }
            return result;
        }

        private static void Pump () {
            Application.DoEvents();
        }

        private static bool IsMagenta (Color color) {
            return color.R > 200 && color.B > 200 && color.G < 100;
        }

        private static bool IsBlack (Color color) {
            return color.R < 20 && color.G < 20 && color.B < 20;
        }

        private static bool IsUniformBlack (Bitmap bitmap) {
            int stepX = Math.Max(1, bitmap.Width / 16);
            int stepY = Math.Max(1, bitmap.Height / 16);
            for( int y = 0; y < bitmap.Height; y += stepY ) {
                for( int x = 0; x < bitmap.Width; x += stepX ) {
                    if( !IsBlack(bitmap.GetPixel(x, y)) ) {
                        return false;
                    }
                }
            }
            return true;
        }

        private static void RequirePixel (
            Form window,
            Func<Color, bool> predicate,
            string failureMessage) {
            Color last = Color.Empty;
            bool matched = false;
            for( int attempt = 0; attempt < 80; attempt++ ) {
                Pump();
                try {
                    last = ReadClientPixel(window);
                }
                catch( Exception exception ) when( exception is ExternalException ||
                    exception is Win32Exception ||
                    exception is ArgumentException ) {
                    Assert.Inconclusive("The desktop pixel could not be sampled: " + exception.Message);
                }

                if( predicate(last) ) {
                    matched = true;
                    break;
                }
                Thread.Sleep(25);
            }

            if( !matched ) {
                Assert.Fail(failureMessage + " Last pixel: " + last);
            }
        }

        private static Color ReadClientPixel (Form window) {
            Point center = window.PointToScreen(new Point(
                window.ClientSize.Width / 2,
                window.ClientSize.Height / 2));
            using( Bitmap pixel = new Bitmap(1, 1, PixelFormat.Format32bppArgb) )
            using( Graphics graphics = Graphics.FromImage(pixel) ) {
                graphics.CopyFromScreen(center, Point.Empty, pixel.Size);
                return pixel.GetPixel(0, 0);
            }
        }

        private static bool IsDwmUnavailable (Exception exception) {
            return exception is ExternalException ||
                exception is DllNotFoundException ||
                exception is EntryPointNotFoundException;
        }

        private sealed class AcceptanceWindow : Form {

            private const int ToolWindow = 0x00000080;
            private const int NoActivate = 0x08000000;
            private readonly bool toolWindow;
            private readonly bool noActivate;

            public AcceptanceWindow (
                string title,
                Color color,
                bool toolWindow,
                bool noActivate,
                Rectangle? bounds,
                bool topMost) {
                Text = title;
                BackColor = color;
                FormBorderStyle = FormBorderStyle.None;
                StartPosition = FormStartPosition.Manual;
                ShowInTaskbar = !toolWindow && !noActivate;
                TopMost = topMost;
                ClientSize = new Size(240, 160);
                Bounds = bounds ?? new Rectangle(120, 120, 240, 160);
                this.toolWindow = toolWindow;
                this.noActivate = noActivate;
            }

            protected override CreateParams CreateParams {
                get {
                    CreateParams result = base.CreateParams;
                    if( toolWindow ) {
                        result.ExStyle |= ToolWindow;
                    }
                    if( noActivate ) {
                        result.ExStyle |= NoActivate;
                    }
                    return result;
                }
            }

        }

    }

}
