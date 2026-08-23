// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Making sure a drive is really awake before gigabytes are written to it.

#![allow(non_snake_case)] // camelCase functions

// ########## WAKING A DRIVE ##########

mod common;

use std::fs;

use common::{Scratch, checks, runTest};
use installer::constants::PROBES;
use installer::wake::probe;

/// Three reads and not one. The first can be answered from the OS cache while
/// the disk underneath is still spinning up, so a probe count of one would pass
/// on a drive that is not actually awake — which is the whole thing this gate
/// exists to catch.
#[test]
fn a_drive_that_answers_is_read_three_times() {
    runTest(|| {
        let dir = Scratch::new("wake-awake");
        let mut reports = 0u64;
        let mut last_done = 0u64;
        let mut every_total_right = true;

        probe(dir.path(), &mut |progress| {
            reports += 1;
            last_done = progress.done;
            every_total_right &= progress.total == PROBES;
        })
        .expect("a directory that exists must answer");

        let mut proved = checks();
        proved.expect(every_total_right, "every report names the same total");
        proved.expect(
            reports == PROBES + 1,
            "one report per round, plus the finish",
        );
        proved.expect(last_done == PROBES, "the bar ends full");
        proved.verdict()
    });
}

/// The unplugged drive, and the reason the failure has to name the path: the
/// message is the only thing the user gets back on the Review screen, and "it
/// did not work" would leave them nothing to act on.
#[test]
fn a_drive_that_is_not_there_fails_on_the_first_round() {
    runTest(|| {
        let dir = Scratch::new("wake-gone");
        let gone = dir.join("removed");
        // Never created, so it is a path inside a folder that does exist.

        let problem = probe(&gone, &mut |_| {}).expect_err("a missing path cannot answer");
        let mut proved = checks();
        proved.expect(
            problem.contains(&gone.display().to_string()),
            "the message names the path",
        );
        proved.expect(
            problem.contains(&format!("probe 1 of {PROBES}")),
            "and says it failed on the first round",
        );
        proved.verdict()
    });
}

/// A drive letter picked up by something that is not a volume root. It answers
/// `metadata` perfectly well, which is why the kind is checked and not just the
/// existence.
#[test]
fn a_path_that_is_not_a_directory_is_refused() {
    runTest(|| {
        let dir = Scratch::new("wake-file");
        let file = dir.join("not-a-drive-root");
        fs::write(&file, b"").expect("write");

        let problem = probe(&file, &mut |_| {}).expect_err("a file is not a cartridge");
        let mut proved = checks();
        proved.expect(
            problem.contains("not a directory"),
            "the message says it is not a directory",
        );
        proved.verdict()
    });
}
