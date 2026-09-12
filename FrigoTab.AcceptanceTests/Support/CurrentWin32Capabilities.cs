using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.RegularExpressions;
using FrigoTab;

namespace FrigoTab.AcceptanceTests.Support {

    /// <summary>
    /// Executable architecture probes over the production assembly and source.
    /// They are deliberately narrower than real desktop acceptance tests: the
    /// manual Windows matrix still validates hooks, focus, HWND races, DWM, and
    /// resource counts on an interactive desktop.
    /// </summary>
    public sealed class CurrentWin32Capabilities {

        private readonly string repositoryRoot = FindRepositoryRoot();

        public bool DwmThumbnailVisibilityRequested {
            get {
                Type flags = typeof(Thumbnail).GetNestedType("ThumbnailFlags", BindingFlags.NonPublic);
                FieldInfo visible = flags?.GetField("Visible", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static);
                string source = ReadSource("FrigoTab", "Thumbnail.cs");
                return visible != null && Convert.ToInt32(visible.GetValue(null)) == 8 &&
                    Regex.IsMatch(source, @"Flags\s*=\s*[^;]*ThumbnailFlags\.Visible") &&
                    Regex.IsMatch(source, @"Visible\s*=\s*true");
            }
        }

        public bool DisplayModesPreserved {
            get {
                string source = ReadSource("FrigoTab", "SessionForm.cs");
                return !source.Contains("ChangeDisplaySettingsEx", StringComparison.Ordinal) &&
                    !Regex.IsMatch(source, @"\.PostMessage\s*\(\s*WindowMessages\.ActivateApp");
            }
        }

        public bool ExplicitTrayOnlyStartup {
            get {
                string program = ReadSource("FrigoTab", "Program.cs");
                MethodInfo overrideMethod = typeof(FrigoForm).GetMethod(
                    "SetVisibleCore",
                    BindingFlags.Instance | BindingFlags.NonPublic | BindingFlags.DeclaredOnly);
                return overrideMethod == null &&
                    Regex.IsMatch(program, @"new\s+ApplicationContext\s*\(\s*\)") &&
                    Regex.IsMatch(program, @"Application\.Run\s*\(\s*context\s*\)");
            }
        }

        public bool DebugSafetyTimerIsRootedUntilItFires {
            get {
                string program = ReadSource("FrigoTab", "Program.cs");
                return program.Contains("private static Timer debugQuitTimer", StringComparison.Ordinal) &&
                    program.Contains("debugQuitTimer = new Timer", StringComparison.Ordinal) &&
                    program.Contains("debugQuitTimer.Dispose()", StringComparison.Ordinal) &&
                    program.Contains("debugQuitTimer = null", StringComparison.Ordinal);
            }
        }

        public bool HookInstallationFailureReported {
            get {
                string program = ReadSource("FrigoTab", "Program.cs");
                return Regex.IsMatch(program, @"catch\s*\(\s*(?:Win32Exception|Exception)") &&
                    program.Contains("MessageBox.Show", StringComparison.Ordinal);
            }
        }

