// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Every constant the crate owns, in one file.
//! Section headers name the module each belongs to.

// ########## LISTENER CONSTANTS ##########

// ========== The Log (log.rs) ==========

pub const LOG_FILE: &str = "listener.log";

// ========== What A Cartridge Carries (trust.rs) ==========

/// Re-exported so `crate::constants::LAUNCHER_NAME` keeps working for
/// `tests/volume.rs` and `tests/trust.rs`, outside this block's remit.
pub use common::cartridge::LAUNCHER_NAME;

// ========== Waiting For Cartridges (trigger/windows.rs) ==========

/// How long a launcher this listener spawned is assumed to still be starting.
/// Covers the gap before it takes its mutex, which a flaky USB link's repeat
/// add events all land in.
pub const LAUNCH_GRACE_MILLISECONDS: u64 = 5000;

/// The listener's mutex name, distinct from the launcher's
/// `Local\Romzeta.CartridgeLauncher`. `Local\` scopes it to the login session,
/// matching a `Run` entry's per-user lifetime.
#[cfg(windows)]
pub const INSTANCE_MUTEX: &str = r"Local\Romzeta.CartridgeListener";

#[cfg(windows)]
pub const WINDOW_CLASS: &str = "Romzeta.ListenerWindow";

/// The icon resource `build.rs` compiles in via `winres`, which assigns id
/// `1` to the first (and only) `set_icon` call.
///
/// This is Win32's `MAKEINTRESOURCE`: an integer id passed where an `LPCWSTR`
/// is expected, which the loader tells apart from a real string by its value
/// being below 65536. `without_provenance` is the honest spelling of "this
/// address is a token, not memory" — it must stay exactly 1, so `ptr::dangling`
/// is not a substitute (that yields `align_of::<u16>()`, which is 2).
#[cfg(windows)]
pub const TRAY_ICON_RESOURCE: *const u16 = std::ptr::without_provenance(1);

#[cfg(windows)]
pub const MENU_ITEMS: [&str; 2] = ["Open log", "Exit"];
#[cfg(windows)]
pub const MENU_OPEN_LOG: usize = 0;
#[cfg(windows)]
pub const MENU_EXIT: usize = 1;
