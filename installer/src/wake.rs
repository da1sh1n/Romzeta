// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Reads the drive a few times before anything is written to it. A cartridge
//! that has spun down, or a letter that went stale while a plan was being
//! assembled, is found here rather than halfway through a multi-gigabyte copy.

// ########## WAKING THE CARTRIDGE ##########

use std::fs;
use std::path::Path;
use std::thread;

use crate::cartridge::Progress;
use crate::constants::{PROBE_GAP, PROBES};

/// Reads `root` [`PROBES`] times, stopping at the first round it cannot.
///
/// Nothing is written. A probe file would fail on a cartridge that is awake but
/// full or mounted read-only — reasons that have nothing to do with the
/// question being asked.
pub fn probe(root: &Path, report: &mut dyn FnMut(Progress)) -> Result<(), String> {
    for round in 1..=PROBES {
        report(Progress {
            done: round - 1,
            total: PROBES,
            label: format!("Waking the cartridge — {round} of {PROBES}"),
        });
        answer(root).map_err(|reason| {
            format!(
                "{} did not answer on probe {round} of {PROBES}: {reason}. The drive may be \
                 asleep, unplugged, or no longer the one on that letter.",
                root.display()
            )
        })?;
        if round < PROBES {
            thread::sleep(PROBE_GAP);
        }
    }
    report(Progress {
        done: PROBES,
        total: PROBES,
        label: "The cartridge is awake".into(),
    });
    Ok(())
}

/// One round. `read_dir` and not `metadata` alone: an attribute lookup can come
/// off the cache, while listing the root has to reach the volume.
fn answer(root: &Path) -> Result<(), String> {
    let meta = fs::metadata(root).map_err(|e| e.to_string())?;
    if !meta.is_dir() {
        return Err("it is not a directory".into());
    }
    // An empty drive is a perfectly good answer; only a failure to read one is
    // not, which is why the entry itself is dropped and its error is not.
    match fs::read_dir(root).map_err(|e| e.to_string())?.next() {
        Some(Err(e)) => Err(e.to_string()),
        _ => Ok(()),
    }
}