        public bool DesktopBackgroundUsesShellSurfaceSnapshot {
            get {
                string session = ReadSource("FrigoTab", "SessionForm.cs");
                string program = ReadSource("FrigoTab", "Program.cs");
                string snapshot = ReadSource("FrigoTab", "ShellDesktopSnapshot.cs");
                string uncommentedSnapshot = Regex.Replace(snapshot, @"//[^\r\n]*", String.Empty);
                Match tryOpen = Regex.Match(
                    session,
                    @"private\s+bool\s+TryOpen[\s\S]*?private\s+void\s+CloseSessionResources");
                Match refreshWorker = Regex.Match(
                    session,
                    @"private\s+void\s+QueueDesktopSnapshotRefresh\s*\(\s*Rectangle\s+bounds\s*\)[\s\S]*?private\s+void\s+PublishDesktopSnapshot");
                Match publication = Regex.Match(
                    session,
                    @"private\s+void\s+PublishDesktopSnapshot[\s\S]*?private\s+static\s+bool\s+TryGetVirtualBounds");
                int snapshotRegistration = session.IndexOf("new ShellDesktopSnapshot", StringComparison.Ordinal);
                int applicationsRegistration = session.IndexOf("new ApplicationWindows", StringComparison.Ordinal);
                int show = session.IndexOf("Visible = true", StringComparison.Ordinal);
                int sessionConstruction = program.IndexOf("new SessionForm", StringComparison.Ordinal);
                int hookConstruction = program.IndexOf("new KeyHook", StringComparison.Ordinal);
                int beginPublication = refreshWorker.Value.IndexOf("BeginInvoke", StringComparison.Ordinal);
                int earlyAdmissionRelease = beginPublication < 0
                    ? -1
                    : refreshWorker.Value.LastIndexOf(
                        "Interlocked.Exchange(ref desktopSnapshotRefreshRunning, 0)",
                        beginPublication,
                        StringComparison.Ordinal);
                return snapshotRegistration >= 0 &&
                    applicationsRegistration > snapshotRegistration &&
                    show > applicationsRegistration &&
                    sessionConstruction >= 0 &&
                    hookConstruction > sessionConstruction &&
                    (session.Contains("currentSnapshot.Draw", StringComparison.Ordinal) ||
                        session.Contains("currentDesktopSnapshot.Draw", StringComparison.Ordinal)) &&
                    tryOpen.Success &&
                    !tryOpen.Value.Contains("new ShellDesktopSnapshot", StringComparison.Ordinal) &&
                    session.Contains("ThreadPool.QueueUserWorkItem", StringComparison.Ordinal) &&
                    refreshWorker.Success &&
                    beginPublication >= 0 &&
                    earlyAdmissionRelease < 0 &&
                    publication.Success &&
                    publication.Value.Contains(
                        "Interlocked.Exchange(ref desktopSnapshotRefreshRunning, 0)",
                        StringComparison.Ordinal) &&
                    !session.Contains("new DesktopSnapshot", StringComparison.Ordinal) &&
                    !session.Contains("DwmGlassBackdrop", StringComparison.Ordinal) &&
                    !session.Contains("DwmDesktopBackdrop", StringComparison.Ordinal) &&
                    !session.Contains("NoRedirectionBitmap", StringComparison.Ordinal) &&
                    snapshot.Contains("IShellDesktopSnapshotApi", StringComparison.Ordinal) &&
                    snapshot.Contains("IShellDesktopSnapshotFrame", StringComparison.Ordinal) &&
                    snapshot.Contains("GdiShellDesktopSnapshotApi", StringComparison.Ordinal) &&
                    snapshot.Contains("SHELLDLL_DefView", StringComparison.Ordinal) &&
                    snapshot.Contains("SysListView32", StringComparison.Ordinal) &&
                    snapshot.Contains("FindWindowEx", StringComparison.Ordinal) &&
                    (snapshot.Contains("GetShellWindow", StringComparison.Ordinal) ||
                        snapshot.Contains("Progman", StringComparison.Ordinal)) &&
                    snapshot.Contains("WorkerW", StringComparison.Ordinal) &&
                    snapshot.Contains("PrintWindowFullContent = 0x00000002", StringComparison.Ordinal) &&
                    Regex.IsMatch(
                        uncommentedSnapshot,
                        @"PrintWindow\s*\(\s*shellDesktop\s*,\s*printedDc\s*,\s*PrintWindowFullContent\s*\)") &&
                    !Regex.IsMatch(
                        uncommentedSnapshot,
                        @"PrintWindow\s*\([^;]*,\s*0\s*\)") &&
                    snapshot.Contains("GetDC", StringComparison.Ordinal) &&
                    snapshot.Contains("CreateCompatibleDC", StringComparison.Ordinal) &&
                    snapshot.Contains("CreateDIBSection", StringComparison.Ordinal) &&
                    snapshot.Contains("SelectObject", StringComparison.Ordinal) &&
                    snapshot.Contains("BitBlt", StringComparison.Ordinal) &&
                    snapshot.Contains("StretchBlt", StringComparison.Ordinal) &&
                    snapshot.Contains("destinationBounds.Size == Size", StringComparison.Ordinal) &&
                    snapshot.Contains("ReleaseDC", StringComparison.Ordinal) &&
                    snapshot.Contains("DeleteDC", StringComparison.Ordinal) &&
                    snapshot.Contains("DeleteObject", StringComparison.Ordinal) &&
                    !Regex.IsMatch(uncommentedSnapshot, @"GetDC\s*\(\s*(?:IntPtr\.Zero|null|NULL)\s*\)") &&
                    !snapshot.Contains("GetWindowDC", StringComparison.Ordinal) &&
                    !snapshot.Contains("EnumWindows", StringComparison.Ordinal) &&
                    !snapshot.Contains("DwmRegisterThumbnail", StringComparison.Ordinal) &&
                    !snapshot.Contains("ApplicationWindows", StringComparison.Ordinal) &&
                    !Regex.IsMatch(snapshot, @"\bCopyFromScreen\s*\(") &&
                    !Regex.IsMatch(snapshot, @"\bDrawImage(?:Unscaled)?\s*\(") &&
                    !File.Exists(Path.Combine(repositoryRoot, "FrigoTab", "BackgroundWindows.cs")) &&
                    !File.Exists(Path.Combine(repositoryRoot, "FrigoTab", "DesktopSnapshot.cs"));
            }
        }

