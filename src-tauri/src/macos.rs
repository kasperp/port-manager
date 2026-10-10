use std::cell::RefCell;

use objc2::{rc::Retained, MainThreadMarker};
use objc2_app_kit::{NSMenu, NSPopover, NSPopoverBehavior, NSViewController, NSWindow};
use objc2_foundation::{NSRectEdge, NSSize};
use tauri::{AppHandle, Manager};

// AppKit objects must stay on the main thread, including when they are dropped.
thread_local! {
    static POPOVER: RefCell<Option<Retained<NSPopover>>> = const { RefCell::new(None) };
    static TRAY_MENU: RefCell<Option<Retained<NSMenu>>> = const { RefCell::new(None) };
}

pub fn detach_tray_menu(tray: &tauri::tray::TrayIcon) -> tauri::Result<()> {
    tray.with_inner_tray_icon(|tray| {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let Some(item) = tray.ns_status_item() else {
            return;
        };
        // macOS 27 swallows left clicks while an NSMenu is attached. Retain it
        // separately and attach it only during a right-click presentation.
        TRAY_MENU.with(|slot| *slot.borrow_mut() = item.menu(mtm));
        item.setMenu(None);
    })
}

pub fn show_tray_menu(app: &AppHandle) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id("main") else {
        return Ok(());
    };
    tray.with_inner_tray_icon(|tray| {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let Some(item) = tray.ns_status_item() else {
            return;
        };
        let Some(button) = item.button(mtm) else {
            return;
        };
        // Release the RefCell borrow before AppKit enters its nested menu loop.
        let menu = TRAY_MENU.with(|slot| slot.borrow().clone());
        let Some(menu) = menu else { return };
        item.setMenu(Some(&menu));
        // A nil sender is valid for this NSButton action.
        unsafe { button.performClick(None) };
        item.setMenu(None);
    })
}

pub fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let mtm = MainThreadMarker::new().ok_or("Popover setup must run on the main thread")?;
    let window = app
        .get_webview_window("main")
        .ok_or("Main webview missing")?;
    window.hide()?;

    // Tauri owns this NSWindow. Move its existing content view into the popover
    // so the webview keeps its IPC handlers and frontend state.
    let native_window = unsafe { &*window.ns_window()?.cast::<NSWindow>() };
    let view = native_window
        .contentView()
        .ok_or("Main content view missing")?;
    let controller = NSViewController::new(mtm);
    controller.setView(&view);

    let popover = NSPopover::new(mtm);
    popover.setBehavior(NSPopoverBehavior::Transient);
    popover.setContentViewController(Some(&controller));
    popover.setContentSize(NSSize::new(600.0, 560.0));
    POPOVER.with(|slot| *slot.borrow_mut() = Some(popover));
    Ok(())
}

pub fn show(app: &AppHandle, toggle: bool) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id("main") else {
        return Ok(());
    };
    tray.with_inner_tray_icon(move |tray| {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let Some(button) = tray.ns_status_item().and_then(|item| item.button(mtm)) else {
            return;
        };
        POPOVER.with(|slot| {
            let slot = slot.borrow();
            let Some(popover) = slot.as_ref() else { return };
            if toggle && popover.isShown() {
                popover.close();
            } else {
                popover.showRelativeToRect_ofView_preferredEdge(
                    button.bounds(),
                    &button,
                    NSRectEdge::MinY,
                );
            }
        });
    })
}
