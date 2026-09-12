using FrigoTab.AcceptanceTests.Support;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("Regression")]
    public sealed class Win32RemediationAcceptanceTests {

        [TestMethod]
        public void T20260912T090300Z_045_DwmThumbnailsRequestVisibility () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.DwmThumbnailVisibilityRequested,
                "T20260912T090300Z_045: Thumbnail registration/update must request DWM_TNP_VISIBLE with fVisible = TRUE.");
        }

        [TestMethod]
        public void T20260912T090300Z_046_StaleHwndCandidatesAreSkippedSafely () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.StaleWindowsSkipped,
                "T20260912T090300Z_046: destroyed or invalid HWNDs must be ignored/recovered without aborting the session.");
        }

        [TestMethod]
        public void T20260912T090300Z_047_MinimizedWindowIsAssignedByItsRestoredRectangle () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.RestoredRectangleSelectsMonitor,
                "T20260912T090300Z_047: the restored window rectangle, not Screen.FromHandle on a minimized HWND, must choose the monitor.");
        }

        [TestMethod]
        public void T20260912T090300Z_048_ExpensiveSessionWorkIsKeptOutOfTheLowLevelHookCallback () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.HookWorkIsDeferred,
                "T20260912T090300Z_048: the WH_KEYBOARD_LL callback must only enqueue bounded work; enumeration and DWM/form creation must run outside it.");
        }

        [TestMethod]
        public void T20260912T090300Z_049_ModifierStateDoesNotDependOnAsynchronousStateInsideTheHook () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.HookModifierStateIsEventDriven,
                "T20260912T090300Z_049: LowLevelKeyboardProc runs before asynchronous key state is updated; track modifier transitions instead of calling GetAsyncKeyState inside the callback.");
        }

        [TestMethod]
        public void T20260912T090300Z_050_DwmFailureHasAControlledFallback () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.DwmFailureFallbackAvailable,
                "T20260912T090300Z_050: DWM HRESULT failures must be diagnosed and rendered through a controlled fallback.");
        }

        [TestMethod]
        public void T20260912T090300Z_051_NativeStringInteropIsExplicitlyUnicode () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.UnicodeInteropDeclared,
                "T20260912T090300Z_051: native string P/Invokes must use explicit Unicode entry points/marshaling.");
        }

        [TestMethod]
        public void T20260912T090300Z_052_NativePointerSizedValuesUsePointerSizedDeclarations () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.PointerSizedInteropDeclared,
                "T20260912T090300Z_052: x64 P/Invoke values such as dwExtraInfo and wParam must be pointer-sized.");
        }

        [TestMethod]
        public void T20260912T090300Z_053_NativeAndGdiResourcesHaveDeterministicDisposal () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.DeterministicNativeDisposal,
                "T20260912T090300Z_053: fonts, DCs, bitmaps, icons, thumbnails, and forms must be disposed deterministically on the UI thread.");
        }

        [TestMethod]
        public void T20260912T090300Z_054_ProcessHasASingleInstanceGuard () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.SingleInstanceGuardPresent,
                "T20260912T090300Z_054: a second launch must not install a competing global hook/tray instance.");
        }

    }

}
