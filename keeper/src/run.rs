// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! The keepalive loop: watches one pid, keeps the cartridge disk from
//! sleeping while it runs, and clears the shared lease once it's gone.

// ########## GAME KEEPALIVE LOOP ##########

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use crate::constants::{KEEPALIVE_INTERVAL_MS, PROCESS_CHECK_INTERVAL_MS};
use crate::log::logLine;
use crate::playtime;

pub fn run(base_dir: &Path, pid: u32, playtime_path: Option<PathBuf>) {
    let mut counter = playtime_path.map(playtime::open);

    if let Err(error) = common::lease::writeLease(pid, base_dir) {
        logLine(
            base_dir,
            &format!("keeper failed to write global lease for pid {pid}: {error}"),
        );
    }

    logLine(
        base_dir,
        &format!(
            "keeper started for pid {pid} (keepalive={}ms, check={}ms)",
            KEEPALIVE_INTERVAL_MS, PROCESS_CHECK_INTERVAL_MS
        ),
    );

    let keepalive_tick = Duration::from_millis(KEEPALIVE_INTERVAL_MS);
    let process_check_tick = Duration::from_millis(PROCESS_CHECK_INTERVAL_MS);
    let mut next_process_check = Instant::now();

    loop {
        if Instant::now() >= next_process_check && !common::lease::processExists(pid) {
            break;
        }

        keepAlive(base_dir, &mut counter);
        thread::sleep(keepalive_tick);
        next_process_check = next_process_check + process_check_tick;
    }

    if let Err(error) = common::lease::clearLease() {
        logLine(
            base_dir,
            &format!("keeper failed to clear global lease: {error}"),
        );
    }
    logLine(base_dir, &format!("keeper stopped for pid {pid}"));
}

/// Keepalive: ticks the playtime counter if there is one, else lists the
/// cartridge root (no game identified, or a dev run with no `--playtime`).
fn keepAlive(base_dir: &Path, counter: &mut Option<playtime::Counter>) {
    match counter {
        Some(counter) => counter.tick(),
        None => {
            // Must reach the volume each tick, not just the page cache.
            if let Ok(mut entries) = fs::read_dir(base_dir) {
                let _ = entries.next();
            }
        }
    }
}
