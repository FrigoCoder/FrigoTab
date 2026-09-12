using System;
using System.Collections.Generic;
using System.Drawing;
using System.Runtime.InteropServices;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("NativeContract")]
    public sealed class T20260912T100000Z_077_DwmDesktopBackdropAcceptanceTests {

        [TestMethod]
        public void LiveGlassExtendsAcrossTheEntireNoRedirectionOwner () {
            FakeDwmGlassApi api = new FakeDwmGlassApi();

            using( DwmGlassBackdrop backdrop = new DwmGlassBackdrop(
                new WindowHandle(new IntPtr(23)), api) ) {
                Assert.IsTrue(backdrop.IsAvailable);
                Assert.AreEqual(1, api.Updates.Count);
                Assert.AreEqual(new WindowHandle(new IntPtr(23)), api.Destination);
                Assert.AreEqual(-1, api.Updates[0].Left);
                Assert.AreEqual(-1, api.Updates[0].Right);
                Assert.AreEqual(-1, api.Updates[0].Top);
                Assert.AreEqual(-1, api.Updates[0].Bottom);
            }

            Assert.AreEqual(2, api.Updates.Count);
            Assert.AreEqual(0, api.Updates[1].Left);
            Assert.AreEqual(0, api.Updates[1].Right);
            Assert.AreEqual(0, api.Updates[1].Top);
            Assert.AreEqual(0, api.Updates[1].Bottom);
        }

        [TestMethod]
        public void FailedGlassCanRecoverWhenTheCompositorBecomesAvailable () {
            FakeDwmGlassApi api = new FakeDwmGlassApi {
                Result = unchecked((int) 0x80004005)
            };
            using( DwmGlassBackdrop backdrop = new DwmGlassBackdrop(
                new WindowHandle(new IntPtr(23)), api) ) {
                Assert.IsFalse(backdrop.IsAvailable);
                api.Result = 0;

                Assert.IsTrue(backdrop.TryApply());
                Assert.IsTrue(backdrop.IsAvailable);
                Assert.AreEqual(2, api.Updates.Count);
            }
        }

        [TestMethod]
        public void MissingGlassDestinationUsesTheFallbackWithoutCallingDwm () {
            FakeDwmGlassApi api = new FakeDwmGlassApi();

            using( DwmGlassBackdrop backdrop = new DwmGlassBackdrop(WindowHandle.Null, api) ) {
                Assert.IsFalse(backdrop.IsAvailable);
            }
            Assert.AreEqual(0, api.Updates.Count);
        }

        [TestMethod]
        public void RegistersTheInjectedDesktopSourceAsOneOpaqueBackdrop () {
            FakeDwmThumbnailApi api = new FakeDwmThumbnailApi();
            FakeDesktopWindowSource source = new FakeDesktopWindowSource(new WindowHandle(new IntPtr(17)));
            Rectangle bounds = new Rectangle(-100, 20, 800, 600);

            using( DwmDesktopBackdrop backdrop = new DwmDesktopBackdrop(
                new WindowHandle(new IntPtr(23)), bounds, api, source) ) {
                Assert.IsTrue(backdrop.IsAvailable);
                Assert.AreEqual(1, source.FindCalls);
                Assert.AreEqual(1, api.RegisterCalls);
                Assert.AreEqual(new WindowHandle(new IntPtr(17)), api.Source);
                Assert.AreEqual(new WindowHandle(new IntPtr(23)), api.Destination);
                Assert.AreEqual(1, api.Updates.Count);

                DwmThumbnailProperties update = api.Updates[0];
                Assert.IsTrue(update.Flags.HasFlag(DwmThumbnailFlags.RectDestination));
                Assert.IsTrue(update.Flags.HasFlag(DwmThumbnailFlags.Visible));
                Assert.IsTrue(update.Flags.HasFlag(DwmThumbnailFlags.Opacity));
                Assert.IsTrue(update.Visible);
                Assert.AreEqual(byte.MaxValue, update.Opacity);
                Assert.AreEqual(bounds.Size, update.Destination.Size());
            }

            Assert.AreEqual(1, api.UnregisterCalls);
        }

        [TestMethod]
        public void MissingDesktopSourceUsesInstantSolidFallback () {
            FakeDwmThumbnailApi api = new FakeDwmThumbnailApi();
            FakeDesktopWindowSource source = new FakeDesktopWindowSource(WindowHandle.Null);
            using( DwmDesktopBackdrop backdrop = new DwmDesktopBackdrop(
                new WindowHandle(new IntPtr(23)), new Rectangle(0, 0, 100, 100), api, source) ) {
                Assert.IsFalse(backdrop.IsAvailable);
                Assert.AreEqual(0, api.RegisterCalls);

                using( Bitmap bitmap = new Bitmap(1, 1) ) {
                    using( Graphics graphics = Graphics.FromImage(bitmap) ) {
                        graphics.Clear(Color.White);
                        backdrop.DrawFallback(graphics);
                    }
                    Assert.AreEqual(Color.Black.ToArgb(), bitmap.GetPixel(0, 0).ToArgb());
                }
            }
        }

        [TestMethod]
        public void DwmRegistrationFailureFallsBackAndReleasesTheHandle () {
            FakeDwmThumbnailApi api = new FakeDwmThumbnailApi {
                RegisterResult = unchecked((int) 0x80004005)
            };
            FakeDesktopWindowSource source = new FakeDesktopWindowSource(new WindowHandle(new IntPtr(17)));

            using( DwmDesktopBackdrop backdrop = new DwmDesktopBackdrop(
                new WindowHandle(new IntPtr(23)), new Rectangle(0, 0, 100, 100), api, source) ) {
                Assert.IsFalse(backdrop.IsAvailable);
            }

            Assert.AreEqual(1, api.RegisterCalls);
            Assert.AreEqual(1, api.UnregisterCalls);
        }

        [TestMethod]
        public void DisposeUnregistersTheDesktopBackdropOnlyOnce () {
            FakeDwmThumbnailApi api = new FakeDwmThumbnailApi();
            FakeDesktopWindowSource source = new FakeDesktopWindowSource(new WindowHandle(new IntPtr(17)));
            DwmDesktopBackdrop backdrop = new DwmDesktopBackdrop(
                new WindowHandle(new IntPtr(23)), new Rectangle(0, 0, 100, 100), api, source);

            backdrop.Dispose();
            backdrop.Dispose();

            Assert.AreEqual(1, api.UnregisterCalls);
        }

        private sealed class FakeDesktopWindowSource : IDesktopWindowSource {

            private readonly WindowHandle result;

            public FakeDesktopWindowSource (WindowHandle result) => this.result = result;

            public int FindCalls { get; private set; }

            public WindowHandle Find () {
                FindCalls++;
                return result;
            }

        }

        private sealed class FakeDwmGlassApi : IDwmGlassApi {

            public int Result;
            public readonly IList<DwmMargins> Updates = new List<DwmMargins>();
            public WindowHandle Destination;

            public int ExtendFrame (WindowHandle destination, ref DwmMargins margins) {
                Destination = destination;
                Updates.Add(margins);
                return Result;
            }

        }

        private sealed class FakeDwmThumbnailApi : IDwmThumbnailApi {

            public readonly IList<DwmThumbnailProperties> Updates = new List<DwmThumbnailProperties>();
            public int RegisterResult;
            public int RegisterCalls;
            public int UnregisterCalls;
            public WindowHandle Destination;
            public WindowHandle Source;

            public int Register (WindowHandle destination, WindowHandle source, out IntPtr thumbnail) {
                RegisterCalls++;
                Destination = destination;
                Source = source;
                thumbnail = new IntPtr(1234);
                return RegisterResult;
            }

            public int Unregister (IntPtr thumbnail) {
                UnregisterCalls++;
                return 0;
            }

            public int Update (IntPtr thumbnail, ref DwmThumbnailProperties properties) {
                Updates.Add(properties);
                return 0;
            }

        }

    }

}
