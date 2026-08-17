// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

// Entry point. Parses `--pid <n> --base <dir>` off the command line and runs
// the keepalive loop against them. The whole binary is that one mode — the
// launcher spawns it detached, once, right after a game starts.

#![windows_subsystem = "windows"]
#![allow(non_snake_case)]

mod log;
mod playtime;
mod run;

#[cfg(windows)]
mod window;

// ########## ENTRY POINT ##########

const PID_FLAG: &str = "--pid"; // The PID of the game process to keep alive. REQUIRED
const BASE_FLAG: &str = "--base"; // The base directory of the game. REQUIRED
const PLAYTIME_FLAG: &str = "--playtime"; // The path to the playtime file, if any. Optional.

fn main() {
    common::aumid::set();

    let mut args = std::env::args_os().skip(1);
    let mut pid = None;
    let mut base_dir = None;
    let mut playtime_path = None;
    while let Some(arg) = args.next() {
        if arg == PID_FLAG {
            pid = args.next().and_then(|value| value.to_str()?.parse().ok());
        } else if arg == BASE_FLAG {
            base_dir = args.next().map(std::path::PathBuf::from);
        } else if arg == PLAYTIME_FLAG {
            playtime_path = args.next().map(std::path::PathBuf::from);
        }
    }

    let (Some(pid), Some(base_dir)) = (pid, base_dir) else {
        return;
    };

    // Windows only: a hidden window, purely so Task Manager and the taskbar
    // have something to hang the AUMID identity.
    #[cfg(windows)]
    window::runBehindHiddenWindow(base_dir, pid, playtime_path);
}
