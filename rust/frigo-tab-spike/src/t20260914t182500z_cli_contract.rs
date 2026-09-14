//! Timestamped unit contracts for command parsing and report formatting.

use super::{parse_command, report_lines, Command};
use frigo_tab_win32::SmokeReport;

#[test]
fn no_arguments_selects_side_effect_free_report() {
    assert_eq!(parse_command(["spike".to_owned()]), Ok(Command::Report));
}

#[test]
fn native_smoke_requires_an_explicit_switch() {
    assert_eq!(
        parse_command(["spike".to_owned(), "--native-smoke".to_owned()]),
        Ok(Command::NativeSmoke)
    );
}

#[test]
fn report_keeps_the_thumbnail_probe_explicit_and_side_effect_free() {
    let lines = report_lines(SmokeReport {
        windows_target: false,
        message_window: false,
        single_instance_and_tray: false,
        keyboard_hook: false,
        dwm_thumbnail: false,
        shell_gdi_capture: false,
    });

    assert!(lines
        .iter()
        .any(|line| line.starts_with("native.thumbnail_probe: explicit-only")));
    assert!(lines.iter().any(|line| line == "target: non-windows"));
}
