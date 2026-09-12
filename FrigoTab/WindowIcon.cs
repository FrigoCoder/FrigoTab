using System;
using System.Diagnostics.CodeAnalysis;
using System.Diagnostics;
using System.Drawing;
using System.Runtime.InteropServices;

namespace FrigoTab {

    public class WindowIcon : IDisposable {

        public event Action Changed;
        public Icon Icon;

        [SuppressMessage("ReSharper", "PrivateFieldCanBeConvertedToLocalVariable")]
        private readonly SendMessageDelegate callback;
        private readonly object sync = new object();
        private bool disposed;

        public WindowIcon (WindowHandle handle) {
            IntPtr icon = GetClassLongPtr(handle, ClassLong.Icon);
            Icon = CloneIcon(icon);
            callback = Callback;
            SendMessageCallback(handle, WindowMessages.GetIcon, (UIntPtr) GetIconSize.Big, IntPtr.Zero, callback, UIntPtr.Zero);
        }

        public void Dispose () {
            Icon current;
            lock( sync ) {
                if( disposed ) {
                    return;
                }
                disposed = true;
                current = Icon;
                Icon = null;
            }
            current?.Dispose();
            GC.SuppressFinalize(this);
        }

        private void Callback (WindowHandle hWnd, int msg, UIntPtr dwData, IntPtr lResult) {
            if( lResult == IntPtr.Zero ) {
                return;
            }

            Icon replacement = null;
            try {
                replacement = CloneIcon(lResult);
                Icon previous;
                lock( sync ) {
                    if( disposed ) {
                        return;
                    }
                    previous = Icon;
                    Icon = replacement;
                    replacement = null;
                }
                previous?.Dispose();
                Changed?.Invoke();
            }
            catch( Exception exception ) {
                // SendMessageCallback enters managed code from User32. Never
                // let a stale icon or rendering failure cross that boundary.
                Trace.WriteLine("Ignoring a late or invalid window-icon callback: " + exception);
            }
            finally {
                replacement?.Dispose();
            }
        }

        private static Icon CloneIcon (IntPtr handle) {
            if( handle != IntPtr.Zero ) {
                using( Icon borrowed = Icon.FromHandle(handle) ) {
                    return (Icon) borrowed.Clone();
                }
            }
            Icon fallback = Program.Icon ?? SystemIcons.Application;
            return (Icon) fallback.Clone();
        }

        private enum ClassLong {

            Icon = -14

        }

        private enum GetIconSize {

            Big = 1

        }

        private delegate void SendMessageDelegate (WindowHandle hWnd, int msg, UIntPtr dwData, IntPtr lResult);

        [DllImport("user32.dll", EntryPoint = "GetClassLongPtrW", CharSet = CharSet.Unicode,
            ExactSpelling = true, SetLastError = true)]
        private static extern IntPtr GetClassLongPtr (WindowHandle hWnd, ClassLong nIndex);

        [DllImport("user32.dll", EntryPoint = "SendMessageCallbackW", CharSet = CharSet.Unicode,
            ExactSpelling = true, SetLastError = true)]
        private static extern bool SendMessageCallback (WindowHandle hWnd, WindowMessages message, UIntPtr wParam, IntPtr lParam,
            SendMessageDelegate lpCallBack, UIntPtr dwData);

    }

}
