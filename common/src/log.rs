// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Appends timestamped lines to a log file, truncating it once it passes a
//! size cap. Every error is discarded.

// ########## LOG FILE PLUMBING ##########

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::time;

/// Appends `message` to `path` behind a UTC timestamp, creating the parent
/// folder and truncating past the shared cap. Errors are discarded.
pub fn appendLine(path: &Path, message: &str) {
    // `parent()` is None only for a bare filename, which has nothing to create.
    if let Some(parent) = path.parent() {
        // If the folder cannot be made, the open below fails and is ignored too.
        let _ = fs::create_dir_all(parent);
    }
    rotateIfLarge(path, crate::constants::DEFAULT_MAX_LOG_BYTES);
    // `.append(true)` seeks to the end per write, so two processes logging at
    // once interleave whole lines instead of overwriting each other.
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        // Nothing useful to do from in here about a disk that filled up mid-line.
        let _ = writeln!(file, "{} {}", time::timestamp(), message);
    }
}

/// Replaces the file at `path` with a marker line once it passes `max_bytes`.
/// A no-op while it does not exist or is still small.
pub fn rotateIfLarge(path: &Path, max_bytes: u64) {
    // No file on the first run: nothing to trim.
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    if meta.len() > max_bytes {
        // A troubleshooting trail, not an audit record — the old content goes
        // rather than moving to a `.1` copy, as only its recent end is useful.
        let _ = fs::write(path, b"-- log truncated --\n");
    }
}

/// Creates `path`'s parent folder and reports whether the file can be appended
/// to. Asked once at startup, so an unwritable log costs nothing per line.
pub fn openAppendable(path: &Path) -> bool {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .is_ok()
}
