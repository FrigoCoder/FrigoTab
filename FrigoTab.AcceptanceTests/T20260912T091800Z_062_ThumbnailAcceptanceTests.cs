using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Runtime.InteropServices;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("NativeContract")]
    public sealed class T20260912T091800Z_062_ThumbnailAcceptanceTests {

        [TestMethod]
        public void ThumbnailUpdateRequestsVisibleOpaqueDestination () {
            FakeDwmThumbnailApi api = new FakeDwmThumbnailApi();

            using( Thumbnail thumbnail = new Thumbnail(
                new WindowHandle(new IntPtr(1)),
                new WindowHandle(new IntPtr(2)),
                api) ) {
                thumbnail.SetDestinationRect(new Rect(new Rectangle(10, 20, 200, 100)));

                DwmThumbnailProperties update = api.Updates[0];
                Assert.IsTrue(update.Flags.HasFlag(DwmThumbnailFlags.RectDestination));
                Assert.IsTrue(update.Flags.HasFlag(DwmThumbnailFlags.Visible));
                Assert.IsTrue(update.Flags.HasFlag(DwmThumbnailFlags.Opacity));
                Assert.IsTrue(update.Visible);
                Assert.AreEqual(byte.MaxValue, update.Opacity);
            }

            Assert.AreEqual(1, api.UnregisterCalls);
        }

        [TestMethod]
        public void FailedThumbnailRegistrationReleasesReturnedHandle () {
            FakeDwmThumbnailApi api = new FakeDwmThumbnailApi {
                RegisterResult = unchecked((int) 0x80004005)
            };

            AssertThrowsExternalException(() => new Thumbnail(
                new WindowHandle(new IntPtr(1)),
                new WindowHandle(new IntPtr(2)),
                api));

            Assert.AreEqual(1, api.UnregisterCalls);
        }

        [TestMethod]
        public void FailedThumbnailUpdateIsSurfacedAndDisposable () {
            FakeDwmThumbnailApi api = new FakeDwmThumbnailApi {
                UpdateResult = unchecked((int) 0x80004005)
            };
            Thumbnail thumbnail = new Thumbnail(
                new WindowHandle(new IntPtr(1)),
                new WindowHandle(new IntPtr(2)),
                api);

            AssertThrowsExternalException(() => thumbnail.SetDestinationRect(new Rect(new Rectangle(0, 0, 100, 100))));
            thumbnail.Dispose();

            Assert.AreEqual(1, api.UnregisterCalls);
        }

        [TestMethod]
        public void ThumbnailSourceUpdateIsVisibleAndOpaque () {
            FakeDwmThumbnailApi api = new FakeDwmThumbnailApi();

            using( Thumbnail thumbnail = new Thumbnail(
                new WindowHandle(new IntPtr(1)),
                new WindowHandle(new IntPtr(2)),
                api) ) {
                thumbnail.SetSourceRect(new Rect(new Rectangle(5, 6, 70, 80)));

                DwmThumbnailProperties update = api.Updates[0];
                Assert.IsTrue(update.Flags.HasFlag(DwmThumbnailFlags.RectSource));
                Assert.IsTrue(update.Flags.HasFlag(DwmThumbnailFlags.Visible));
                Assert.IsTrue(update.Flags.HasFlag(DwmThumbnailFlags.Opacity));
                Assert.IsTrue(update.Visible);
                Assert.AreEqual(byte.MaxValue, update.Opacity);
            }
        }

        [TestMethod]
        public void FailedThumbnailUnregisterIsDiagnosedWithoutEscapingDispose () {
            FakeDwmThumbnailApi api = new FakeDwmThumbnailApi {
                UnregisterResult = unchecked((int) 0x80004005)
            };
            using( StringWriter diagnostic = new StringWriter() ) {
                TextWriterTraceListener listener = new TextWriterTraceListener(diagnostic);
                Trace.Listeners.Add(listener);
                try {
                    Thumbnail thumbnail = new Thumbnail(
                        new WindowHandle(new IntPtr(1)),
                        new WindowHandle(new IntPtr(2)),
                        api);

                    thumbnail.Dispose();
                    Trace.Flush();

                    Assert.AreEqual(1, api.UnregisterCalls);
                    StringAssert.Contains(diagnostic.ToString(), "DwmUnregisterThumbnail failed");
                }
                finally {
                    Trace.Listeners.Remove(listener);
                    listener.Dispose();
                }
            }
        }

        private static void AssertThrowsExternalException (Action action) {
            try {
                action();
                Assert.Fail("Expected an ExternalException-compatible HRESULT failure.");
            }
            catch( ExternalException ) {
            }
        }

        private sealed class FakeDwmThumbnailApi : IDwmThumbnailApi {

            public readonly List<DwmThumbnailProperties> Updates = new List<DwmThumbnailProperties>();
            public int RegisterResult;
            public int UpdateResult;
            public int UnregisterResult;
            public int UnregisterCalls;

            public int Register (WindowHandle destination, WindowHandle source, out IntPtr thumbnail) {
                thumbnail = new IntPtr(1234);
                return RegisterResult;
            }

            public int Unregister (IntPtr thumbnail) {
                UnregisterCalls++;
                return UnregisterResult;
            }

            public int Update (IntPtr thumbnail, ref DwmThumbnailProperties properties) {
                Updates.Add(properties);
                return UpdateResult;
            }

        }

    }

}
