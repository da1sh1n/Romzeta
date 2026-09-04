// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

// Entry point. Answers `--version` / `--signature`, resolves the content folder
// and seeds it, takes the single-instance lock when deployed, then hands off to
// `ui::run`.
//
// No console window: this is a GUI app.
#![windows_subsystem = "windows"]
#![allow(non_snake_case)] // camelCase functions

// ########## ENTRY POINT ##########

use launcher::constants::LOG_FILE;
use launcher::{content, instance, ui};

fn main() -> wry::Result<()> {
    // Before anything else, so Task Manager groups this process under the same
    // "Romzeta" entry as the listener, keeper and installer.
    common::aumid::set();

    // Before anything touches the disk: answering a question about this exe
    // must not write to the cartridge holding it.
    if common::version::handled(env!("CARGO_PKG_VERSION"), None) {
        return Ok(());
    }

    let base_dir = content::resolveBaseDir();
    // A read-only cartridge still has to show its games, so a layout that could
    // not be written is a log line and nothing more.
    if let Err(problem) = content::ensureLayout(&base_dir) {
        common::log::appendLine(&base_dir.join(LOG_FILE), &problem);
    }

    // Single-instance: a named mutex the OS releases when the process dies.
    // Nothing listens on a port.
    let _instance = match instance::acquire() {
        Some(guard) => Some(guard),
        None => return Ok(()),
    };

    ui::run(&base_dir)
}
