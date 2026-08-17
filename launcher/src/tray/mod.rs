// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! The tray icon shown once a game is up and the launcher window has gone.
//! Entirely a Win32 affair; see `windows.rs`. Same split as `listener`'s
//! `trigger` module, one file per platform.

// ########## PER-PLATFORM SPLIT ##########

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{init, remove, show};

/// Adds the icon and remembers `proxy`. `false` if it couldn't be created —
/// a courtesy, not a requirement; see the call site in `ui.rs`.
#[cfg(not(windows))]
pub fn init(_proxy: tao::event_loop::EventLoopProxy<crate::ui::UserEvent>) -> bool {
    false
}

/// Shows the icon. `false` when there is none to show.
#[cfg(not(windows))]
pub fn show() -> bool {
    false
}

/// Removes the icon.
#[cfg(not(windows))]
pub fn remove() {}
