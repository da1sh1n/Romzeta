// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! One game's cumulative playtime, ticked once a second. The flush on every
//! write is not bookkeeping — it is the keepalive: a read can be answered
//! from the OS cache forever, but a synced write has to reach the disk.

// ########## PER-GAME PLAYTIME ##########

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::constants::KEEPALIVE_INTERVAL_MS;

pub struct Counter {
    path: PathBuf,
    seconds: u32,
}

/// Opens the counter at `path`, picking up wherever a previous session left
/// off. A missing or unparseable file starts at zero rather than failing —
/// the first tick creates it.
pub fn open(path: PathBuf) -> Counter {
    let seconds = fs::read_to_string(&path)
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0);
    Counter { path, seconds }
}

impl Counter {
    pub fn tick(&mut self) {
        self.seconds += (KEEPALIVE_INTERVAL_MS / 1000) as u32; // adds Interval seconds
        if let Ok(mut file) = fs::File::create(&self.path)
        // open for write, truncating
        {
            let _ = file.write_all(self.seconds.to_string().as_bytes()); // write the new value
            let _ = file.sync_all(); // flush to disk, so a read can see it
        }
    }
}
