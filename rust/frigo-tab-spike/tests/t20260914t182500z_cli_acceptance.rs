//! Timestamped command-line acceptance checks for the side-effect-free spike.

use std::process::Command;

fn executable() -> std::path::PathBuf {
    if let Some(path) = std::env::var_os("CARGO_BIN_EXE_frigo_tab_spike") {
        return std::path::PathBuf::from(path);
    }

    // Cargo does not expose CARGO_BIN_EXE_* for every target/toolchain
    // combination.  Integration tests run below target/<profile>/deps, so
    // the sibling executable is a stable fallback for ordinary cargo test
    // runs on both Windows and Unix hosts.
    let mut path = std::env::current_exe().expect("the integration test has a path");
    path.pop();
    path.pop();
    path.push(if cfg!(windows) {
        "frigo-tab-spike.exe"
    } else {
        "frigo-tab-spike"
    });
    path
}

#[test]
fn default_mode_reports_compiled_inventory_without_running_native_smoke() {
    let output = Command::new(executable())
        .output()
        .expect("default report mode should start");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("mode: report (no native resources created"));
    assert!(stdout.contains("native.global_hook: explicit-only"));
    assert!(stdout.contains("native.tray_icon: explicit-only"));
}

#[test]
fn help_mode_describes_the_explicit_native_probe() {
    let output = Command::new(executable())
        .arg("--help")
        .output()
        .expect("help mode should start");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--native-smoke"));
    assert!(stdout.contains("Explicitly run a reversible"));
    assert!(stdout.contains("pass-through hook"));
    assert!(stdout.contains("invisible shell thumbnail"));
}

#[test]
fn unknown_arguments_are_rejected_without_native_side_effects() {
    let output = Command::new(executable())
        .arg("--not-a-real-switch")
        .output()
        .expect("argument validation should start");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unrecognized argument"));
    assert!(stderr.contains("--native-smoke"));
}

#[cfg(not(windows))]
#[test]
fn native_smoke_is_a_clean_unsupported_result_off_windows() {
    let output = Command::new(executable())
        .arg("--native-smoke")
        .output()
        .expect("unsupported native smoke should start");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unsupported on this host"));
}
