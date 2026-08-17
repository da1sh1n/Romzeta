// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! A hidden top-level window with no purpose of its own — see
//! `common::aumid`. Task Manager and the taskbar only fold a process into the
//! "Romzeta" group if there is a window to hang that AUMID on; without one
//! (the launcher and installer have real windows, listener has a hidden one)
//! keeper showed up as its own ungrouped "Romzeta Keeper" entry. This window
//! is never shown and never painted — it just has to exist for as long as the
//! keepalive loop runs.

// ########## THE HIDDEN WINDOW ##########

use std::path::PathBuf;
use std::ptr;
use std::thread;

use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG,
    PostThreadMessageW, RegisterClassW, TranslateMessage, WM_QUIT, WNDCLASSW, WS_OVERLAPPED,
};

use common::utf16::wide;

const WINDOW_CLASS: &str = "Romzeta.KeeperWindow";

/// Creates the hidden window on this thread, runs the keepalive loop on a
/// background thread, and pumps messages until that thread posts `WM_QUIT` —
/// which is what lets the process exit once the game does.
pub fn runBehindHiddenWindow(base_dir: PathBuf, pid: u32, playtime_path: Option<PathBuf>) {
    // If window creation fails, the keepalive loop still runs —
    // PostThreadMessageW targets the thread, not the window (same
    // reasoning as listener's addTrayIcon).
    createHiddenWindow();

    let main_thread = unsafe { GetCurrentThreadId() };
    thread::spawn(move || {
        crate::run::run(&base_dir, pid, playtime_path);
        unsafe {
            PostThreadMessageW(main_thread, WM_QUIT, 0, 0);
        }
    });

    messageLoop();
}

fn createHiddenWindow() {
    unsafe {
        let instance = GetModuleHandleW(ptr::null());
        let class_name = wide(WINDOW_CLASS);

        let mut class: WNDCLASSW = std::mem::zeroed();
        class.lpfnWndProc = Some(DefWindowProcW);
        class.hInstance = instance;
        class.lpszClassName = class_name.as_ptr();
        RegisterClassW(&class);

        let title = wide("Romzeta Keeper");
        CreateWindowExW(
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
    }
}

/// Blocks in `GetMessage` until the background thread's `PostThreadMessageW`
/// delivers `WM_QUIT`.
fn messageLoop() {
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
