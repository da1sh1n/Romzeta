// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! The tray icon shown once a game is up and the launcher window has gone.
//! Windows-only; there is no fallback for other platforms.
//!
//! Owns the tray icon on a second, never-shown window rather than the
//! launcher's own — a top-level window, so `Shell_NotifyIconW` can target
//! it, but never passed to `ShowWindow`, so it has no taskbar button.
//!
//! Created before the launcher's own `event_loop.run(...)` starts, on the
//! same thread it runs on: a thread has one Win32 message queue, and `tao`
//! pumps that whole queue, not only its own window's, so this window's
//! messages reach its `wndProc` with no plumbing into `tao`'s event loop —
//! the same trick `tray-icon`-style crates use for a foreign event loop.
//!
//! Restoring the launcher window is done by sending a `UserEvent` back
//! through `tao`'s own proxy rather than a raw `ShowWindow` on its `HWND`:
//! `tao::window::Window::set_visible` diffs against a cached flag, and a
//! raw Win32 call would never update that cache, breaking the *next*
//! `set_visible(false)`.

// ########## THE TRAY ICON ##########

use std::cell::RefCell;

use common::win32::{Tray, TrayIcon, WM_TRAYICON};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, WM_CONTEXTMENU, WM_LBUTTONUP, WM_RBUTTONUP,
};

use crate::constants::{MENU_EXIT, MENU_ITEMS, MENU_OPEN, WINDOW_CLASS};
use crate::ui::UserEvent;

/// Everything the window procedure needs. Held in a thread-local rather than
/// `GWLP_USERDATA`: the tray window, the launcher's own window and every
/// `wndProc` call all live on the one thread `init` is called from.
struct State {
    proxy: tao::event_loop::EventLoopProxy<UserEvent>,
    tray: Tray,
    iconPresent: bool,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn withState<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|state| state.borrow_mut().as_mut().map(f))
}

// ========== Window And Message Loop ==========

/// Creates the window the tray icon lives on and remembers `proxy`. `false`
/// if the class or the window couldn't be created.
pub fn init(proxy: tao::event_loop::EventLoopProxy<UserEvent>) -> bool {
    let Some(hwnd) =
        common::win32::hiddenWindow(WINDOW_CLASS, "Romzeta Launcher Tray", Some(wndProc))
    else {
        return false;
    };
    STATE.with(|state| {
        *state.borrow_mut() = Some(State {
            proxy,
            tray: Tray::new(hwnd),
            iconPresent: false,
        })
    });
    true
}

/// # Safety
///
/// Called only by Windows, with the arguments it documents for each message.
/// `extern "system"` because Windows calls it, not Rust.
unsafe extern "system" fn wndProc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_TRAYICON => {
            // With no `NIM_SETVERSION` call, the shell reports mouse activity
            // the old way: `lparam` is the mouse message itself.
            match lparam as u32 {
                WM_LBUTTONUP => {
                    withState(|state| {
                        let _ = state.proxy.send_event(UserEvent::TrayRestoreRequested);
                    });
                }
                WM_RBUTTONUP | WM_CONTEXTMENU => showTrayMenu(hwnd),
                _ => {}
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

// ========== Tray Icon ==========

/// Adds the icon. `false` if the shell refused it, or if `init` never ran —
/// either way not fatal, see the call site in `ui.rs`.
pub fn show() -> bool {
    withState(|state| {
        if state.iconPresent {
            return true;
        }
        let added = state.tray.add(TrayIcon::System, "Romzeta");
        state.iconPresent = added;
        added
    })
    .unwrap_or(false)
}

/// Removes the icon, so it doesn't linger as a stale entry once the window
/// it belongs to is gone (restored, or the process exiting).
pub fn remove() {
    withState(|state| {
        if !state.iconPresent {
            return;
        }
        state.tray.remove();
        state.iconPresent = false;
    });
}

/// Builds and shows the right-click menu at the cursor, then acts on
/// whichever item (if any) was picked.
fn showTrayMenu(hwnd: HWND) {
    let picked = Tray::new(hwnd).showMenu(&MENU_ITEMS);
    match picked {
        Some(MENU_OPEN) => withState(|state| {
            let _ = state.proxy.send_event(UserEvent::TrayRestoreRequested);
        }),
        Some(MENU_EXIT) => withState(|state| {
            let _ = state.proxy.send_event(UserEvent::CloseRequested);
        }),
        _ => None,
    };
}
