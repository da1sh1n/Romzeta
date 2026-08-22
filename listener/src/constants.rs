// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Every constant the crate owns, in one file. Section headers name the module
//! each belongs to.

// ########## LISTENER CONSTANTS ##########

// ========== The Log (log.rs) ==========

pub const LOG_FILE: &str = "listener.log";

// ========== What A Cartridge Carries (trust.rs) ==========

/// The binary a cartridge is expected to carry, by name. Hardcoded per platform
/// rather than named by the disk, so there is no attacker-supplied path to
/// sandbox in the first place.
#[cfg(windows)]
pub const LAUNCHER_NAME: &str = "launcher.exe";
#[cfg(not(windows))]
pub const LAUNCHER_NAME: &str = "launcher";

// ========== Handling One Volume (volume.rs) ==========

/// How often the gate re-checks whether the game it let through is still
/// running, in milliseconds.
pub const PROCESS_CHECK_INTERVAL_MS: u64 = 10_000;

// ========== Waiting For Cartridges (trigger/windows.rs) ==========

/// How long to ignore repeat arrivals for a drive letter already handled. A
/// flaky USB link fires several add events for one physical connection.
pub const DEBOUNCE_SECONDS: u64 = 5;

/// The listener's mutex name, distinct from the launcher's
/// `Local\Romzeta.CartridgeLauncher`. `Local\` scopes it to the login session,
/// matching a `Run` entry's per-user lifetime.
#[cfg(windows)]
pub const INSTANCE_MUTEX: &str = r"Local\Romzeta.CartridgeListener";

#[cfg(windows)]
pub const WINDOW_CLASS: &str = "Romzeta.ListenerWindow";

/// `uID` `Shell_NotifyIconW` identifies this icon by, alongside `hWnd`. One
/// tray icon per process, so any constant does — it only has to be stable
/// between the `NIM_ADD` in `addTrayIcon` and the `NIM_DELETE` on shutdown.
#[cfg(windows)]
pub const TRAY_ICON_UID: u32 = 1;

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

/// Custom message `Shell_NotifyIconW` delivers mouse activity on the tray
/// icon through. `WM_APP` is the documented start of the range an
/// application is free to define its own messages in.
#[cfg(windows)]
pub const WM_TRAYICON: u32 = windows_sys::Win32::UI::WindowsAndMessaging::WM_APP + 1;

#[cfg(windows)]
pub const ID_MENU_OPEN_LOG: u32 = 1;
#[cfg(windows)]
pub const ID_MENU_EXIT: u32 = 2;
