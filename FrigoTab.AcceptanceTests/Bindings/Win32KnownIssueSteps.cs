using FrigoTab.AcceptanceTests.Support;
using Microsoft.VisualStudio.TestTools.UnitTesting;
using Reqnroll;

namespace FrigoTab.AcceptanceTests.Bindings {

    [Binding]
    public sealed class Win32KnownIssueSteps {

        private CurrentWin32Capabilities capabilities;

        [Given(@"the current Win32 capabilities")]
        public void GivenTheCurrentWin32Capabilities () {
            capabilities = new CurrentWin32Capabilities();
        }

        [Then(@"DWM thumbnail visibility is requested")]
        public void ThenDwmThumbnailVisibilityIsRequested () {
            RequireCapabilities();
            Assert.IsTrue(
                capabilities.DwmThumbnailVisibilityRequested,
                "KI-THUMB-001: Thumbnail registration/update must request DWM_TNP_VISIBLE with fVisible = TRUE.");
        }

        [Then(@"display modes are preserved while opening a session")]
        public void ThenDisplayModesArePreservedWhileOpeningASession () {
            RequireCapabilities();
            Assert.IsTrue(
                capabilities.DisplayModesPreserved,
                "KI-DISPLAY-001: session startup must not reset display modes or synthesize WM_ACTIVATEAPP.");
        }

        [Then(@"the application uses an explicit tray-only message loop")]
        public void ThenTheApplicationUsesAnExplicitTrayOnlyMessageLoop () {
            RequireCapabilities();
            Assert.IsTrue(
                capabilities.ExplicitTrayOnlyStartup,
                "FT-START-001: startup must use a form-less ApplicationContext and must not rely on a stack-trace visibility override.");
        }

        [Then(@"a hook installation failure has a user-facing diagnostic")]
        public void ThenAHookInstallationFailureHasAUserFacingDiagnostic () {
            RequireCapabilities();
            Assert.IsTrue(
                capabilities.HookInstallationFailureReported,
                "KI-HOOK-STARTUP-001: a failed global hook installation must be reported instead of terminating without context.");
        }

        [Then(@"stale HWND candidates are skipped safely")]
        public void ThenStaleHwndCandidatesAreSkippedSafely () {
            RequireCapabilities();
            Assert.IsTrue(
                capabilities.StaleWindowsSkipped,
                "KI-STALE-001: destroyed or invalid HWNDs must be ignored/recovered without aborting the session.");
        }

        [Then(@"restored rectangles determine candidate monitors")]
        public void ThenRestoredRectanglesDetermineCandidateMonitors () {
            RequireCapabilities();
            Assert.IsTrue(
                capabilities.RestoredRectangleSelectsMonitor,
                "KI-LAYOUT-MONITOR-001: the restored window rectangle, not Screen.FromHandle on a minimized HWND, must choose the monitor.");
        }

        [Then(@"session opening work is deferred outside the low-level hook callback")]
        public void ThenSessionOpeningWorkIsDeferredOutsideTheLowLevelHookCallback () {
            RequireCapabilities();
            Assert.IsTrue(
                capabilities.HookWorkIsDeferred,
                "KI-HOOK-LATENCY-001: the WH_KEYBOARD_LL callback must only enqueue bounded work; enumeration and DWM/form creation must run outside it.");
        }

        [Then(@"hook modifier state is derived from the event stream")]
        public void ThenHookModifierStateIsDerivedFromTheEventStream () {
            RequireCapabilities();
            Assert.IsTrue(
                capabilities.HookModifierStateIsEventDriven,
                "KI-HOOK-MODIFIER-001: LowLevelKeyboardProc runs before asynchronous key state is updated; track modifier transitions instead of calling GetAsyncKeyState inside the callback.");
        }

        [Then(@"DWM failure has a controlled fallback")]
        public void ThenDwmFailureHasAControlledFallback () {
            RequireCapabilities();
            Assert.IsTrue(
                capabilities.DwmFailureFallbackAvailable,
                "KI-DWM-FAILURE-001: DWM HRESULT failures must be diagnosed and rendered through a controlled fallback.");
        }

        [Then(@"native string interop is explicitly Unicode")]
        public void ThenNativeStringInteropIsExplicitlyUnicode () {
            RequireCapabilities();
            Assert.IsTrue(
                capabilities.UnicodeInteropDeclared,
                "KI-INTEROP-UNICODE: native string P/Invokes must use explicit Unicode entry points/marshaling.");
        }

        [Then(@"native pointer-sized values use pointer-sized declarations")]
        public void ThenNativePointerSizedValuesUsePointerSizedDeclarations () {
            RequireCapabilities();
            Assert.IsTrue(
                capabilities.PointerSizedInteropDeclared,
                "KI-INTEROP-POINTER: x64 P/Invoke values such as dwExtraInfo and wParam must be pointer-sized.");
        }

        [Then(@"native and GDI resources have deterministic disposal")]
        public void ThenNativeAndGdiResourcesHaveDeterministicDisposal () {
            RequireCapabilities();
            Assert.IsTrue(
                capabilities.DeterministicNativeDisposal,
                "KI-RESOURCE-001: fonts, DCs, bitmaps, icons, thumbnails, and forms must be disposed deterministically on the UI thread.");
        }

        [Then(@"the process has a single-instance guard")]
        public void ThenTheProcessHasASingleInstanceGuard () {
            RequireCapabilities();
            Assert.IsTrue(
                capabilities.SingleInstanceGuardPresent,
                "KI-INSTANCE-001: a second launch must not install a competing global hook/tray instance.");
        }

        private void RequireCapabilities () {
            Assert.IsNotNull(capabilities, "The current Win32 capabilities fixture was not initialized.");
        }

    }

}
