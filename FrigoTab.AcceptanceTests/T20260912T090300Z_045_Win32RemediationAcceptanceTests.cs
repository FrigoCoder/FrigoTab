using FrigoTab.AcceptanceTests.Support;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("Regression")]
    public sealed class T20260912T090300Z_045_Win32RemediationAcceptanceTests {

        [TestMethod]
        public void DwmThumbnailsRequestVisibility () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.DwmThumbnailVisibilityRequested,
                "Thumbnail registration/update must request DWM_TNP_VISIBLE with fVisible = TRUE.");
        }

        [TestMethod]
        public void StaleHwndCandidatesAreSkippedSafely () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.StaleWindowsSkipped,
                "destroyed or invalid HWNDs must be ignored/recovered without aborting the session.");
        }

        [TestMethod]
        public void MinimizedWindowIsAssignedByItsRestoredRectangle () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.RestoredRectangleSelectsMonitor,
                "the restored window rectangle, not Screen.FromHandle on a minimized HWND, must choose the monitor.");
        }

        [TestMethod]
        public void ExpensiveSessionWorkIsKeptOutOfTheLowLevelHookCallback () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.HookWorkIsDeferred,
                "the WH_KEYBOARD_LL callback must only enqueue bounded work; enumeration and DWM/form creation must run outside it.");
        }

        [TestMethod]
        public void ModifierStateDoesNotDependOnAsynchronousStateInsideTheHook () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.HookModifierStateIsEventDriven,
                "LowLevelKeyboardProc runs before asynchronous key state is updated; track modifier transitions instead of calling GetAsyncKeyState inside the callback.");
        }

        [TestMethod]
        public void DwmFailureHasAControlledFallback () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.DwmFailureFallbackAvailable,
                "DWM HRESULT failures must be diagnosed and rendered through a controlled fallback.");
        }

        [TestMethod]
        public void NativeStringInteropIsExplicitlyUnicode () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.UnicodeInteropDeclared,
                "native string P/Invokes must use explicit Unicode entry points/marshaling.");
        }

        [TestMethod]
        public void NativePointerSizedValuesUsePointerSizedDeclarations () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.PointerSizedInteropDeclared,
                "x64 P/Invoke values such as dwExtraInfo and wParam must be pointer-sized.");
        }

        [TestMethod]
        public void NativeAndGdiResourcesHaveDeterministicDisposal () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.DeterministicNativeDisposal,
                "fonts, DCs, bitmaps, icons, thumbnails, and forms must be disposed deterministically on the UI thread.");
        }

        [TestMethod]
        public void ProcessHasASingleInstanceGuard () {
            CurrentWin32Capabilities capabilities = new CurrentWin32Capabilities();

            Assert.IsTrue(
                capabilities.SingleInstanceGuardPresent,
                "a second launch must not install a competing global hook/tray instance.");
        }

    }

}
