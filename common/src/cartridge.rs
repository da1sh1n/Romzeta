// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! The cartridge's on-disk layout: the names written onto it, the files a
//! catalog entry claims as its own, and the launcher-to-keeper command line.

// ########## THE CARTRIDGE LAYOUT ##########

use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};

// ========== Names On The Cartridge ==========

/// The launcher's filename, hardcoded per platform rather than read off the
/// disk, so a cartridge cannot name the binary its own listener will run.
#[cfg(windows)]
pub const LAUNCHER_NAME: &str = "launcher.exe";
#[cfg(not(windows))]
pub const LAUNCHER_NAME: &str = "launcher";

/// The launcher's detached keepalive worker, written beside it.
#[cfg(windows)]
pub const KEEPER_NAME: &str = "keeper.exe";
#[cfg(not(windows))]
pub const KEEPER_NAME: &str = "keeper";

/// The PC-side service, which lives in the user's data folder and not on the
/// cartridge.
#[cfg(windows)]
pub const LISTENER_NAME: &str = "listener.exe";
#[cfg(not(windows))]
pub const LISTENER_NAME: &str = "listener";

pub const CATALOG_FILE: &str = "catalog.json";
pub const CONFIG_FILE: &str = "config.toml";
pub const GAMES_DIR: &str = "games";

/// Cover art, under `assets/` beside WebView2's own cache, so the cartridge
/// root holds only what a person put there.
pub const IMAGES_DIR: &str = "assets/images";

/// Where covers lived before they moved under `assets/`. Nothing creates this
/// any more; the launcher still serves it so older cartridges keep working.
pub const LEGACY_IMAGES_DIR: &str = "images";

pub const LOGS_DIR: &str = "logs";

/// Per-game playtime counter, inside that game's own folder.
pub const PLAYTIME_FILE: &str = "counter.txt";

/// WebView2's cache. The leaf name is the engine's own and cannot be chosen.
pub const WEBVIEW_CACHE_DIR: &str = "assets/EBWebView";

// ========== Paths A Catalog Entry Names ==========

/// Whether joining `relative` onto the cartridge root can only ever land
/// somewhere inside it. A drive prefix, a UNC root, a leading `/`, a `..` or a
/// path naming no component at all are all refused.
///
/// `..` is refused outright rather than resolved and range-checked, because a
/// symlink inside `games/` could make an in-range path resolve elsewhere.
pub fn isContained(relative: &str) -> bool {
    containedParts(relative).is_some()
}

/// The plain components of a contained path, or `None` if it escapes. The one
/// place the rule above is written down; everything below is built on it.
fn containedParts(relative: &str) -> Option<Vec<&OsStr>> {
    let mut parts = Vec::new();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => parts.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }
    (!parts.is_empty()).then_some(parts)
}

/// The folder on the cartridge holding one entry's game files, from the `exe`
/// path that names it.
///
/// Used by remove, which deletes that folder — so a path escaping the cartridge
/// must resolve to nothing rather than to a directory tree.
pub fn gameDir(root: &Path, catalog_exe: &str) -> Option<PathBuf> {
    let parts = containedParts(catalog_exe)?;
    // games/<slug>/… — anything shallower names no folder of its own.
    if parts.len() < 3 || parts[0] != OsStr::new(GAMES_DIR) {
        return None;
    }
    Some(root.join(parts[0]).join(parts[1]))
}

/// The `<slug>` an entry's files live under, out of the `exe` path naming it.
///
/// Derived rather than re-slugged from the name: the folder is fixed when the
/// game is added, and renaming the game afterwards does not move it.
pub fn slugOf(catalog_exe: &str) -> Option<String> {
    let parts = containedParts(catalog_exe)?;
    if parts.len() < 2 || parts[0] != OsStr::new(GAMES_DIR) {
        return None;
    }
    Some(parts[1].to_string_lossy().into_owned())
}

/// An entry's executable relative to its own folder — `bin/game.exe` out of
/// `games/<slug>/bin/game.exe`. The form the picker and the scan both speak.
pub fn exeRelative(catalog_exe: &str) -> Option<PathBuf> {
    let parts = containedParts(catalog_exe)?;
    if parts.len() < 3 || parts[0] != OsStr::new(GAMES_DIR) {
        return None;
    }
    Some(parts[2..].iter().copied().collect())
}

