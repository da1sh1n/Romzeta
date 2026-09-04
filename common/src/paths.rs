// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! The PC-side folder Romzeta keeps per-user data in: the game lease, the
//! listener's install and its log. Read with `var_os` rather than `var`,
//! because a non-UTF-8 profile path is still a usable path.

// ########## THE USER DATA FOLDER ##########

use std::path::PathBuf;

/// `%LOCALAPPDATA%\Romzeta`, or `None` when the environment does not say where
/// the user's data lives.
///
/// No fallback is baked in, because the callers disagree on purpose: the lease
/// and the listener's log drop to a temp folder so a game still runs and a
/// problem is still recorded, while the installer refuses rather than write a
/// resident service somewhere temporary.
#[cfg(windows)]
pub fn romzetaDataDir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|local| PathBuf::from(local).join("Romzeta"))
}

/// `$XDG_STATE_HOME/romzeta`, else `$HOME/.local/state/romzeta`, else `None`.
#[cfg(not(windows))]
pub fn romzetaDataDir() -> Option<PathBuf> {
    match std::env::var_os("XDG_STATE_HOME") {
        Some(state_home) => Some(PathBuf::from(state_home).join("romzeta")),
        None => std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("romzeta")
        }),
    }
}
