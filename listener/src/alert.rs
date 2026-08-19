// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Shows a modal warning box on a thread of its own, so the caller never
//! blocks. A no-op off Windows.

// ########## THE ONE WARNING ##########

/// A box that is still up. Dropping it leaves it standing, which is what the
/// trigger wants; `wait` is for a one-shot run that would otherwise exit and
/// take the box with it.
pub struct Warning(Option<std::thread::JoinHandle<()>>);

impl Warning {
    /// Blocks until the user dismisses the box.
    pub fn wait(self) {
        if let Some(handle) = self.0 {
            // A panic inside a `MessageBoxW` call is nothing the caller can act
            // on, and the wait is over either way.
            drop(handle.join());
        }
    }
}

/// Shows a warning and returns at once.
///
/// The listener sits in `GetMessage` on one thread for its whole life, so a box
/// shown there would stall every later device arrival until it was dismissed.
#[cfg(windows)]
pub fn warn(title: &str, message: &str) -> Warning {
    use common::utf16::wide;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MB_ICONWARNING, MB_OK, MB_SETFOREGROUND, MB_SYSTEMMODAL, MessageBoxW,
    };

    // Owned buffers: a `&str` borrowed from the caller cannot move into a
    // thread that outlives the call.
    let title = wide(title);
    let message = wide(message);
    Warning(Some(std::thread::spawn(move || {
        // No owner window, so without these flags the box can open behind a
        // fullscreen game.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                message.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONWARNING | MB_SETFOREGROUND | MB_SYSTEMMODAL,
            );
        }
    })))
}

/// No portable way to raise a dialog from a headless process, and the Linux
/// trigger is one-shot from udev with no session to show one in. The log
/// carries the same sentence.
#[cfg(not(windows))]
pub fn warn(_title: &str, _message: &str) -> Warning {
    Warning(None)
}
