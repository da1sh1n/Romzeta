// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Resolves the folder holding the cartridge's content, creates the layout on
//! first run, and seeds `config.toml` and `catalog.json` when absent.

// ########## THE CONTENT FOLDER ##########

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use common::cartridge as contract;

use crate::constants::{DEFAULT_CATALOG, DEFAULT_CONFIG};

/// The folder holding `launcher.exe` and all cartridge content: the exe's own
/// parent folder.
pub fn resolveBaseDir() -> PathBuf {
    let exe = env::current_exe().expect("failed to resolve current exe path");
    exe.parent()
        .expect("current exe has no parent directory")
        .to_path_buf()
}

/// Creates the content folders and puts config.toml/catalog.json in place.
/// Cartridge content — covers, games, catalog — is never touched once
/// present, so hand-dropped files survive every build.
///
/// Never fatal on a write-protected cartridge: every step below is attempted
/// whatever the ones before it did, and only the first problem is returned.
/// One unwritable folder is not allowed to cost the ones that would have
/// worked.
pub fn ensureLayout(base: &Path) -> Result<(), String> {
    let mut first_problem: Option<String> = None;

    // Cover art and WebView2's cache both live under assets/ so the cartridge
    // root holds only what a person put there: the exe, the two data files,
    // their games, and the logs.
    //
    // `images/` is deliberately NOT created any more. An older cartridge that
    // has one keeps it, and `assets::handleRequest` still serves from it —
    // but a fresh cartridge should not be given an empty folder it will never
    // use just because the previous layout had one.
    for sub in [
        contract::GAMES_DIR,
        contract::LOGS_DIR,
        contract::IMAGES_DIR,
        contract::WEBVIEW_CACHE_DIR,
    ] {
        if let Err(error) = fs::create_dir_all(base.join(sub))
            && first_problem.is_none()
        {
            first_problem = Some(format!("could not create {sub}/: {error}"));
        }
    }

    // Written once if missing, then never rewritten. The cartridge's owner
    // owns its config, and an update must not restyle their launcher out from
    // under them. The one thing that does still happen is
    // `config::syncDefaults` appending (commented, inert) documentation for a
    // setting that didn't exist when this file was written — see its doc
    // comment.
    let config_path = base.join(contract::CONFIG_FILE);
    if let Err(problem) = seedIfMissing(&config_path, DEFAULT_CONFIG)
        && first_problem.is_none()
    {
        first_problem = Some(problem);
    }
    // A no-op for a config.toml that seedIfMissing just wrote fresh (it
    // already has every key); this is what catches up one written before
    // some setting existed.
    crate::config::syncDefaults(&config_path);

    if let Err(problem) = seedIfMissing(&base.join(contract::CATALOG_FILE), DEFAULT_CATALOG)
        && first_problem.is_none()
    {
        first_problem = Some(problem);
    }

    match first_problem {
        Some(problem) => Err(problem),
        None => Ok(()),
    }
}

/// Writes `contents` at `path` only if nothing is there, so a cartridge's own
/// copy is never overwritten.
fn seedIfMissing(path: &Path, contents: &str) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    fs::write(path, contents)
        .map_err(|error| format!("could not write {}: {error}", path.display()))
}