        public bool ForegroundAcquisitionIsBestEffort {
            get {
                string session = ReadSource("FrigoTab", "SessionForm.cs");
                Match foregroundFailure = Regex.Match(
                    session,
                    @"if\s*\(\s*!WindowHandle\.SetForeground\(\)\s*\)\s*\{(?<body>[^}]*)\}");
                if( !foregroundFailure.Success ) {
                    return false;
                }

                string body = foregroundFailure.Groups["body"].Value;
                return body.Contains("Trace.WriteLine", StringComparison.Ordinal) &&
                    !body.Contains("CloseSessionResources", StringComparison.Ordinal) &&
                    !body.Contains("return false", StringComparison.Ordinal);
            }
        }

        public bool OverlayTilesRoutePointerInputToSession {
            get {
                string window = ReadSource("FrigoTab", "ApplicationWindow.cs");
                return window.Contains("NonClientHitTestMessage", StringComparison.Ordinal) &&
                    window.Contains("TransparentHitTest", StringComparison.Ordinal) &&
                    window.Contains("WindowExStyles.NoActivate", StringComparison.Ordinal);
            }
        }

        public bool StaleWindowsSkipped {
            get {
                string handle = ReadSource("FrigoTab", "WindowHandle.cs");
                string layout = ReadSource("FrigoTab", "Layout.cs");
                return handle.Contains("TryGetRect", StringComparison.Ordinal) &&
                    layout.Contains("TryGetRect", StringComparison.Ordinal) &&
                    Regex.IsMatch(layout, @"if\s*\(\s*!.*TryGetRect");
            }
        }

        public bool RestoredRectangleSelectsMonitor {
            get {
                string layout = ReadSource("FrigoTab", "Layout.cs");
                return !layout.Contains("window.GetScreen().DeviceName", StringComparison.Ordinal) &&
                    (layout.Contains("Screen.FromRectangle", StringComparison.Ordinal) ||
                        layout.Contains("MonitorFromRect", StringComparison.Ordinal));
            }
        }

        public bool HookWorkIsDeferred {
            get {
                string hook = ReadSource("FrigoTab", "KeyHook.cs");
                return hook.Contains("DeferredKeyboardDispatcher", StringComparison.Ordinal) &&
                    hook.Contains("BeginInvoke(action)", StringComparison.Ordinal) &&
                    hook.Contains("ShouldConsume(input, out admissionToken)", StringComparison.Ordinal) &&
                    hook.Contains("AbortPendingAdmission(admissionToken)", StringComparison.Ordinal) &&
                    hook.Contains("deferredDispatcher.InvalidatePending()", StringComparison.Ordinal) &&
                    hook.Contains("lock( inputStateGate )", StringComparison.Ordinal) &&
                    hook.Contains("TryDispatchCritical(input)", StringComparison.Ordinal) &&
                    hook.Contains("IsSessionEndingInput", StringComparison.Ordinal) &&
                    Regex.IsMatch(hook, @"private\s+void\s+DispatchOnUiThread[\s\S]*?subscribers\.GetInvocationList\(\)");
            }
        }

