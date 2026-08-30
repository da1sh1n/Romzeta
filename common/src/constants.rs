// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Every constant the crate owns, in one file. Section headers name the module
//! each belongs to.

// ########## SHARED CONSTANTS ##########

// ========== Log Files (log.rs) ==========

/// Rewrite a log from scratch once it passes this size. Every crate's log is
/// a troubleshooting trail, not an audit record, so old content is fine to
/// lose once it grows large.
pub const DEFAULT_MAX_LOG_BYTES: u64 = 1024 * 1024;

// ========== App User Model Id (aumid.rs) ==========

/// The identity Windows groups the four programs under. One string for all of
/// them is the whole point — a per-program AUMID would give each its own
/// taskbar button.
pub const AUMID: &str = "Romzeta";

// ========== The Launcher's Single Instance ==========

/// The mutex the launcher takes to be the only one open. Shared because the
/// listener opens it too, to ask whether starting another is pointless.
#[cfg(windows)]
pub const LAUNCHER_INSTANCE_MUTEX: &str = r"Local\Romzeta.CartridgeLauncher";

// ========== The Game Lease (lease.rs) ==========

/// Name of the file the launcher writes and the listener reads to learn that a
/// game is running.
pub const LEASE_FILE: &str = "active_game_lease.txt";

// ========== The Registry (reg.rs) ==========

/// Access mask for a key that is only going to be read.
#[cfg(windows)]
pub const REG_READ: u32 = windows_sys::Win32::System::Registry::KEY_QUERY_VALUE;
/// Access mask for a key that is going to be written or have values deleted.
#[cfg(windows)]
pub const REG_WRITE: u32 = windows_sys::Win32::System::Registry::KEY_SET_VALUE;
