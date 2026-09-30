/// One keyboard transition sent by the acceptance-test driver.
///
/// `SendInput` is deliberately kept behind this small value type so tests can
/// choose the same virtual-key, scan-code, extended-key, and provenance fields
/// that a real input transition would carry.  A `None` scan code uses the
/// virtual-key form of `KEYBDINPUT`; `Some` uses the scan-code form and leaves
/// `virtual_key` at zero as required by Win32.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestKeyInput {
    pub virtual_key: u16,
    pub scan_code: Option<u16>,
    pub extended: bool,
    pub key_up: bool,
    pub extra_info: usize,
}

impl TestKeyInput {
    /// Creates an explicitly marked virtual-key transition accepted by the
    /// real hook in acceptance mode.
    pub const fn marked(virtual_key: u16, key_up: bool, extra_info: usize) -> Self {
        Self {
            virtual_key,
            scan_code: None,
            extended: false,
            key_up,
            extra_info,
        }
    }

    /// Creates an explicitly marked transition using a hardware scan code.
    /// The low-level hook derives the virtual key from this scan code.
    pub const fn marked_scan_code(
        scan_code: u16,
        extended: bool,
        key_up: bool,
        extra_info: usize,
    ) -> Self {
        Self {
            virtual_key: 0,
            scan_code: Some(scan_code),
            extended,
            key_up,
            extra_info,
        }
    }

    /// Creates a virtual-key transition with caller-controlled provenance and
    /// extended-key flag.  This is useful for testing normal and ignored
    /// injected input without duplicating the `KEYBDINPUT` layout in tests.
    pub const fn with_options(
        virtual_key: u16,
        extended: bool,
        key_up: bool,
        extra_info: usize,
    ) -> Self {
        Self {
            virtual_key,
            scan_code: None,
            extended,
            key_up,
            extra_info,
        }
    }
}
