// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Every constant the crate owns, in one file. Section headers name the module
//! each belongs to.

// ########## KEEPER CONSTANTS ##########

// ========== The Log (run.rs) ==========

/// The keeper's log, relative to the game's base directory.
pub fn logFile() -> std::path::PathBuf {
    std::path::Path::new(common::cartridge::LOGS_DIR).join("keeper.log")
}

// ========== The Keepalive Loop (run.rs) ==========

/// Disk-touch cadence in milliseconds.
pub const KEEPALIVE_INTERVAL_MS: u64 = 10_000;

/// Process-liveness check cadence in milliseconds.
pub const PROCESS_CHECK_INTERVAL_MS: u64 = 30_000;

// ========== The Hidden Window (window.rs) ==========

#[cfg(windows)]
pub const WINDOW_CLASS: &str = "Romzeta.KeeperWindow";
