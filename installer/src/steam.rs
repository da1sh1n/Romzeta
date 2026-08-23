// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Finds a game's Steam app id on the disk it is being copied from, and says
//! where `steam_appid.txt` belongs on the cartridge.
//!
//! A copied game is outside its Steam library, so `steam_api.dll` can no longer
//! work out which app it is. That file beside the exe is how it is told.

// ########## THE STEAM APP ID ##########

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::constants::{APPID_FILE, LIBRARY_DIR, MANIFEST_BYTES, MAX_LIBRARY_DEPTH};

/// Where an id came from, so the screen can say so rather than making the user
/// wonder where a number they did not type appeared from.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Found {
    /// A `steam_appid.txt` the game already shipped.
    File,
    /// Steam's record of this install.
    Manifest,
}

/// The app id for the game in `source`, read off this PC.
///
/// Nothing here guesses. Every source is a file Steam or the game itself wrote,
/// so a `None` means "ask the user" rather than "try harder".
pub fn detect(source: &Path, exeRelative: Option<&Path>) -> Option<(u32, Found)> {
    // The exe's own folder first: that is the path steam_api.dll reads, so a
    // file there outranks one at the root of a game that has both.
    let beside_exe = exeRelative
        .and_then(|relative| relative.parent())
        .map(|dir| source.join(dir).join(APPID_FILE));

    for path in beside_exe.into_iter().chain([source.join(APPID_FILE)]) {
        if let Some(appid) = read(&path).as_deref().and_then(parse) {
            return Some((appid, Found::File));
        }
    }
    fromManifest(source).map(|appid| (appid, Found::Manifest))
}

/// An app id from text a person typed or a file held. Zero is refused: it is
/// what an empty or malformed manifest parses to, and no game has it.
pub fn parse(text: &str) -> Option<u32> {
    let trimmed = text.trim();
    if trimmed.is_empty() || !trimmed.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    trimmed.parse().ok().filter(|appid| *appid != 0)
}

/// Where `steam_appid.txt` goes for a game copied to `destination`: beside the
/// exe, which for a nested exe is *not* the game folder's root.
pub fn appidFileIn(destination: &Path, exeRelative: &Path) -> PathBuf {
    destination
        .join(exeRelative)
        .parent()
        .unwrap_or(destination)
        .join(APPID_FILE)
}

/// The id Steam recorded for the install in `source`, by finding the library
/// above it and reading the manifest whose `installdir` names that folder.
///
/// Matched on `installdir` rather than on the manifest's filename, because the
/// filename is the id — trusting it would mean picking one at random out of a
/// library holding fifty games.
fn fromManifest(source: &Path) -> Option<u32> {
    let folder = source.file_name()?.to_string_lossy().to_lowercase();

    let library = source
        .ancestors()
        .take(MAX_LIBRARY_DEPTH + 1)
        .map(|dir| dir.join(LIBRARY_DIR))
        .find(|library| library.is_dir())?;

    for entry in std::fs::read_dir(&library).ok()?.flatten() {
        let path = entry.path();
        let name = path.file_name()?.to_string_lossy().to_lowercase();
        if !name.starts_with("appmanifest_") || !name.ends_with(".acf") {
            continue;
        }
        let Some((appid, installdir)) = read(&path).as_deref().and_then(manifest) else {
            continue;
        };
        if installdir.to_lowercase() == folder {
            return Some(appid);
        }
    }
    None
}

/// `appid` and `installdir` out of one manifest, if it holds both.
///
/// The format is Valve's KeyValues: one `"key"\t\t"value"` pair per line, in
/// braces this does not need to follow. Only the two keys are looked at, and a
/// nested block that happened to repeat one cannot win — the first wins.
pub fn manifest(text: &str) -> Option<(u32, String)> {
    let mut appid = None;
    let mut installdir = None;
    for line in text.lines() {
        match quotedPair(line) {
            Some(("appid", value)) if appid.is_none() => appid = parse(value),
            Some(("installdir", value)) if installdir.is_none() => {
                installdir = Some(value.to_string())
            }
            _ => {}
        }
        if appid.is_some() && installdir.is_some() {
            break;
        }
    }
    Some((appid?, installdir?))
}

/// The first two double-quoted tokens on a line, which is one KeyValues pair.
fn quotedPair(line: &str) -> Option<(&str, &str)> {
    let mut parts = line.split('"').skip(1).step_by(2);
    Some((parts.next()?, parts.next()?))
}

/// A small text file, or `None` for anything that isn't one. Every failure here
/// is a reason to look somewhere else, never a reason to stop.
fn read(path: &Path) -> Option<String> {
    let mut bytes = Vec::new();
    File::open(path)
        .ok()?
        .take(MANIFEST_BYTES as u64)
        .read_to_end(&mut bytes)
        .ok()?;
    String::from_utf8(bytes).ok()
}
