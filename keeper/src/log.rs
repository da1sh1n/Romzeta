// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Writes `logs/keeper.log`. Errors are discarded.

// ########## LOG ##########

use std::path::Path;

/// Appends one timestamped line to `logs/keeper.log` under `base_dir`.
/// Errors are ignored, and the file is truncated once it grows too large.
pub fn logLine(base_dir: &Path, message: &str) {
    common::log::appendLine(
        &base_dir.join("logs").join("keeper.log"),
        message,
        common::log::DEFAULT_MAX_LOG_BYTES,
    );
}