        public bool HookRunsOnDedicatedMessageLoopThread {
            get {
                string hook = ReadSource("FrigoTab", "KeyHook.cs");
                Match threadMain = Regex.Match(
                    hook,
                    @"private\s+void\s+HookThreadMain\s*\(\s*\)[\s\S]*?private\s+void\s+UninstallHookOnCurrentThread");
                return hook.Contains("new Thread(HookThreadMain)", StringComparison.Ordinal) &&
                    !hook.Contains("~KeyHook", StringComparison.Ordinal) &&
                    hook.Contains("PeekMessage(", StringComparison.Ordinal) &&
                    threadMain.Success &&
                    threadMain.Value.Contains("SetWindowsHookEx(", StringComparison.Ordinal) &&
                    threadMain.Value.Contains("GetMessage(", StringComparison.Ordinal);
            }
        }

        public bool HookSubscriberFailuresAreIsolated {
            get {
                string hook = ReadSource("FrigoTab", "KeyHook.cs");
                Match dispatch = Regex.Match(
                    hook,
                    @"private\s+void\s+DispatchOnUiThread[\s\S]*?private\s+static\s+bool\s+ReplayNativeAltTab");
                if( !dispatch.Success ) {
                    return false;
                }

                Match failure = Regex.Match(
                    dispatch.Value,
                    @"catch\s*\(\s*Exception\s+exception\s*\)\s*\{(?<body>[^}]*)\}");
                string body = failure.Success ? failure.Groups["body"].Value : String.Empty;
                return dispatch.Value.Contains("GetInvocationList()", StringComparison.Ordinal) &&
                    dispatch.Value.Contains("handled |= e.Handled", StringComparison.Ordinal) &&
                    body.Contains("Debug.WriteLine(exception)", StringComparison.Ordinal) &&
                    !body.Contains("return;", StringComparison.Ordinal) &&
                    !body.Contains("SetSessionVisible", StringComparison.Ordinal) &&
                    !body.Contains("ReplayNativeAltTab", StringComparison.Ordinal);
            }
        }

        public bool HookModifierStateIsEventDriven {
            get {
                string hook = ReadSource("FrigoTab", "KeyHook.cs");
                return !hook.Contains("GetAsyncKeyState", StringComparison.Ordinal);
            }
        }

        public bool DwmFailureFallbackAvailable {
            get {
                string thumbnail = ReadSource("FrigoTab", "Thumbnail.cs");
                string applicationWindow = ReadSource("FrigoTab", "ApplicationWindow.cs");
                bool checksNativeResult = thumbnail.Contains("ThrowExceptionForHR", StringComparison.Ordinal) &&
                    thumbnail.Contains("IDwmThumbnailApi", StringComparison.Ordinal);
                bool hasFallback = applicationWindow.Contains("TryCreateThumbnail", StringComparison.Ordinal) &&
                    applicationWindow.Contains("fallback", StringComparison.OrdinalIgnoreCase);
                return checksNativeResult && hasFallback;
            }
        }

        public bool UnicodeInteropDeclared {
            get {
                foreach( MethodInfo method in NativeMethods() ) {
                    DllImportAttribute import = method.GetCustomAttribute<DllImportAttribute>();
                    if( import == null || !IsTextApi(method) ) {
                        continue;
                    }
                    if( import.CharSet != CharSet.Unicode ) {
                        return false;
                    }
                }
                return true;
            }
        }

