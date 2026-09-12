using FrigoTab.AcceptanceTests.Support;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("Regression")]
    [TestCategory("ContractProbe")]
    public sealed class T20260912T090300Z_038_Win32RegressionTests {

        [TestMethod]
        public void OpeningASessionPreservesDisplayModes () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.DisplayModesPreserved,
                "session startup must not reset display modes or synthesize WM_ACTIVATEAPP.");
        }

        [TestMethod]
        public void StartupIsExplicitlyTrayOnly () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.ExplicitTrayOnlyStartup,
                "startup must use a form-less ApplicationContext and must not rely on a stack-trace visibility override.");
        }

        [TestMethod]
        public void DebugSafetyTimerRemainsAliveUntilItExitsTheSmokeRun () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.DebugSafetyTimerIsRootedUntilItFires,
                "the Debug safety timer must remain rooted until it fires and disposes itself.");
        }

        [TestMethod]
        public void HookInstallationFailureHasAUserFacingDiagnostic () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.HookInstallationFailureReported,
                "a failed global hook installation must be reported instead of terminating without context.");
        }

        [TestMethod]
        public void BackgroundUsesOneOpaqueNativeSnapshotWithoutPerPaintCapture () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.DesktopBackgroundUsesOpaqueNativeSnapshot,
                "capture the desktop once into an opaque native frame before showing the owner, then reuse it without per-paint GDI+ capture or scaling.");
        }

        [TestMethod]
        public void ForegroundActivationDenialDoesNotReplayNativeAltTab () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.ForegroundAcquisitionIsBestEffort,
                "foreground activation is best effort after admission and must not close the overlay or replay native Alt+Tab.");
        }

        [TestMethod]
        public void GlobalHookHasItsOwnNativeMessageLoop () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.HookRunsOnDedicatedMessageLoopThread,
                "install and pump the low-level keyboard hook on a dedicated thread so UI rendering cannot time it out.");
        }

        [TestMethod]
        public void OneSubscriberFailureDoesNotPreventOtherKeyboardSubscribers () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.HookSubscriberFailuresAreIsolated,
                "one unexpected UI subscriber exception must not prevent another subscriber from admitting Alt+Tab.");
        }

        [TestMethod]
        public void VisualTilesRoutePointerInputToTheSessionSurface () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.OverlayTilesRoutePointerInputToSession,
                "layered preview tiles must not intercept the owner surface's hover and click handling.");
        }

    }

}
