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

use crate::playtime;

/// Disk-touch cadence in milliseconds.
pub(crate) const KEEPALIVE_INTERVAL_MS: u64 = 10_000;

/// Process-liveness check cadence in milliseconds.
const PROCESS_CHECK_INTERVAL_MS: u64 = 30_000;

/// Rewrite `logs/keeper.log` from scratch once it passes this size. Same
/// reasoning as the launcher's own log: a troubleshooting trail, not an audit
/// record.
const MAX_LOG_BYTES: u64 = 1024 * 1024;

pub fn run(base_dir: &Path, pid: u32, playtime_path: Option<PathBuf>) {
    let mut counter = playtime_path.map(playtime::open);

    if let Err(error) = common::lease::writeLease(pid, base_dir) {
        log(
            base_dir,
            &format!("keeper failed to write global lease for pid {pid}: {error}"),
        );
    }

    log(
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
        if Instant::now() >= next_process_check {
            if !common::lease::processExists(pid) {
                break;
            }
            next_process_check = Instant::now() + process_check_tick;
        }

        keepAlive(base_dir, &mut counter);
        thread::sleep(keepalive_tick);
    }

    if let Err(error) = common::lease::clearLease() {
        log(base_dir, &format!("keeper failed to clear global lease: {error}"));
    }
    log(base_dir, &format!("keeper stopped for pid {pid}"));
}

/// Ticks the playtime counter when there is one — the flushed write is the
/// keepalive. Falls back to listing the cartridge root otherwise (no game
/// identified, e.g. a hand-edited catalog, or a dev run with no `--playtime`).
fn keepAlive(base_dir: &Path, counter: &mut Option<playtime::Counter>) {
    match counter {
        Some(counter) => counter.tick(),
        None => {
            // A repeated read of the same file/offset is a page-cache hit
            // after the first tick, not a disk access — wake.rs hit this
            // already. Listing the root forces every tick down to the volume
            // instead of proving only that RAM still remembers last tick's
            // answer.
            if let Ok(mut entries) = fs::read_dir(base_dir) {
                let _ = entries.next();
            }
        }
    }
}

fn log(base_dir: &Path, message: &str) {
    common::log::appendLine(
        &base_dir.join("logs").join("keeper.log"),
        message,
        MAX_LOG_BYTES,
    );
}
