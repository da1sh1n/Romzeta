// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Win32 plumbing shared by the launcher, listener and keeper: a hidden
//! top-level window and its message loop, a named-mutex single-instance
//! guard, and a tray icon with its popup menu.

// ########## WIN32 SHARED PIECES ##########

#![cfg(windows)]

use std::ptr;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, POINT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CW_USEDEFAULT, CreatePopupMenu, CreateWindowExW, DestroyMenu, DispatchMessageW,
    GetCursorPos, GetMessageW, IDI_APPLICATION, LoadIconW, MF_STRING, MSG, RegisterClassW,
    SetForegroundWindow, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage, WM_APP,
    WNDCLASSW, WNDPROC, WS_OVERLAPPED,
};

use crate::utf16::wide;

// ========== Window ==========

/// Registers the class and creates a never-shown top-level window. `None` if
/// the class or the window couldn't be created.
pub fn hiddenWindow(class: &str, title: &str, wndproc: WNDPROC) -> Option<HWND> {
    unsafe {
        let instance = GetModuleHandleW(ptr::null());
        let class_name = wide(class);

        let mut wndclass: WNDCLASSW = std::mem::zeroed();
        wndclass.lpfnWndProc = wndproc;
        wndclass.hInstance = instance;
        wndclass.lpszClassName = class_name.as_ptr();
        if RegisterClassW(&wndclass) == 0 {
            return None;
        }

        let title = wide(title);
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

/// Blocks in `GetMessage` until the window is destroyed or the session ends.
pub fn messageLoop() {
    let mut message: MSG = unsafe { std::mem::zeroed() };
    loop {
        // 0 = WM_QUIT, -1 = error. Either way there is nothing left to pump.
        let result = unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) };
        if result <= 0 {
            return;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

// ========== Single Instance ==========

/// Holds the process-wide single-instance mutex; releasing it (on drop or
/// process exit) frees the name for the next launch.
pub struct InstanceGuard(HANDLE);

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// `Some(guard)` if this is the first instance under `name`, `None` if
/// another is already running.
pub fn singleInstance(name: &str) -> Option<InstanceGuard> {
    let name = wide(name);
    unsafe {
        // A named mutex, not a lock file: the kernel frees the name when the
        // process dies, however it dies, so a crash cannot leave a stale claim.
        let handle = CreateMutexW(ptr::null(), 0, name.as_ptr());
        if handle.is_null() {
            // Never let the guard itself become a reason not to run.
            return Some(InstanceGuard(handle));
        }
        // CreateMutexW succeeds either way; only the error code separates
        // "created it" from "opened someone else's".
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(handle);
            return None;
        }
        Some(InstanceGuard(handle))
    }
}

// ========== Tray ==========

pub const TRAY_ICON_UID: u32 = 1;
/// `WM_APP` is the documented start of the range an application is free to
/// define its own messages in.
pub const WM_TRAYICON: u32 = WM_APP + 1;

/// Where a tray icon's image comes from.
pub enum TrayIcon {
    /// One of the predefined system icons, via a null instance handle.
    System,
    /// A `MAKEINTRESOURCE` id into this exe's own resources.
    Resource(*const u16),
}

/// A tray icon owned by `hwnd`. `Copy` so a window procedure can build one
/// from a bare `HWND` in one line.
#[derive(Clone, Copy)]
pub struct Tray {
    hwnd: HWND,
}

impl Tray {
    pub fn new(hwnd: HWND) -> Tray {
        Tray { hwnd }
    }

    /// Adds the icon. `false` if the shell refused it.
    pub fn add(&self, icon: TrayIcon, tip: &str) -> bool {
        unsafe {
            let icon = match icon {
                TrayIcon::System => LoadIconW(ptr::null_mut(), IDI_APPLICATION),
                TrayIcon::Resource(id) => LoadIconW(GetModuleHandleW(ptr::null()), id),
            };

            let mut data: NOTIFYICONDATAW = std::mem::zeroed();
            data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            data.hWnd = self.hwnd;
            data.uID = TRAY_ICON_UID;
            data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
            data.uCallbackMessage = WM_TRAYICON;
            data.hIcon = icon;
            setTip(&mut data.szTip, tip);

            Shell_NotifyIconW(NIM_ADD, &data) != 0
        }
    }

    /// Removes the icon, so it doesn't linger as a stale entry once the
    /// window it belongs to is gone.
    pub fn remove(&self) {
        unsafe {
            let mut data: NOTIFYICONDATAW = std::mem::zeroed();
            data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            data.hWnd = self.hwnd;
            data.uID = TRAY_ICON_UID;
            Shell_NotifyIconW(NIM_DELETE, &data);
        }
    }

    /// Builds and shows a popup menu at the cursor with one entry per item in
    /// `items`, and returns which one (if any) was picked, as an index into
    /// `items`.
    pub fn showMenu(&self, items: &[&str]) -> Option<usize> {
        unsafe {
            let menu = CreatePopupMenu();
            if menu.is_null() {
                return None;
            }
            // TrackPopupMenu returns 0 for "nothing picked", so the ids handed
            // to AppendMenuW run from 1 rather than matching `items` directly.
            let labels: Vec<Vec<u16>> = items.iter().map(|item| wide(item)).collect();
            for (index, label) in labels.iter().enumerate() {
                AppendMenuW(menu, MF_STRING, index + 1, label.as_ptr());
            }

            let mut point: POINT = std::mem::zeroed();
            GetCursorPos(&mut point);

            // Forces this (never-activated) window to the foreground so the
            // popup dismisses itself on an outside click — the classic Win32
            // workaround for the menu that won't go away.
            SetForegroundWindow(self.hwnd);
            let cmd = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                point.x,
                point.y,
                0,
                self.hwnd,
                ptr::null(),
            );
            DestroyMenu(menu);

            (cmd != 0).then_some(cmd as usize - 1)
        }
    }
}

/// Copies `text` (truncated to fit) into a `NOTIFYICONDATAW` fixed-size
/// UTF-16 field, NUL-terminated.
fn setTip(field: &mut [u16], text: &str) {
    let wide = wide(text);
    let n = wide.len().min(field.len());
    field[..n].copy_from_slice(&wide[..n]);
    field[n - 1] = 0;
}
