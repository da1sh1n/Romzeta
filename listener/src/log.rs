// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Resolves where this listener writes its log — beside the exe, or
//! `%LOCALAPPDATA%\Romzeta` when that is not writable, or nowhere — and holds
//! the handle. The writing itself is `common::log`.

// ########## THE LISTENER'S LOG ##########

use std::env;
use std::path::{Path, PathBuf};

use crate::constants::LOG_FILE;

pub struct Log {
    /// `None` means no usable path: the listener runs silently rather than
    /// refusing to start.
    path: Option<PathBuf>,
}

impl Log {
    /// Opens (or creates) the log at `path`, creating parent folders as needed.
    /// `None` means no log at all.
    pub fn open(path: Option<PathBuf>) -> Log {
        let Some(path) = path else {
            return Log { path: None };
        };
        // Proved once here, not per line, so an unwritable log costs nothing
        // per device event.
        if common::log::openAppendable(&path) {
            return Log { path: Some(path) };
        }
        let fallback = fallbackLogPath();
        Log {
            path: common::log::openAppendable(&fallback).then_some(fallback),
        }
    }

    /// Where this log is writing. For the tray menu's "Open log".
    #[cfg(windows)]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Appends one timestamped line. Errors are ignored: a failed write stops
    /// the logging, not the listener.
    pub fn line(&self, message: &str) {
        let Some(path) = &self.path else {
            return;
        };
        common::log::appendLine(path, message);
    }
}

// ========== Where It Writes ==========

/// The log beside the exe in `dir`, so the listener's two files sit in one
/// folder you can open.
pub fn defaultLogPath(dir: &Path) -> PathBuf {
    dir.join(LOG_FILE)
}

/// Where the log goes when the folder beside the exe turns out to be
/// read-only, or when the platform's data folder cannot be named at all.
#[cfg(windows)]
fn fallbackLogPath() -> PathBuf {
    match common::paths::romzetaDataDir() {
        Some(dir) => dir.join(LOG_FILE),
        None => env::temp_dir().join("Romzeta").join(LOG_FILE),
    }
}

#[cfg(not(windows))]
fn fallbackLogPath() -> PathBuf {
    match common::paths::romzetaDataDir() {
        Some(dir) => dir.join(LOG_FILE),
        None => env::temp_dir().join("romzeta").join(LOG_FILE),
    }
}
