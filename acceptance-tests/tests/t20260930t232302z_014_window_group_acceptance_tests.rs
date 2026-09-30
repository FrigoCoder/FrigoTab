#![cfg(windows)]

//! Real HWND acceptance coverage for applications with owned windows.
//!
//! GIMP creates detached docks as owned tool windows. Activating such a dock
//! changes the root window's last-active popup, but must not remove the main
//! application window from FrigoTab's candidate list.

use frigotab::window::{WindowFinder, WindowHandle};
use frigotab_acceptance::{FixtureOptions, FixtureWindow, rgb, serial_guard};
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    WS_CAPTION, WS_EX_TOOLWINDOW, WS_POPUP, WS_SYSMENU,
};

#[test]
fn an_active_detached_tool_window_keeps_its_application_root_available() {
    let _serial = serial_guard();
    let application = FixtureWindow::show(
        "FrigoTab.Acceptance.GimpLike.Main",
        rgb(80, 100, 120),
        rect(100, 100, 900, 700),
    );
    let application_handle = WindowHandle::new(application.handle());
    assert!(WindowFinder::new().windows.contains(&application_handle));

    let mut detached_dock = FixtureWindow::show_with_options(
        "FrigoTab.Acceptance.GimpLike.DetachedDock",
        rgb(120, 100, 80),
        FixtureOptions {
            bounds: rect(950, 150, 1250, 650),
            style: WS_POPUP | WS_CAPTION | WS_SYSMENU,
            ex_style: WS_EX_TOOLWINDOW,
            owner: application.handle(),
            activate: true,
            ..FixtureOptions::default()
        },
    );

    let finder = WindowFinder::new();
    assert!(
        finder.windows.contains(&application_handle),
        "activating an owned tool window removed the application root"
    );
    assert!(
        !finder
            .windows
            .contains(&WindowHandle::new(detached_dock.handle())),
        "the detached tool window became a separate application candidate"
    );

    detached_dock.hide();
    let finder = WindowFinder::new();
    assert!(
        finder.windows.contains(&application_handle),
        "hiding the last-active tool window removed the application root"
    );

    detached_dock.close();
    let finder = WindowFinder::new();
    assert!(
        finder.windows.contains(&application_handle),
        "destroying the last-active tool window removed the application root"
    );
}

#[test]
fn an_active_owned_application_window_remains_the_group_representative() {
    let _serial = serial_guard();
    let application = FixtureWindow::show(
        "FrigoTab.Acceptance.OwnedPopup.Main",
        rgb(80, 100, 120),
        rect(100, 100, 900, 700),
    );
    let application_handle = WindowHandle::new(application.handle());
    let dialog = FixtureWindow::show_with_options(
        "FrigoTab.Acceptance.OwnedPopup.Dialog",
        rgb(120, 100, 80),
        FixtureOptions {
            bounds: rect(300, 250, 700, 550),
            style: WS_POPUP | WS_CAPTION | WS_SYSMENU,
            owner: application.handle(),
            activate: true,
            ..FixtureOptions::default()
        },
    );
    let dialog_handle = WindowHandle::new(dialog.handle());

    let finder = WindowFinder::new();
    assert!(!finder.windows.contains(&application_handle));
    assert!(
        finder.windows.contains(&dialog_handle),
        "an ordinary active owned window should represent its application group"
    );
}

const fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}