/// The cover file on the cartridge, from an entry's `image` path. Accepts both
/// [`IMAGES_DIR`] and [`LEGACY_IMAGES_DIR`] because the prefix comes out of the
/// cartridge's own catalog, not from here.
pub fn imageFile(root: &Path, catalog_image: &str) -> Option<PathBuf> {
    let parts = containedParts(catalog_image)?;
    Some(root.join(parts.iter().copied().collect::<PathBuf>()))
}

/// A folder- and URL-safe name derived from the game's, used for both
/// `games/<slug>/` and the cover file named after it.
///
/// Whatever the user's disk happened to hold — spaces, `™`, a trailing dot —
/// has to survive a JSON file, a webview fetch and a FAT32 volume.
pub fn slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('_') {
            slug.push('_');
        }
    }
    let slug = slug.trim_matches('_').to_string();
    if slug.is_empty() { "game".into() } else { slug }
}

/// [`slug`], with a numeric suffix when that name is already taken.
///
/// Only ever reached for two *differently named* games that squash to the same
/// slug (`Game: II` and `Game II`).
pub fn uniqueSlug(name: &str, taken: &mut HashSet<String>) -> String {
    let base = slug(name);
    let mut candidate = base.clone();
    let mut n = 2;
    while !taken.insert(candidate.clone()) {
        candidate = format!("{base}_{n}");
        n += 1;
    }
    candidate
}

// ========== The Catalog Entry ==========

/// One row of `catalog.json`. The field names are the on-disk format's, so they
/// are spelled the way the JSON spells them.
#[cfg(feature = "catalog")]
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub exe: String,
    pub image: String,
    /// Whether the launcher should bring the Steam client up before starting
    /// this game. Skipped when false so a cartridge full of DRM-free games
    /// gains no key it never needed.
    #[serde(default, skip_serializing_if = "isFalse")]
    pub steam: bool,
}

/// `skip_serializing_if` wants `fn(&T) -> bool`, which `Not::not` is not.
#[cfg(feature = "catalog")]
fn isFalse(value: &bool) -> bool {
    !*value
}

// ========== The Keeper Command Line ==========

// Private: the launcher and the keeper agree through [`KeeperArgs`], never by
// each spelling a flag. A rename here is a compile error, not a silent no-op.
const PID_FLAG: &str = "--pid";
const BASE_FLAG: &str = "--base";
const PLAYTIME_FLAG: &str = "--playtime";

/// What the launcher tells the keeper about the game it just started.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeeperArgs {
    pub pid: u32,
    pub base_dir: PathBuf,
    /// Where the keeper ticks this game's playtime counter, when it has one.
    pub playtime_path: Option<PathBuf>,
}

impl KeeperArgs {
    /// The argv the launcher hands the keeper when it spawns it.
    pub fn toArgv(&self) -> Vec<OsString> {
        let mut argv = vec![
            OsString::from(PID_FLAG),
            OsString::from(self.pid.to_string()),
            OsString::from(BASE_FLAG),
            self.base_dir.clone().into_os_string(),
        ];
        if let Some(path) = &self.playtime_path {
            argv.push(OsString::from(PLAYTIME_FLAG));
            argv.push(path.clone().into_os_string());
        }
        argv
    }
}

/// The keeper's end of the same contract, over `args_os()` past the exe name.
///
/// `None` when the pid or the base directory is missing: neither has a sane
/// default, and the keepalive loop has nothing to watch without them.
pub fn parseKeeperArgs<I: IntoIterator<Item = OsString>>(args: I) -> Option<KeeperArgs> {
    let mut args = args.into_iter();
    let mut pid = None;
    let mut base_dir = None;
    let mut playtime_path = None;

    while let Some(arg) = args.next() {
        if arg == PID_FLAG {
            pid = args.next().and_then(|value| value.to_str()?.parse().ok());
        } else if arg == BASE_FLAG {
            base_dir = args.next().map(PathBuf::from);
        } else if arg == PLAYTIME_FLAG {
            playtime_path = args.next().map(PathBuf::from);
        }
    }

    Some(KeeperArgs {
        pid: pid?,
        base_dir: base_dir?,
        playtime_path,
    })
}
