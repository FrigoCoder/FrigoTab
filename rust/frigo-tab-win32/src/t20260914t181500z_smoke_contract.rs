use super::{smoke_report, SmokeReport};

#[test]
fn smoke_report_is_side_effect_free_and_complete() {
    let report = smoke_report();

    #[cfg(windows)]
    assert_eq!(
        report,
        SmokeReport {
            windows_target: true,
            message_window: true,
            single_instance_and_tray: true,
            keyboard_hook: true,
            dwm_thumbnail: true,
            shell_gdi_capture: true,
        }
    );

    #[cfg(not(windows))]
    assert_eq!(report, SmokeReport::NON_WINDOWS);
}
