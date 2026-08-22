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
pub fn ensureLayout(base: &Path) {
    // Cover art and WebView2's cache both live under assets/ so the cartridge
    // root holds only what a person put there: the exe, the two data files,
    // their games, and the logs.
    //
    // `images/` is deliberately NOT created any more. An older cartridge that
    // has one keeps it, and `assets::handleRequest` still serves from it —
    // but a fresh cartridge should not be given an empty folder it will never
    // use just because the previous layout had one.
    for sub in ["games", "logs", "assets/images", "assets/EBWebView"] {
        fs::create_dir_all(base.join(sub))
            .unwrap_or_else(|e| panic!("failed to create {sub}/: {e}"));
    }

    // Written once if missing, then never rewritten. The cartridge's owner
    // owns its config, and an update must not restyle their launcher out from
    // under them. The one thing that does still happen is
    // `config::syncDefaults` appending (commented, inert) documentation for a
    // setting that didn't exist when this file was written — see its doc
    // comment.
    let config_path = base.join("config.toml");
    seedIfMissing(&config_path, DEFAULT_CONFIG);
    // A no-op for a config.toml that seedIfMissing just wrote fresh (it
    // already has every key); this is what catches up one written before
    // some setting existed.
    crate::config::syncDefaults(&config_path);

    seedIfMissing(&base.join("catalog.json"), DEFAULT_CATALOG);
}

fn seedIfMissing(path: &Path, contents: &str) {
    if !path.exists() {
        fs::write(path, contents)
            .unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));
    }
}
