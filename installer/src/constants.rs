// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Every constant the crate owns: the names written onto a cartridge, the
//! registry paths Windows is addressed by, the weights behind exe detection,
//! and the embedded payload. Section headers name the module each belongs to.

// ########## INSTALLER CONSTANTS ##########

use std::time::Duration;

#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    BusType1394, BusTypeMmc, BusTypeSd, BusTypeUsb, STORAGE_BUS_TYPE,
};

// ========== Writing A Cartridge (cartridge.rs) ==========

pub const CONFIG_FILE: &str = "config.toml";

/// The launcher's filename on a Windows cartridge, and the file the listener
/// looks for. Both sides hardcode it — see `../../listener/src/trust.rs`, which
/// explains why letting the cartridge name its own binary was a liability
/// rather than a feature.
pub const LAUNCHER_NAME: &str = "launcher.exe";

/// The launcher's detached keepalive worker, written beside it on the same
/// cartridge — `keeper::spawn` finds it by looking next to whichever
/// `launcher.exe` is currently running.
pub const KEEPER_NAME: &str = "keeper.exe";

/// Headroom demanded on top of the measured bytes before a copy is offered.
///
/// The measurement is a sum of file sizes, and what a filesystem actually
/// consumes is that plus per-file slack, directory entries and whatever the
/// volume's cluster size rounds each file up to. Filling a cartridge to the last
/// byte also leaves the launcher no room for its log and WebView2 cache.
pub const FREE_SPACE_SLACK: u64 = 256 * 1024 * 1024;

// ========== Catalog.json (catalog.rs) ==========

pub const CATALOG_FILE: &str = "catalog.json";
pub const GAMES_DIR: &str = "games";

/// Cover art, under the cartridge's `assets/` folder alongside WebView2's own
/// cache — so the root holds only what a person put there.
///
/// This was a bare `images/` and a cartridge written by an older installer
/// still says so in its catalog. Nothing needs migrating: the launcher serves
/// whatever path the catalog names, and accepts both prefixes.
pub const IMAGES_DIR: &str = "assets/images";

// ========== The Cancellable Copy (copy.rs) ==========

/// Read/write unit. Large enough that the syscall overhead is irrelevant next to
/// the disk, small enough that cancel is felt immediately even on slow USB.
pub const CHUNK_BYTES: usize = 1024 * 1024;

// ========== Finding The Game's Exe (detect.rs) ==========

/// How much better the top score must be before it is treated as a clear
/// winner. One depth level is worth [`DEPTH_PENALTY`], so this threshold means
/// "shallower than the runner-up, or a better name match" — a rank the runner-up
/// can't be within noise of.
pub const CLEAR_WINNER_MARGIN: i64 = DEPTH_PENALTY;

/// Cost of each folder level between the game root and the exe. The launcher of
/// a game is near its root; its tools are buried.
pub const DEPTH_PENALTY: i64 = 120;

/// The exe is named after the folder it is in — by far the strongest signal.
pub const EXACT_NAME_BONUS: i64 = 500;
/// Weaker version of the same: the name contains the folder name or vice versa.
pub const PARTIAL_NAME_BONUS: i64 = 200;

/// Size contributes, but only as a tiebreak — capped so a 40 GB packed
/// executable cannot outrank a correctly named one at the root.
pub const MAX_SIZE_SCORE: i64 = 100;

/// Stop walking below this depth. Nothing this deep in a game folder is the
/// game, and it bounds the scan of a pathological tree.
pub const MAX_DEPTH: usize = 8;

/// A file this small is a stub or a shim, not a game binary.
pub const MIN_PLAUSIBLE_BYTES: u64 = 16 * 1024;

// ========== The System Ui Font (font.rs) ==========

/// Keys into [`egui::FontDefinitions::font_data`]. Names, not faces — what the
/// desktop is actually using is decided at runtime and is nobody's business
/// here.
pub const SYSTEM: &str = "system-ui";
pub const FALLBACK: &str = "ubuntu-light";

// ========== Cover Dimensions (image.rs) ==========

/// The size the launcher's covers are laid out at, and the shape that follows
/// from it. Quoted to the user by both the hint beside the picker and the
/// warning below, so it is stated once here.
pub const TARGET_WIDTH: u32 = 600;
pub const TARGET_HEIGHT: u32 = 900;
pub const TARGET_RATIO: f64 = TARGET_WIDTH as f64 / TARGET_HEIGHT as f64;

/// How far off 2:3 a cover may be before it is worth mentioning. A percent or
/// two is rounding in whatever tool produced the file.
pub const RATIO_TOLERANCE: f64 = 0.02;

/// Enough for every header the image module understands, and for the JPEG
/// segment walk to reach a real SOF marker in practice.
pub const HEADER_BYTES: usize = 64 * 1024;

// ========== Installing The Listener (listener.rs) ==========

pub const EXE_NAME: &str = if cfg!(windows) {
    "listener.exe"
} else {
    "listener"
};

/// The config file earlier builds wrote. Nothing reads it any more; it is named
/// here only so an upgrade can clear it away rather than leave a file behind
/// that looks like it still configures something.
pub const STALE_CONFIG_FILE: &str = "config.toml";

/// The folder name, under `%LOCALAPPDATA%`.
pub const FOLDER: &str = "Romzeta";

/// Name of the `Run` value. Also what the user sees in Task Manager's Startup
/// tab, so it is a product name and not an exe name.
pub const AUTOSTART_NAME: &str = "Romzeta Listener";

// ========== Autoplay Suppression (autoplay.rs) ==========

