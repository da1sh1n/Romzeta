// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

// Entry point. Parses `--pid <n> --base <dir>` off the command line and runs
// the keepalive loop against them. The whole binary is that one mode — the
// launcher spawns it detached, once, right after a game starts.

#![windows_subsystem = "windows"]
#![allow(non_snake_case)] // camelCase functions

mod constants;
mod playtime;
mod run;

#[cfg(windows)]
mod window;

// ########## ENTRY POINT ##########

fn main() {
    common::aumid::set();

    let Some(args) = common::cartridge::parseKeeperArgs(std::env::args_os().skip(1)) else {
        return;
    };

    // Windows only: a hidden window, purely so Task Manager and the taskbar
    // have something to hang the AUMID identity.
    #[cfg(windows)]
    window::runBehindHiddenWindow(args.base_dir, args.pid, args.playtime_path);
}
