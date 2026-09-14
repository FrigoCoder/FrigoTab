//! A deliberately small, opt-in native probe for the Rust FrigoTab port.
//!
//! Running this executable without arguments is always side-effect free: it
//! reports which portions of the platform adapter were compiled.  The
//! `--native-smoke` operation is intentionally explicit and short-lived.  It
//! exercises only resources owned by this process; its optional hook probe is
//! pass-through and is removed immediately. It never creates a tray icon,
//! enumerates other windows, or shows a visible UI.

use std::env;
use std::process;

use frigo_tab_core::SwitcherState;
use frigo_tab_win32::{smoke_report, SmokeReport};

const NATIVE_SMOKE_SWITCH: &str = "--native-smoke";
const HELP_SWITCH: &str = "--help";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Report,
    NativeSmoke,
    Help,
}

fn parse_command<I>(arguments: I) -> Result<Command, String>
where
    I: IntoIterator<Item = String>,
{
    let mut arguments = arguments.into_iter();
    let _program = arguments.next();

    match arguments.next().as_deref() {
        None => Ok(Command::Report),
        Some(NATIVE_SMOKE_SWITCH) => {
            if arguments.next().is_some() {
                Err(format!(
                    "{NATIVE_SMOKE_SWITCH} does not accept additional arguments"
                ))
            } else {
                Ok(Command::NativeSmoke)
            }
        }
        Some(HELP_SWITCH) | Some("-h") => {
            if arguments.next().is_some() {
                Err("--help does not accept additional arguments".to_owned())
            } else {
                Ok(Command::Help)
            }
        }
        Some(other) => Err(format!("unrecognized argument: {other}")),
    }
}

fn usage() -> &'static str {
    "Usage: frigo-tab-spike [--native-smoke]\n\n\
Without arguments, print the compiled capability inventory.\n\
--native-smoke  Explicitly run a reversible, non-visual Win32 smoke probe.\n\
                 It also probes a pass-through hook and an invisible shell thumbnail.\n\
--help          Print this help."
}

fn report_lines(report: SmokeReport) -> Vec<String> {
    let target = if report.windows_target {
        "windows"
    } else {
        "non-windows"
    };

    vec![
        "FrigoTab Rust spike".to_owned(),
        "mode: report (no native resources created)".to_owned(),
        format!("target: {target}"),
        format!(
            "core_policy: available (initial state: {:?})",
            SwitcherState::Idle
        ),
        format!("native.message_window: {}", yes_no(report.message_window)),
        format!(
            "native.single_instance_and_tray: {}",
            yes_no(report.single_instance_and_tray)
        ),
        format!("native.keyboard_hook: {}", yes_no(report.keyboard_hook)),
        format!("native.dwm_thumbnail: {}", yes_no(report.dwm_thumbnail)),
        format!(
            "native.shell_gdi_capture: {}",
            yes_no(report.shell_gdi_capture)
        ),
        "native.global_hook: explicit-only (not installed by this probe)".to_owned(),
        "native.tray_icon: explicit-only (not created by this probe)".to_owned(),
        "native.thumbnail_probe: explicit-only (hidden/invisible; not run in report mode)"
            .to_owned(),
    ]
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "compiled"
    } else {
        "unavailable"
    }
}

fn run_report() -> i32 {
    let report = smoke_report();
    for line in report_lines(report) {
        println!("{line}");
    }

    if !report.windows_target {
        println!("native runtime: unsupported on this host; no native smoke was run");
    }

    0
}