        public bool PointerSizedInteropDeclared {
            get {
                Type keyboardData = typeof(KeyHook).GetNestedType("LowLevelKeyStruct", BindingFlags.NonPublic);
                FieldInfo extraInfo = keyboardData?.GetField("DwExtraInfo", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
                MethodInfo sendMessage = NativeMethods().Single(method => method.DeclaringType == typeof(WindowIcon) && method.Name == "SendMessageCallback");
                bool deprecatedSyntheticInputRemoved = !NativeMethods().Any(
                    method => method.DeclaringType == typeof(WindowHandle) && method.Name == "keybd_event");
                return IsPointer(extraInfo?.FieldType) &&
                    deprecatedSyntheticInputRemoved &&
                    IsPointer(ParameterType(sendMessage, "wParam")) &&
                    IsPointer(ParameterType(sendMessage, "dwData"));
            }
        }

        public bool DeterministicNativeDisposal {
            get {
                string applicationWindow = ReadSource("FrigoTab", "ApplicationWindow.cs");
                string desktopSnapshot = ReadSource("FrigoTab", "ShellDesktopSnapshot.cs");
                bool fontsDisposed = Regex.Matches(applicationWindow, @"new\s+Font\s*\(").Count ==
                    Regex.Matches(applicationWindow, @"using\s*\(\s*Font\b").Count;
                bool constructorsAreTransactional =
                    Regex.IsMatch(applicationWindow, @"public\s+ApplicationWindow[\s\S]*?catch\s*\{") &&
                    Regex.IsMatch(desktopSnapshot, @"public\s+GdiShellDesktopSnapshotFrame[\s\S]*?catch\s*\{") &&
                    desktopSnapshot.Contains("Release(ref newMemoryDc", StringComparison.Ordinal);
                bool backdropIsDisposable =
                    typeof(IDisposable).IsAssignableFrom(typeof(ShellDesktopSnapshot)) &&
                    typeof(IDisposable).IsAssignableFrom(typeof(GdiShellDesktopSnapshotFrame));
                return fontsDisposed && constructorsAreTransactional && backdropIsDisposable &&
                    typeof(IDisposable).IsAssignableFrom(typeof(WindowIcon));
            }
        }

        public bool SingleInstanceGuardPresent {
            get {
                string program = ReadSource("FrigoTab", "Program.cs");
                return typeof(IDisposable).IsAssignableFrom(typeof(SingleInstanceGuard)) &&
                    program.Contains("SingleInstanceGuard.TryAcquire", StringComparison.Ordinal);
            }
        }

        private static IEnumerable<MethodInfo> NativeMethods () {
            const BindingFlags Flags = BindingFlags.Public | BindingFlags.NonPublic |
                BindingFlags.Static | BindingFlags.Instance;
            return typeof(Program).Assembly.GetTypes().SelectMany(type => type.GetMethods(Flags))
                .Where(method => method.GetCustomAttribute<DllImportAttribute>() != null);
        }

        private static bool IsTextApi (MethodInfo method) {
            string entryPoint = method.GetCustomAttribute<DllImportAttribute>()?.EntryPoint ?? method.Name;
            if( entryPoint.StartsWith("GetWindowText", StringComparison.Ordinal) ||
                entryPoint.StartsWith("GetClassName", StringComparison.Ordinal) ||
                entryPoint.StartsWith("GetModuleHandle", StringComparison.Ordinal) ) {
                return true;
            }
            return method.GetParameters().Any(parameter =>
                parameter.ParameterType == typeof(string) ||
                parameter.ParameterType == typeof(StringBuilder) ||
                parameter.ParameterType == typeof(char[]));
        }

        private static Type ParameterType (MethodInfo method, string name) =>
            method.GetParameters().Single(parameter => parameter.Name == name).ParameterType;

        private static bool IsPointer (Type type) => type == typeof(IntPtr) || type == typeof(UIntPtr);

        private string ReadSource (params string[] path) =>
            File.ReadAllText(Path.Combine(new[] {repositoryRoot}.Concat(path).ToArray()));

        private static string FindRepositoryRoot () {
            DirectoryInfo directory = new DirectoryInfo(AppContext.BaseDirectory);
            while( directory != null ) {
                if( File.Exists(Path.Combine(directory.FullName, "FrigoTab.sln")) ) {
                    return directory.FullName;
                }
                directory = directory.Parent;
            }
            throw new DirectoryNotFoundException("Could not locate the FrigoTab repository root for production contract probes.");
        }

    }

}
