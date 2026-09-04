// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Loads and validates `catalog.json` into the `Game` list the page is given,
//! marking each entry present or missing. Rejects any `exe` or `image` path
//! that would resolve outside the cartridge.

// ########## THE GAME LIST ##########

use std::fs;
use std::path::Path;

use common::cartridge as contract;

use crate::constants::LOG_FILE;

/// The launcher's name for a catalog entry. An alias rather than a fresh type:
/// the fields are the shared contract's own (`name`, `exe`, `image`, `steam`),
/// and `game.name` / `game.exe` read the same either way.
pub type Game = contract::Entry;

/// Reads `catalog.json` from the content folder (already seeded by
/// `content::ensureLayout`), dropping any entry whose `exe` or `image` does
/// not stay inside it.
///
/// A file that cannot be read or parsed gives an empty shelf and a logged
/// reason, never a dead process: the README tells people to hand-edit this
/// file, and a stray comma must cost the covers, not the window.
pub fn load(base_dir: &Path) -> Vec<Game> {
    let path = base_dir.join(contract::CATALOG_FILE);
    let json = match fs::read_to_string(&path) {
        Ok(json) => json,
        Err(error) => {
            common::log::appendLine(
                &base_dir.join(LOG_FILE),
                &format!("UNREADABLE {}: {error}", path.display()),
            );
            return Vec::new();
        }
    };
    let games: Vec<Game> = match serde_json::from_str(&json) {
        Ok(games) => games,
        Err(error) => {
            common::log::appendLine(
                &base_dir.join(LOG_FILE),
                &format!("UNPARSABLE {}: {error}", path.display()),
            );
            return Vec::new();
        }
    };

    games
        .into_iter()
        .filter(|game| {
            let contained = contract::isContained(&game.exe) && contract::isContained(&game.image);
            if !contained {
                common::log::appendLine(
                    &base_dir.join(LOG_FILE),
                    &format!(
                        "REFUSED {}: catalog exe/image path escapes the cartridge \
                         (exe {:?}, image {:?})",
                        game.name, game.exe, game.image
                    ),
                );
            }
            contained
        })
        .collect()
}

/// The game list as handed to the page.
///
/// Rebuilt rather than passed through as the raw catalog text so each entry can
/// carry `available`: whether its exe is actually on the cartridge. Checked
/// once, at startup — a game whose files never shipped is a state of the
/// cartridge, not of a launch, and the page marks those covers as unplayable
/// instead of letting the player click into a guaranteed failure.
pub fn payload(base_dir: &Path, games: &[Game]) -> serde_json::Value {
    serde_json::Value::Array(
        games
            .iter()
            .map(|game| {
                serde_json::json!({
                    "name": game.name,
                    "exe": game.exe,
                    "image": game.image,
                    "available": base_dir.join(&game.exe).is_file(),
                })
            })
            .collect(),
    )
}
