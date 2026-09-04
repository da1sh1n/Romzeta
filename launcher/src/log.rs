// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Writes `logs/launcher.log`, and opens the per-game `out.log` and `err.log`
//! a launched process is given. Errors are discarded.

// ########## LOGS ##########

use std::fs::{self, File};
use std::path::Path;
use std::process::Stdio;

use common::cartridge as contract;

use crate::catalog::Game;

/// Fresh stdout/stderr files for `game`, so a game that prints why it died
/// leaves that behind. Truncated per launch, so what is in them is always the
/// current run.
///
/// Falls back to discarding the output when the files cannot be opened — no
/// game goes unlaunched over a log.
pub fn gameOutput(base: &Path, game: &Game) -> (Stdio, Stdio) {
    let dir = base
        .join(contract::LOGS_DIR)
        .join(contract::slug(&game.name));
    if fs::create_dir_all(&dir).is_err() {
        return (Stdio::null(), Stdio::null());
    }
    // A closure because both handles want identical treatment, and `Stdio` is
    // not `Clone` so the two cannot come from one value.
    let open = |name: &str| {
        File::create(dir.join(name))
            .map(Stdio::from)
            .unwrap_or_else(|_| Stdio::null())
    };
    (open("out.log"), open("err.log"))
}
