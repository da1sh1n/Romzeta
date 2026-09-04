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
use std::thread;

use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{DefWindowProcW, PostThreadMessageW, WM_QUIT};

use crate::constants::WINDOW_CLASS;

/// Creates the hidden window on this thread, runs the keepalive loop on a
/// background thread, and pumps messages until that thread posts `WM_QUIT` —
/// which is what lets the process exit once the game does.
pub fn runBehindHiddenWindow(base_dir: PathBuf, pid: u32, playtime_path: Option<PathBuf>) {
    // If window creation fails, the keepalive loop still runs —
    // PostThreadMessageW targets the thread, not the window (same
    // reasoning as listener's addTrayIcon).
    common::win32::hiddenWindow(WINDOW_CLASS, "Romzeta Keeper", Some(DefWindowProcW));

    let main_thread = unsafe { GetCurrentThreadId() };
    thread::spawn(move || {
        crate::run::run(&base_dir, pid, playtime_path);
        unsafe {
            PostThreadMessageW(main_thread, WM_QUIT, 0, 0);
        }
    });

    common::win32::messageLoop();
}
