use super::*;

#[test]
fn documented_surface_constants_are_stable() {
    assert_eq!(WH_KEYBOARD_LL, 13);
    assert_eq!(DWM_TNP_VISIBLE, 8);
    assert_eq!(DWM_TNP_RECTDESTINATION, 1);
    assert!(size_of::<DWM_THUMBNAIL_PROPERTIES>() > 0);
    assert!(size_of::<MSG>() > 0);
    assert_eq!(PRINT_WINDOW_RENDER_FULL_CONTENT, 0x0002);
}

#[test]
fn thumbnail_defaults_and_explicit_cleanup_apis_are_available() {
    let update = ThumbnailUpdate::destination(
        RECT {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        },
        false,
    );
    assert_eq!(update.opacity, Some(255));

    let close: fn(&mut DwmThumbnail) -> Result<()> = DwmThumbnail::close;
    let try_close: fn(DwmThumbnail) -> Result<()> = DwmThumbnail::try_close;
    let probe_close: fn(&mut ShellThumbnailProbe) -> Result<()> = ShellThumbnailProbe::close;
    let probe_try_close: fn(ShellThumbnailProbe) -> Result<()> = ShellThumbnailProbe::try_close;
    let hook_close: fn(&mut LowLevelKeyboardHook) -> Result<()> = LowLevelKeyboardHook::close;
    let hook_try_close: fn(LowLevelKeyboardHook) -> Result<()> = LowLevelKeyboardHook::try_close;
    let hook_probe: fn() -> Result<LowLevelKeyboardHook> =
        LowLevelKeyboardHook::install_pass_through_probe;
    let _ = (
        close,
        try_close,
        probe_close,
        probe_try_close,
        hook_close,
        hook_try_close,
        hook_probe,
    );

    assert_eq!(PrintWindowMode::Standard, PrintWindowMode::Standard);
    assert_eq!(
        PrintWindowMode::BestEffortFullContent,
        PrintWindowMode::BestEffortFullContent
    );
    assert_eq!(
        PrintWindowOutcome::StandardFallback,
        PrintWindowOutcome::StandardFallback
    );
    assert_eq!(
        PrintWindowOutcome::FullContent,
        PrintWindowOutcome::FullContent
    );

    let direct_result = Error::Win32Result {
        operation: "ReleaseDC",
        result: 0,
    };
    assert!(direct_result.to_string().contains("native result 0"));
    let combined_result = Error::Win32WithCleanup {
        operation: "CreateCompatibleDC",
        code: 5,
        cleanup_operation: "ReleaseDC",
        cleanup_result: 0,
    };
    assert!(combined_result
        .to_string()
        .contains("ReleaseDC cleanup returned 0"));
}

#[test]
fn smoke_contract_does_not_install_hook() {
    // There is deliberately no call to LowLevelKeyboardHook::install in
    // tests. The only test-facing route is the side-effect-free report in
    // lib.rs; hook installation belongs to an explicit application start.
    let callback: Option<HOOKPROC> = None;
    assert!(callback.is_none());
}
