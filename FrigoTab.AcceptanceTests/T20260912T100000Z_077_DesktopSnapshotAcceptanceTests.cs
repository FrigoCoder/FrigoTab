using System;
using System.Collections.Generic;
using System.Drawing;
using FrigoTab;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("Regression")]
    public sealed class T20260912T100000Z_077_DesktopSnapshotAcceptanceTests {

        [TestMethod]
        public void CapturesTheExactVirtualDesktopBoundsOnce () {
            Rectangle virtualDesktop = new Rectangle(-1920, 0, 4480, 1440);
            FakeSnapshotFrame frame = new FakeSnapshotFrame(virtualDesktop.Size);
            FakeSnapshotApi api = new FakeSnapshotApi(frame);

            using( DesktopSnapshot snapshot = new DesktopSnapshot(virtualDesktop, api) ) {
                Assert.AreEqual(1, api.CaptureCalls);
                Assert.AreEqual(virtualDesktop, api.CapturedBounds);
            }
        }

        [TestMethod]
        public void RepeatedPaintsReuseTheCapturedFrameWithoutRecapturing () {
            Rectangle virtualDesktop = new Rectangle(-1920, 0, 4480, 1440);
            FakeSnapshotFrame frame = new FakeSnapshotFrame(virtualDesktop.Size);
            FakeSnapshotApi api = new FakeSnapshotApi(frame);

            using( DesktopSnapshot snapshot = new DesktopSnapshot(virtualDesktop, api) )
            using( Bitmap canvas = new Bitmap(virtualDesktop.Width, virtualDesktop.Height) )
            using( Graphics graphics = Graphics.FromImage(canvas) ) {
                Rectangle clientBounds = new Rectangle(0, 0, virtualDesktop.Width, virtualDesktop.Height);

                snapshot.Draw(graphics, clientBounds);
                snapshot.Draw(graphics, clientBounds);

                Assert.AreEqual(1, api.CaptureCalls);
                Assert.AreEqual(2, frame.DrawCalls);
                CollectionAssert.AreEqual(
                    new[] {clientBounds, clientBounds},
                    frame.Destinations);
            }
        }

        [TestMethod]
        public void CaptureFailureLeavesAnOpaqueBlackFallback () {
            Rectangle virtualDesktop = new Rectangle(-1920, 0, 4480, 1440);
            FakeSnapshotApi api = new FakeSnapshotApi {
                Failure = new InvalidOperationException("desktop capture unavailable")
            };

            using( DesktopSnapshot snapshot = new DesktopSnapshot(virtualDesktop, api) )
            using( Bitmap canvas = new Bitmap(1, 1) )
            using( Graphics graphics = Graphics.FromImage(canvas) ) {
                graphics.Clear(Color.White);
                snapshot.Draw(graphics, new Rectangle(0, 0, 1, 1));

                Assert.AreEqual(1, api.CaptureCalls);
                Assert.AreEqual(Color.Black.ToArgb(), canvas.GetPixel(0, 0).ToArgb());
            }
        }

        [TestMethod]
        public void MissingFrameLeavesAnOpaqueBlackFallback () {
            Rectangle virtualDesktop = new Rectangle(-1920, 0, 4480, 1440);
            FakeSnapshotApi api = new FakeSnapshotApi();

            using( DesktopSnapshot snapshot = new DesktopSnapshot(virtualDesktop, api) )
            using( Bitmap canvas = new Bitmap(1, 1) )
            using( Graphics graphics = Graphics.FromImage(canvas) ) {
                graphics.Clear(Color.White);
                snapshot.Draw(graphics, new Rectangle(0, 0, 1, 1));

                Assert.AreEqual(1, api.CaptureCalls);
                Assert.AreEqual(Color.Black.ToArgb(), canvas.GetPixel(0, 0).ToArgb());
            }
        }

        [TestMethod]
        public void ExactClientBoundsAreForwardedToTheCapturedFrame () {
            Rectangle virtualDesktop = new Rectangle(-1920, 0, 4480, 1440);
            FakeSnapshotFrame frame = new FakeSnapshotFrame(virtualDesktop.Size);
            FakeSnapshotApi api = new FakeSnapshotApi(frame);
            Rectangle clientBounds = new Rectangle(0, 0, virtualDesktop.Width, virtualDesktop.Height);

            using( DesktopSnapshot snapshot = new DesktopSnapshot(virtualDesktop, api) )
            using( Bitmap canvas = new Bitmap(virtualDesktop.Width, virtualDesktop.Height) )
            using( Graphics graphics = Graphics.FromImage(canvas) ) {
                snapshot.Draw(graphics, clientBounds);

                Assert.AreEqual(1, frame.DrawCalls);
                CollectionAssert.AreEqual(new[] {clientBounds}, frame.Destinations);
            }
        }

        [TestMethod]
        public void EmptyBoundsFailOpenWithoutInvokingNativeCapture () {
            FakeSnapshotApi api = new FakeSnapshotApi(new FakeSnapshotFrame(Size.Empty));

            using( DesktopSnapshot snapshot = new DesktopSnapshot(Rectangle.Empty, api) ) {
                Assert.AreEqual(0, api.CaptureCalls);
            }
        }

        [TestMethod]
        public void DisposeReleasesTheCapturedFrameOnceAndRejectsFurtherPainting () {
            Rectangle virtualDesktop = new Rectangle(-1920, 0, 4480, 1440);
            FakeSnapshotFrame frame = new FakeSnapshotFrame(virtualDesktop.Size);
            FakeSnapshotApi api = new FakeSnapshotApi(frame);
            DesktopSnapshot snapshot = new DesktopSnapshot(virtualDesktop, api);

            snapshot.Dispose();
            snapshot.Dispose();

            Assert.AreEqual(1, frame.DisposeCalls);
            using( Bitmap canvas = new Bitmap(1, 1) )
            using( Graphics graphics = Graphics.FromImage(canvas) ) {
                AssertThrowsObjectDisposedException(
                    () => snapshot.Draw(graphics, new Rectangle(0, 0, 1, 1)));
            }
        }

        private static void AssertThrowsObjectDisposedException (Action action) {
            try {
                action();
                Assert.Fail("Expected painting a disposed desktop snapshot to fail.");
            }
            catch( ObjectDisposedException ) {
            }
        }

        private sealed class FakeSnapshotApi : IDesktopSnapshotApi {

            private readonly IDesktopSnapshotFrame frame;

            public FakeSnapshotApi (IDesktopSnapshotFrame frame = null) {
                this.frame = frame;
            }

            public int CaptureCalls { get; private set; }
            public Rectangle CapturedBounds { get; private set; }
            public Exception Failure { get; set; }

            public IDesktopSnapshotFrame Capture (Rectangle sourceBounds) {
                CaptureCalls++;
                CapturedBounds = sourceBounds;
                if( Failure != null ) {
                    throw Failure;
                }
                return frame;
            }

        }

        private sealed class FakeSnapshotFrame : IDesktopSnapshotFrame {

            public FakeSnapshotFrame (Size size) {
                Size = size;
            }

            public Size Size { get; }
            public int DrawCalls { get; private set; }
            public int DisposeCalls { get; private set; }
            public List<Rectangle> Destinations { get; } = new List<Rectangle>();

            public void Draw (Graphics graphics, Rectangle destinationBounds) {
                DrawCalls++;
                Destinations.Add(destinationBounds);
            }

            public void Dispose () {
                DisposeCalls++;
            }

        }

    }

}