/// The AutoPlay event for "a drive with ordinary files on it just arrived".
/// Named separately from the paths below because it is the thing being talked
/// about; the paths are just where Windows keeps the answer.
#[cfg(windows)]
pub const CHOSEN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\AutoplayHandlers\UserChosenExecuteHandlers\StorageOnArrival";

/// The parallel key the Settings app reads to show the current selection.
/// Writing only [`CHOSEN_KEY`] works, but leaves Settings displaying the old
/// choice — which reads as the change not having taken.
#[cfg(windows)]
pub const DEFAULT_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\AutoplayHandlers\EventHandlersDefaultSelection\StorageOnArrival";

/// The handler that means "do nothing". A real registered handler
/// (`HKLM\…\AutoplayHandlers\Handlers\MSTakeNoAction`, ProgID
/// `Shell.AutoplaySpecial`), not an invented value — deleting the choice
/// instead would fall back to the "ask me every time" popup, which is still a
/// thing appearing over the launcher.
#[cfg(windows)]
pub const TAKE_NO_ACTION: &str = "MSTakeNoAction";

/// Ours, and the only key outside AutoPlay's own that this program writes. What
/// was there before we changed it is parked here so uninstalling can put it
/// back — a setting silently changed and never restored is the kind of thing
/// people find years later and cannot explain.
#[cfg(windows)]
pub const BACKUP_KEY: &str = r"Software\Romzeta\AutoPlay";

#[cfg(windows)]
pub const BACKUP_CHOSEN: &str = "PreviousChosen";
#[cfg(windows)]
pub const BACKUP_DEFAULT: &str = "PreviousDefault";

/// Recorded when the value we are replacing was not set at all, so that
/// restoring knows to delete rather than to write something back. A literal
/// handler name could never collide with this, since handler names are registry
/// key names.
#[cfg(windows)]
pub const NONE_SENTINEL: &str = "<none>";

/// Per-user autostart. The listener is resident on Windows — it has to be
/// running to hear `WM_DEVICECHANGE` — so something must start it at login.
/// `HKCU\…\Run` is the lightest thing that does, and it needs no admin.
#[cfg(windows)]
pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

// ========== Which Drives Can Be Unplugged (volume.rs) ==========

/// The buses that mean "the user can unplug this".
#[cfg(windows)]
pub const EXTERNAL: [STORAGE_BUS_TYPE; 4] = [BusTypeUsb, BusType1394, BusTypeSd, BusTypeMmc];

// ========== The Steam App Id (steam.rs) ==========

/// The name `steam_api.dll` looks for, beside the executable it is loaded into.
pub const APPID_FILE: &str = "steam_appid.txt";

/// Steam's own install records, one `appmanifest_<appid>.acf` per installed game.
pub const LIBRARY_DIR: &str = "steamapps";

/// How far above the game folder to look for [`LIBRARY_DIR`]. The standard
/// layout puts it two up (`<library>/steamapps/common/<game>`); the rest is
/// slack for a game whose files sit a level deeper.
pub const MAX_LIBRARY_DEPTH: usize = 4;

/// A manifest is a couple of kilobytes. The cap is not a limit anyone reaches;
/// it is what stops a file that is not really a manifest from being read whole.
pub const MANIFEST_BYTES: usize = 64 * 1024;

// ========== Waking The Cartridge (wake.rs) ==========

/// Why three and not one: the first read can be answered from the OS cache
/// while the disk underneath is still spinning up. A third one still coming
/// back is what says the drive itself is awake and serving.
pub const PROBES: u64 = 3;

/// Long enough that the rounds are separate reads rather than one read and two
/// echoes of it, short enough to stay under the eye.
pub const PROBE_GAP: Duration = Duration::from_millis(200);

// ========== The Wizard Frame (ui/mod.rs, ui/games.rs) ==========

/// Warnings and blockers. Read against both the light and dark egui themes.
pub const WARN: egui::Color32 = egui::Color32::from_rgb(0xe0, 0xb1, 0x3a);
pub const BAD: egui::Color32 = egui::Color32::from_rgb(0xd1, 0x3a, 0x3a);
pub const GOOD: egui::Color32 = egui::Color32::from_rgb(0x5c, 0xb8, 0x5c);

/// Where to go to find an app id by hand. The site indexes every app on Steam,
/// including the ones the store no longer lists.
pub const STEAMDB_URL: &str = "https://steamdb.info";

// ========== The Embedded Payload (payload.rs) ==========

/// The cartridge's app, written to `<volume>/launcher.exe` — packed.
///
/// Unpacked, these bytes carry the minisign signature `xtask sign` appended
/// before this crate was built, and that signature *is* the cartridge's
/// identity. `build.rs` verifies it before packing, so an installer that would
/// produce cartridges its own listener rejects cannot be built.
pub const LAUNCHER_EXE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/launcher.exe.z"));

/// The PC-side service, written into the listener's install folder — packed.
pub const LISTENER_EXE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/listener.exe.z"));

/// The launcher's detached keepalive worker, written to `<volume>/keeper.exe`
/// beside it — packed.
pub const KEEPER_EXE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/keeper.exe.z"));

/// Seed for a new cartridge's `config.toml` — look and feel only, no key.
pub const LAUNCHER_CONFIG: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/launcher-config.toml"));

/// Seed for a cartridge's `catalog.json`. Never read: job 1 writes a catalog
/// built from the games the user actually chose, and the launcher's seed is an
/// empty list by design — a launcher must never invent games it can't run, so
/// there is no example entry here to read the shape off either. Staged only so
/// the two crates keep pointing at one file.
#[allow(dead_code)]
pub const LAUNCHER_CATALOG: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/launcher-catalog.json"));
