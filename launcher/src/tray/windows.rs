// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

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
use std::ptr;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CW_USEDEFAULT, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    GetCursorPos, IDI_APPLICATION, LoadIconW, MF_STRING, RegisterClassW, SetForegroundWindow,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, WM_CONTEXTMENU, WM_LBUTTONUP, WM_RBUTTONUP,
    WNDCLASSW, WS_OVERLAPPED,
};

use common::utf16::wide;

use crate::constants::{ID_MENU_EXIT, ID_MENU_OPEN, TRAY_ICON_UID, WINDOW_CLASS, WM_TRAYICON};
use crate::ui::UserEvent;

/// Everything the window procedure needs. Held in a thread-local rather than
/// `GWLP_USERDATA`: the tray window, the launcher's own window and every
/// `wndProc` call all live on the one thread `init` is called from.
struct State {
    proxy: tao::event_loop::EventLoopProxy<UserEvent>,
    trayHwnd: HWND,
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
    let Some(hwnd) = createTrayWindow() else {
        return false;
    };
    STATE.with(|state| {
        *state.borrow_mut() = Some(State {
            proxy,
            trayHwnd: hwnd,
            iconPresent: false,
        })
    });
    true
}

fn createTrayWindow() -> Option<HWND> {
    unsafe {
        let instance = GetModuleHandleW(ptr::null());
        let class_name = wide(WINDOW_CLASS);

        let mut class: WNDCLASSW = std::mem::zeroed();
        class.lpfnWndProc = Some(wndProc);
        class.hInstance = instance;
        class.lpszClassName = class_name.as_ptr();
        if RegisterClassW(&class) == 0 {
            return None;
        }

        let title = wide("Romzeta Launcher Tray");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            ptr::null(),
        );
        (!hwnd.is_null()).then_some(hwnd)
    }
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
                WM_RBUTTONUP | WM_CONTEXTMENU => unsafe { showTrayMenu(hwnd) },
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
        let added = unsafe { addTrayIcon(state.trayHwnd) };
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
        unsafe { removeTrayIcon(state.trayHwnd) };
        state.iconPresent = false;
    });
}

unsafe fn addTrayIcon(hwnd: HWND) -> bool {
    unsafe {
        // NULL instance: the documented way to ask for one of the predefined
        // system icons rather than one from this exe's own resources.
        let icon = LoadIconW(ptr::null_mut(), IDI_APPLICATION);

        let mut data: NOTIFYICONDATAW = std::mem::zeroed();
        data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = hwnd;
        data.uID = TRAY_ICON_UID;
        data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        data.uCallbackMessage = WM_TRAYICON;
        data.hIcon = icon;
        setTip(&mut data.szTip, "Romzeta");

        Shell_NotifyIconW(NIM_ADD, &data) != 0
    }
}

unsafe fn removeTrayIcon(hwnd: HWND) {
    unsafe {
        let mut data: NOTIFYICONDATAW = std::mem::zeroed();
        data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = hwnd;
        data.uID = TRAY_ICON_UID;
        Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

/// Copies `text` (truncated to fit) into a `NOTIFYICONDATAW` fixed-size
/// UTF-16 field, NUL-terminated.
fn setTip(field: &mut [u16], text: &str) {
    let wide = wide(text);
    let n = wide.len().min(field.len());
    let truncated = n == field.len();
    field[..n].copy_from_slice(&wide[..n]);
    if truncated {
        field[n - 1] = 0;
    }
}

/// Builds and shows the right-click menu at the cursor, then acts on
/// whichever item (if any) was picked.
unsafe fn showTrayMenu(hwnd: HWND) {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        let open = wide("Open Romzeta");
        let exit = wide("Exit");
        AppendMenuW(menu, MF_STRING, ID_MENU_OPEN as usize, open.as_ptr());
        AppendMenuW(menu, MF_STRING, ID_MENU_EXIT as usize, exit.as_ptr());

        let mut point: POINT = std::mem::zeroed();
        GetCursorPos(&mut point);

        // Forces this (never-activated) window to the foreground so the
        // popup dismisses itself on an outside click — the classic Win32
        // workaround, safe here because a tray click is fresh input the
        // process just received, one of the documented foreground-lock
        // exceptions.
        SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            hwnd,
            ptr::null(),
        );
        DestroyMenu(menu);

        match cmd as u32 {
            ID_MENU_OPEN => withState(|state| {
                let _ = state.proxy.send_event(UserEvent::TrayRestoreRequested);
            }),
            ID_MENU_EXIT => withState(|state| {
                let _ = state.proxy.send_event(UserEvent::CloseRequested);
            }),
            _ => None,
        };
    }
}