#[cfg(windows)]
fn run_native_smoke() -> Result<(), String> {
    use frigo_tab_win32::{
        composition_enabled, current_thread_id, pump_message, LowLevelKeyboardHook,
        MessageOnlyWindow, MessagePumpStep, ShellThumbnailProbe, SingleInstance,
        PRIVATE_CALLBACK_MESSAGE,
    };

    // This name is intentionally separate from the production C# mutex
    // (`Local\\FrigoTab`).  The probe therefore cannot make either
    // application believe the other is the production instance.
    let _instance = SingleInstance::acquire("Local\\FrigoTab.Rust.Spike.NativeSmoke.v1")
        .map_err(|error| format_native_error("single-instance mutex", error))?;

    let window = MessageOnlyWindow::create()
        .map_err(|error| format_native_error("message-only window", error))?;

    // The probe is explicitly pass-through: it forwards every event and is
    // removed before this operation returns. It is safe to run while the
    // existing C# application is active because it never consumes input or
    // mutates the hook chain beyond its own short-lived registration.
    let hook = LowLevelKeyboardHook::install_pass_through_probe()
        .map_err(|error| format_native_error("pass-through keyboard hook", error))?;

    // The window has no visual surface.  Posting an application message and
    // then WM_QUIT gives us a bounded two-step queue exercise.  We never call
    // the unbounded run_message_loop from this probe.
    window
        .post(PRIVATE_CALLBACK_MESSAGE, 0, 0)
        .map_err(|error| format_native_error("post WM_APP probe message", error))?;
    MessageOnlyWindow::post_quit(0);

    let first = pump_message().map_err(|error| format_native_error("pump first message", error))?;
    let second = match first {
        MessagePumpStep::Dispatched => {
            Some(pump_message().map_err(|error| format_native_error("pump WM_QUIT", error))?)
        }
        MessagePumpStep::Quit(_) => None,
    };

    if !matches!(first, MessagePumpStep::Dispatched)
        || !matches!(second, Some(MessagePumpStep::Quit(0)))
    {
        return Err(format!(
            "message pump returned an unexpected bounded sequence: first={first:?}, second={second:?}"
        ));
    }

    hook.try_close()
        .map_err(|error| format_native_error("remove pass-through keyboard hook", error))?;

    let composition = match composition_enabled() {
        Ok(true) => "enabled".to_owned(),
        Ok(false) => "disabled".to_owned(),
        Err(error) => format!("unavailable ({error})"),
    };

    // ShellThumbnailProbe owns a hidden 1x1 destination and keeps the DWM
    // thumbnail invisible. Creation and explicit close are both attempted,
    // but missing Explorer/DWM support is a capability result rather than a
    // failed application smoke run.
    let shell_thumbnail_status = match ShellThumbnailProbe::create() {
        Ok(probe) => match probe.try_close() {
            Ok(()) => "registered, updated invisibly, and unregistered".to_owned(),
            Err(error) => format!("degraded (cleanup failed: {error})"),
        },
        Err(error) => format!("degraded (unavailable: {error})"),
    };

    println!("FrigoTab Rust native smoke: passed");
    println!("thread: {}", current_thread_id());
    println!("single_instance: acquired and released on the owning thread");
    println!("message_window: created and destroyed (message-only, non-visual)");
    println!("message_pump: WM_APP dispatched, WM_QUIT observed");
    println!("keyboard_hook_probe: installed, pumped, and removed (pass-through)");
    println!("dwm_composition: {composition}");
    println!("shell_thumbnail_probe: {shell_thumbnail_status}");

    Ok(())
}

#[cfg(not(windows))]
fn run_native_smoke() -> Result<(), String> {
    Err("native smoke is unsupported on this host; no native resources were created".to_owned())
}

#[cfg(windows)]
fn format_native_error(operation: &str, error: frigo_tab_win32::Error) -> String {
    match error {
        frigo_tab_win32::Error::AlreadyRunning => {
            format!("{operation} could not run because another Rust smoke probe is active")
        }
        other => format!("{operation} failed: {other}"),
    }
}

fn run(arguments: impl IntoIterator<Item = String>) -> i32 {
    let command = match parse_command(arguments) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("error: {error}\n\n{}", usage());
            return 2;
        }
    };

    match command {
        Command::Report => run_report(),
        Command::Help => {
            println!("{}", usage());
            0
        }
        Command::NativeSmoke => match run_native_smoke() {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("native smoke: {error}");
                1
            }
        },
    }
}

fn main() {
    process::exit(run(env::args()));
}

#[cfg(test)]
#[path = "t20260914t182500z_cli_contract.rs"]
mod t20260914t182500z_cli_contract;
