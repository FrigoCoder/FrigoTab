using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Forms;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [STATestClass]
    [DoNotParallelize]
    [TestCategory("Acceptance")]
    public class T20260914T212400Z_003_RealProcessAcceptanceTests {

        [TestInitialize]
        public void UsePerMonitorPhysicalCoordinates () {
            SetThreadDpiAwarenessContext((IntPtr) (-4));
        }

        [TestMethod]
        public void ExecutableStartsResidentWithoutShowingAWindow () {
            using( RunningFrigoTab application = RunningFrigoTab.Start() ) {
                Assert.IsFalse(application.Process.HasExited);
                Assert.IsTrue(application.Windows.Count > 0, "The executable did not create its native message window.");
                Assert.AreEqual(0, application.Windows.Count(IsWindowVisible));
            }
        }

        [TestMethod]
        public void ASecondExecutableInstanceExitsWhileTheFirstRemainsResident () {
            using( RunningFrigoTab first = RunningFrigoTab.Start() ) {
                using( Process second = Process.Start(new ProcessStartInfo(RunningFrigoTab.ExecutablePath) {
                    UseShellExecute = false
                }) ) {
                    Assert.IsNotNull(second);
                    Assert.IsTrue(second.WaitForExit(5000), "The competing executable did not exit.");
                    Assert.AreEqual(0, second.ExitCode);
                    Assert.IsFalse(first.Process.HasExited);
                }
            }
        }

        [TestMethod]
        public void ClosingTheRealSessionWindowStopsTheTrayProcess () {
            using( RunningFrigoTab application = RunningFrigoTab.Start() ) {
                PostMessage(application.SessionWindow, 0x0010, IntPtr.Zero, IntPtr.Zero);

                Assert.IsTrue(application.Process.WaitForExit(5000), "Closing the native session window did not stop FrigoTab.");
                Assert.AreEqual(0, application.Process.ExitCode);
            }
        }

        [TestMethod]
        public void ExecutableOpensARealFullDesktopSession () {
            using( Form fixture = ShowFixture(Color.Lime) )
            using( RunningFrigoTab application = RunningFrigoTab.Start() ) {
                IntPtr owner = application.OpenSession();
                Rectangle bounds = GetBounds(owner);

                Assert.AreEqual(SystemInformation.VirtualScreen, bounds);
                Point preview = WaitForScreenPixel(
                    bounds,
                    IsLime,
                    "The executable did not render the real fixture preview.");
                Assert.IsTrue(bounds.Contains(preview));
            }
        }

        [TestMethod]
        public void PointerClickOnARealPreviewClosesTheExecutableSession () {
            using( Form fixture = ShowFixture(Color.Lime) )
            using( RunningFrigoTab application = RunningFrigoTab.Start() ) {
                IntPtr owner = application.OpenSession();
                Point point = WaitForScreenPixel(
                    GetBounds(owner),
                    IsLime,
                    "The real fixture preview did not become visible for pointer selection.");
                ScreenToClient(owner, ref point);
                IntPtr packedPoint = (IntPtr) ((point.Y << 16) | (point.X & 0xffff));

                PostMessage(owner, 0x0200, IntPtr.Zero, packedPoint);
                PostMessage(owner, 0x0201, (IntPtr) 1, packedPoint);

                WaitUntil(() => application.VisibleWindows.Count == 0, "The real pointer click did not close the session.");
                Assert.IsFalse(application.Process.HasExited);
            }
        }

        [TestMethod]
        public void FirstVisibleFrameUsesTheDesktopInsteadOfTheCoveringApplication () {
            Rectangle desktop = SystemInformation.VirtualScreen;
            Rectangle primary = Screen.PrimaryScreen.Bounds;
            using( Bitmap expectedDesktop = new Bitmap(desktop.Width, desktop.Height) )
            using( ShellDesktopSnapshot snapshot = new ShellDesktopSnapshot(desktop) ) {
                Assert.IsTrue(snapshot.IsAvailable, "Explorer did not expose a desktop surface for this acceptance test.");
                using( Graphics graphics = Graphics.FromImage(expectedDesktop) ) {
                    snapshot.Draw(graphics, new Rectangle(Point.Empty, expectedDesktop.Size));
                }

                using( Form fixture = ShowFixture(Color.Magenta, primary) ) {
                    fixture.TopMost = true;
                    fixture.Activate();
                    fixture.BringToFront();
                    Application.DoEvents();
                    DwmFlush();
                    Point fixtureCenter = new Point(primary.Left + primary.Width / 2, primary.Top + primary.Height / 2);
                    Assert.IsTrue(ColorDistance(Color.Magenta, ReadScreenPixel(fixtureCenter)) <= 10,
                        "The sentinel application did not cover the primary monitor.");

                    using( RunningFrigoTab application = RunningFrigoTab.Start() ) {
                        fixture.TopMost = false;
                        IntPtr owner = application.OpenSession();
                        Point sample = FindDesktopSample(primary, expectedDesktop, desktop);
                        Color desktopPixel = expectedDesktop.GetPixel(sample.X - desktop.Left, sample.Y - desktop.Top);
                        DwmFlush();
                        Color firstFrame = ReadScreenPixel(sample);
                        Thread.Sleep(300);
                        DwmFlush();
                        Color settledFrame = ReadScreenPixel(sample);

                        Assert.IsTrue(ColorDistance(desktopPixel, firstFrame) <= 45,
                            $"The first frame copied application pixels. Desktop={desktopPixel}, first={firstFrame}.");
                        Assert.IsTrue(ColorDistance(firstFrame, settledFrame) <= 20,
                            $"The background changed after opening. First={firstFrame}, settled={settledFrame}.");
                    }
                }
            }
        }

        private static Point FindDesktopSample (Rectangle monitor, Bitmap desktopImage, Rectangle desktop) {
            const int inset = 2;
            Point[] candidates = {
                new Point(monitor.Left + inset, monitor.Top + inset),
                new Point(monitor.Right - inset - 1, monitor.Top + inset),
                new Point(monitor.Left + inset, monitor.Bottom - inset - 1),
                new Point(monitor.Right - inset - 1, monitor.Bottom - inset - 1)
            };
            foreach( Point candidate in candidates ) {
                Color expected = desktopImage.GetPixel(candidate.X - desktop.Left, candidate.Y - desktop.Top);
                if( !IsMagenta(expected) ) {
                    return candidate;
                }
            }
            Assert.Fail("No stable shell-desktop sample was available outside the preview margin.");
            return Point.Empty;
        }

        private static Form ShowFixture (Color color, Rectangle? bounds = null) {
            Form fixture = new Form {
                Text = "FrigoTab process acceptance fixture",
                BackColor = color,
                Bounds = bounds ?? new Rectangle(80, 80, 640, 480),
                FormBorderStyle = FormBorderStyle.None,
                StartPosition = FormStartPosition.Manual,
                ShowInTaskbar = true
            };
            fixture.Show();
            fixture.Refresh();
            Application.DoEvents();
            return fixture;
        }

        private static Rectangle GetBounds (IntPtr window) {
            NativeRect bounds;
            Assert.IsTrue(GetWindowRect(window, out bounds));
            return Rectangle.FromLTRB(bounds.Left, bounds.Top, bounds.Right, bounds.Bottom);
        }

        private static long Area (Rectangle bounds) => (long) bounds.Width * bounds.Height;

        private static Color ReadScreenPixel (Point point) {
            using( Bitmap bitmap = new Bitmap(1, 1) ) {
                using( Graphics graphics = Graphics.FromImage(bitmap) ) {
                    graphics.CopyFromScreen(point, Point.Empty, new Size(1, 1));
                }
                return bitmap.GetPixel(0, 0);
            }
        }

        private static int ColorDistance (Color left, Color right) =>
            Math.Abs(left.R - right.R) + Math.Abs(left.G - right.G) + Math.Abs(left.B - right.B);

        private static bool IsLime (Color color) =>
            color.G > 200 && color.R < 80 && color.B < 80;

        private static bool IsMagenta (Color color) =>
            color.R > 200 && color.B > 200 && color.G < 80;

        private static Point WaitForScreenPixel (
            Rectangle bounds,
            Func<Color, bool> predicate,
            string failure) {
            Stopwatch timeout = Stopwatch.StartNew();
            while( timeout.Elapsed < TimeSpan.FromSeconds(5) ) {
                Point? result = FindScreenPixel(bounds, predicate);
                if( result.HasValue ) {
                    return result.Value;
                }
                Application.DoEvents();
                Thread.Sleep(50);
            }
            Assert.Fail(failure);
            return Point.Empty;
        }

        private static Point? FindScreenPixel (Rectangle bounds, Func<Color, bool> predicate) {
            using( Bitmap bitmap = new Bitmap(bounds.Width, bounds.Height, System.Drawing.Imaging.PixelFormat.Format32bppArgb) )
            using( Graphics graphics = Graphics.FromImage(bitmap) ) {
                graphics.CopyFromScreen(bounds.Location, Point.Empty, bounds.Size);
                System.Drawing.Imaging.BitmapData data = bitmap.LockBits(
                    new Rectangle(Point.Empty, bitmap.Size),
                    System.Drawing.Imaging.ImageLockMode.ReadOnly,
                    System.Drawing.Imaging.PixelFormat.Format32bppArgb);
                try {
                    int byteCount = Math.Abs(data.Stride) * data.Height;
                    byte[] pixels = new byte[byteCount];
                    Marshal.Copy(data.Scan0, pixels, 0, byteCount);
                    for( int y = 0; y < data.Height; y += 4 ) {
                        int row = data.Stride >= 0
                            ? y * data.Stride
                            : (data.Height - y - 1) * -data.Stride;
                        for( int x = 0; x < data.Width; x += 4 ) {
                            int offset = row + x * 4;
                            Color color = Color.FromArgb(
                                pixels[offset + 2],
                                pixels[offset + 1],
                                pixels[offset]);
                            if( predicate(color) ) {
                                return new Point(bounds.Left + x, bounds.Top + y);
                            }
                        }
                    }
                }
                finally {
                    bitmap.UnlockBits(data);
                }
            }
            return null;
        }

        private static void WaitUntil (Func<bool> condition, string failure) {
            Stopwatch timeout = Stopwatch.StartNew();
            while( timeout.Elapsed < TimeSpan.FromSeconds(5) ) {
                Application.DoEvents();
                if( condition() ) {
                    return;
                }
                Thread.Sleep(25);
            }
            Assert.Fail(failure);
        }

        private sealed class RunningFrigoTab : IDisposable {

            public static string ExecutablePath {
                get {
                    string configuredPath = Environment.GetEnvironmentVariable("FRIGOTAB_EXE");
                    string path = String.IsNullOrWhiteSpace(configuredPath)
                        ? Path.Combine(
                            Path.GetDirectoryName(typeof(SessionForm).Assembly.Location) ?? String.Empty,
                            "FrigoTab.exe")
                        : Path.GetFullPath(configuredPath);
                    Assert.IsTrue(File.Exists(path), "FrigoTab.exe was not found at: " + path);
                    return path;
                }
            }

            private readonly IntPtr sessionWindow;

            private RunningFrigoTab (Process process, IntPtr sessionWindow) {
                Process = process;
                this.sessionWindow = sessionWindow;
            }

            public Process Process { get; }

            public List<IntPtr> Windows => FindWindows(Process.Id);

            public List<IntPtr> VisibleWindows => Windows.Where(IsWindowVisible).ToList();

            public IntPtr SessionWindow => Process.HasExited || !IsWindow(sessionWindow)
                ? IntPtr.Zero
                : sessionWindow;

            public static RunningFrigoTab Start () {
                Process process = Process.Start(new ProcessStartInfo(ExecutablePath) {
                    UseShellExecute = false
                }) ?? throw new InvalidOperationException("Could not start FrigoTab.exe.");
                Stopwatch timeout = Stopwatch.StartNew();
                while( !process.HasExited && timeout.Elapsed < TimeSpan.FromSeconds(5) ) {
                    IntPtr sessionWindow = FindSessionWindow(process.Id);
                    if( sessionWindow != IntPtr.Zero ) {
                        return new RunningFrigoTab(process, sessionWindow);
                    }
                    Thread.Sleep(25);
                }
                int? exitCode = process.HasExited ? process.ExitCode : null;
                if( !process.HasExited ) {
                    process.Kill(true);
                    process.WaitForExit();
                }
                process.Dispose();
                Assert.Fail("FrigoTab did not become resident. Exit code: " + (exitCode?.ToString() ?? "still running"));
                throw new InvalidOperationException();
            }

            public IntPtr OpenSession () {
                PostMessage(SessionWindow, 0x4001, IntPtr.Zero, IntPtr.Zero);
                WaitUntil(() => SessionWindow != IntPtr.Zero && IsWindowVisible(SessionWindow),
                    "The executable session did not become visible.");
                return SessionWindow;
            }

            public void Dispose () {
                if( !Process.HasExited ) {
                    IntPtr session = SessionWindow;
                    if( session != IntPtr.Zero ) {
                        PostMessage(session, 0x0010, IntPtr.Zero, IntPtr.Zero);
                    }
                    if( !Process.WaitForExit(2000) ) {
                        Process.Kill(true);
                        Process.WaitForExit();
                    }
                }
                Process.Dispose();
            }

            private static List<IntPtr> FindWindows (int expectedProcessId) {
                List<IntPtr> windows = new List<IntPtr>();
                EnumWindows((window, parameter) => {
                    GetWindowThreadProcessId(window, out uint processId);
                    if( processId == expectedProcessId ) {
                        windows.Add(window);
                    }
                    return true;
                }, IntPtr.Zero);
                return windows;
            }

            private static IntPtr FindSessionWindow (int expectedProcessId) {
                foreach( IntPtr window in FindWindows(expectedProcessId) ) {
                    long extendedStyle = GetWindowLongPtr(window, -20).ToInt64();
                    NativeRect bounds;
                    if( (extendedStyle & 0x80) != 0 &&
                        GetWindowRect(window, out bounds) &&
                        bounds.Right - bounds.Left >= 100 &&
                        bounds.Bottom - bounds.Top >= 100 ) {
                        return window;
                    }
                }
                return IntPtr.Zero;
            }

        }

        private delegate bool EnumWindowsCallback (IntPtr window, IntPtr parameter);

        [StructLayout(LayoutKind.Sequential)]
        private struct NativeRect {
            public int Left;
            public int Top;
            public int Right;
            public int Bottom;
        }

        [DllImport("user32.dll")]
        private static extern bool EnumWindows (EnumWindowsCallback callback, IntPtr parameter);

        [DllImport("user32.dll")]
        private static extern uint GetWindowThreadProcessId (IntPtr window, out uint processId);

        [DllImport("user32.dll")]
        private static extern bool IsWindowVisible (IntPtr window);

        [DllImport("user32.dll")]
        private static extern bool IsWindow (IntPtr window);

        [DllImport("user32.dll")]
        private static extern bool GetWindowRect (IntPtr window, out NativeRect bounds);

        [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")]
        private static extern IntPtr GetWindowLongPtr (IntPtr window, int index);

        [DllImport("user32.dll")]
        private static extern bool PostMessage (IntPtr window, uint message, IntPtr wParam, IntPtr lParam);

        [DllImport("user32.dll")]
        private static extern bool ScreenToClient (IntPtr window, ref Point point);

        [DllImport("dwmapi.dll")]
        private static extern int DwmFlush ();

        [DllImport("user32.dll")]
        private static extern IntPtr SetThreadDpiAwarenessContext (IntPtr dpiContext);

    }

}
