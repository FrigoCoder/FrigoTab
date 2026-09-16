pub(super) fn write_diagnostic(message: &str) {
    let message = format!("{message}\0");
    let wide: Vec<u16> = message.encode_utf16().collect();
    unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::OutputDebugStringW(wide.as_ptr());
    }
}
