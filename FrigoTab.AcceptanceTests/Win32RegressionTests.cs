using FrigoTab.AcceptanceTests.Support;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("Regression")]
    [TestCategory("ContractProbe")]
    public sealed class Win32RegressionTests {

        [TestMethod]
        public void T20260912T090300Z_038_OpeningASessionPreservesDisplayModes () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.DisplayModesPreserved,
                "T20260912T090300Z_038: session startup must not reset display modes or synthesize WM_ACTIVATEAPP.");
        }

        [TestMethod]
        public void T20260912T090300Z_039_StartupIsExplicitlyTrayOnly () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.ExplicitTrayOnlyStartup,
                "T20260912T090300Z_039: startup must use a form-less ApplicationContext and must not rely on a stack-trace visibility override.");
        }

        [TestMethod]
        public void T20260912T090300Z_040_HookInstallationFailureHasAUserFacingDiagnostic () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.HookInstallationFailureReported,
                "T20260912T090300Z_040: a failed global hook installation must be reported instead of terminating without context.");
        }

        [TestMethod]
        public void T20260912T091800Z_066_BackgroundUsesOnePreOverlayDesktopSnapshot () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.DesktopBackgroundUsesSingleSnapshot,
                "T20260912T091800Z_066: capture the composed virtual desktop once before showing the overlay; do not rebuild it from tool windows.");
        }

        [TestMethod]
        public void T20260912T093300Z_073_VisualTilesRoutePointerInputToTheSessionSurface () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.OverlayTilesRoutePointerInputToSession,
                "T20260912T093300Z_073: layered preview tiles must not intercept the owner surface's hover and click handling.");
        }

    }

}
