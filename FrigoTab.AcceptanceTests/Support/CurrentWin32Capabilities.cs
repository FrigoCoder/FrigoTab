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

        public bool HookInstallationFailureReported {
            get {
                string program = ReadSource("FrigoTab", "Program.cs");
                return Regex.IsMatch(program, @"catch\s*\(\s*Win32Exception") &&
                    program.Contains("MessageBox.Show", StringComparison.Ordinal);
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
                string session = ReadSource("FrigoTab", "SessionForm.cs");
                return !hook.Contains("KeyEvent?.Invoke(e)", StringComparison.Ordinal) ||
                    Regex.IsMatch(session, @"HandleKeyEvents[^}]*?(BeginInvoke|PostMessage)");
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
                string source = ReadSource("FrigoTab", "Thumbnail.cs");
                bool checksNativeResult = source.Contains("ThrowExceptionForHR", StringComparison.Ordinal) ||
                    Regex.IsMatch(source, @"(?:int|HRESULT)\s+\w+\s*=\s*Dwm(?:Register|Update)Thumbnail");
                bool hasFallback = source.Contains("Fallback", StringComparison.OrdinalIgnoreCase) ||
                    source.Contains("IThumbnail", StringComparison.Ordinal);
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
                MethodInfo keybdEvent = NativeMethods().Single(method => method.DeclaringType == typeof(WindowHandle) && method.Name == "keybd_event");
                MethodInfo sendMessage = NativeMethods().Single(method => method.DeclaringType == typeof(WindowIcon) && method.Name == "SendMessageCallback");
                return IsPointer(extraInfo?.FieldType) &&
                    IsPointer(ParameterType(keybdEvent, "dwExtraInfo")) &&
                    IsPointer(ParameterType(sendMessage, "wParam"));
            }
        }

        public bool DeterministicNativeDisposal {
            get {
                string applicationWindow = ReadSource("FrigoTab", "ApplicationWindow.cs");
                string backgroundWindow = ReadSource("FrigoTab", "BackgroundWindow.cs");
                bool fontsDisposed = Regex.Matches(applicationWindow, @"new\s+Font\s*\(").Count ==
                    Regex.Matches(applicationWindow, @"using\s*\(\s*Font\b").Count;
                bool constructorsAreTransactional =
                    Regex.IsMatch(applicationWindow, @"public\s+ApplicationWindow[\s\S]*?catch\s*\{") &&
                    Regex.IsMatch(backgroundWindow, @"public\s+BackgroundWindow[\s\S]*?catch\s*\{");
                return fontsDisposed && constructorsAreTransactional && typeof(IDisposable).IsAssignableFrom(typeof(WindowIcon));
            }
        }

        public bool SingleInstanceGuardPresent {
            get {
                string source = ReadSource("FrigoTab", "Program.cs");
                return source.Contains("Mutex", StringComparison.Ordinal) ||
                    source.Contains("CreateMutex", StringComparison.Ordinal);
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
