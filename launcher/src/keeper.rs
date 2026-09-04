// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Starts the detached `keeper.exe` that babysits one launched game.

// ########## SPAWNING THE KEEPER ##########

use std::path::{Path, PathBuf};
use std::process::Command;

use common::cartridge as contract;

/// Launches `keeper.exe` — the sibling of whichever `launcher.exe` is
/// currently running, on a real cartridge or under `cargo run` alike — detached
/// and pointed at `pid`. Returns as soon as the process starts; the keeper then
/// runs independently of this one. `playtime_path`, when given, is where
/// keeper ticks this game's playtime counter — the write that also keeps the
/// cartridge from going idle.
pub fn spawn(base_dir: &Path, pid: u32, playtime_path: Option<&Path>) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let keeper_exe: PathBuf = exe.with_file_name(contract::KEEPER_NAME);

    let args = contract::KeeperArgs {
        pid,
        base_dir: base_dir.to_path_buf(),
        playtime_path: playtime_path.map(Path::to_path_buf),
    };

    let mut command = Command::new(keeper_exe);
    command.args(args.toArgv());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}
