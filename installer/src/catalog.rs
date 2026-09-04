// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Writes and reads `catalog.json`. The slugs and cartridge paths its entries
//! hold are `common::cartridge`'s job; this file builds catalog strings out of
//! them.

// ########## CATALOG.JSON ##########

use std::fs;
use std::path::{Component, Path};

use common::cartridge as contract;

/// Field names and types match the launcher's `Game` exactly; it is a hard
/// error there if they don't. Re-exported so `installer::catalog::Entry` keeps
/// resolving for callers outside this block's remit.
pub use contract::Entry;

/// Reads an existing cartridge's catalog.
///
/// A missing file is an empty list, not an error: a volume can carry a
/// `.cartridge` marker and no catalog yet, and edit mode should let you add the
/// first game rather than refusing to open. A file that is *there* but unparsable
/// does fail, because overwriting it would throw away a list we couldn't read.
pub fn read(root: &Path) -> Result<Vec<Entry>, String> {
    let path = root.join(contract::CATALOG_FILE);
    let json = match fs::read_to_string(&path) {
        Ok(json) => json,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{} could not be read: {e}", path.display())),
    };
    if json.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&json)
        .map_err(|e| format!("{} is not a valid catalog: {e}", path.display()))
}

/// Writes the catalog, pretty-printed — it is a file people open and edit by
/// hand on a cartridge that is already made.
pub fn write(root: &Path, entries: &[Entry]) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(entries).expect("catalog entries always serialize");
    fs::write(root.join(contract::CATALOG_FILE), json + "\n")
}

/// `games/<slug>/<exe>` for the catalog's `exe` field.
pub fn exePath(slug: &str, exeRelative: &Path) -> String {
    format!(
        "{}/{slug}/{}",
        contract::GAMES_DIR,
        toRelativeString(exeRelative)
    )
}

/// `images/<slug>.<ext>` for the catalog's `image` field.
///
/// The extension is kept from the file the user picked rather than forced to
/// `.png`: the launcher hands the path to the webview, which goes by content and
/// not by name, and renaming a `.jpg` to `.png` only makes the cartridge harder
/// to understand later.
pub fn imagePath(slug: &str, source: &Path) -> String {
    let ext = source
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_else(|| "png".into());
    format!("{}/{slug}.{ext}", contract::IMAGES_DIR)
}

/// A path relative to something, as the catalog spells it: `/` separators, no
/// leading `./`.
pub fn toRelativeString(path: &Path) -> String {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}
