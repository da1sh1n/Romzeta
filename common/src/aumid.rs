// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Sets the process's AppUserModelID so Windows groups the launcher, listener,
//! keeper and installer under one taskbar/Task Manager entry.

// ########## APP USER MODEL ID ##########

const AUMID: &str = "Romzeta";

/// Call once, before any window or background work starts. Failure is not
/// reported — a process that could not set its AUMID still runs correctly, it
/// just doesn't group with the others.
#[cfg(windows)]
pub fn set() {
    use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    let wide: Vec<u16> = AUMID.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(wide.as_ptr());
    }
}

#[cfg(not(windows))]
pub fn set() {}
