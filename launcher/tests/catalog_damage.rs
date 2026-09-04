// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! What a broken or absent `catalog.json` costs. The README tells people to
//! edit that file by hand, and the launcher is a `windows_subsystem = "windows"`
//! binary built with `panic = "abort"` — so a panic here is a process that
//! vanishes without a window or a log line.

#![allow(non_snake_case)] // camelCase functions

// ########## A DAMAGED CATALOG ##########

mod common;

use std::fs;

use common::{Scratch, checks, runTest};
use launcher::catalog::load;
use launcher::constants::LOG_FILE;

const ONE_GAME: &str = r#"[
  { "name": "Baldur's Gate 3", "exe": "games/bg3/bg3.exe", "image": "assets/images/bg3.png" }
]"#;

/// The launcher log the code under test wrote inside the fixture, or empty if
/// it never made one.
fn logText(scratch: &Scratch) -> String {
    fs::read_to_string(scratch.join(LOG_FILE)).unwrap_or_default()
}

#[test]
fn malformed_json_leaves_an_empty_shelf() {
    runTest(|| {
        let scratch = Scratch::new("catalog-malformed");
        // A trailing comma: the single likeliest thing a hand edit leaves behind.
        fs::write(
            scratch.join("catalog.json"),
            "[{ \"name\": \"A\", \"exe\": \"games/a/a.exe\", \"image\": \"a.png\" },]",
        )
        .expect("write catalog");

        let games = load(scratch.path());
        let logged = logText(&scratch);

        let mut proved = checks();
        proved.expect(
            games.is_empty(),
            "the shelf is empty rather than the process",
        );
        proved.expect(
            logged.contains("UNPARSABLE"),
            "the log says the file could not be parsed",
        );
        proved.expect(
            logged.contains("catalog.json"),
            "and names the file it gave up on",
        );
        proved.verdict()
    });
}

#[test]
fn a_missing_catalog_leaves_an_empty_shelf() {
    runTest(|| {
        // Seeding is `content::ensureLayout`'s job and it can fail on a
        // write-protected stick, so `load` meets this case for real.
        let scratch = Scratch::new("catalog-missing");

        let games = load(scratch.path());
        let logged = logText(&scratch);

        let mut proved = checks();
        proved.expect(games.is_empty(), "no catalog means no games");
        proved.expect(
            logged.contains("UNREADABLE"),
            "the log says the file could not be read",
        );
        proved.expect(
            logged.contains("catalog.json"),
            "and names the file it looked for",
        );
        proved.verdict()
    });
}

#[test]
fn a_well_formed_catalog_still_loads() {
    runTest(|| {
        // The other half of the fix: an empty list has to mean damage, not the
        // new normal.
        let scratch = Scratch::new("catalog-intact");
        fs::write(scratch.join("catalog.json"), ONE_GAME).expect("write catalog");

        let games = load(scratch.path());

        let mut proved = checks();
        proved.expect(games.len() == 1, "the one entry survives");
        proved.expect(
            games
                .first()
                .is_some_and(|game| game.name == "Baldur's Gate 3"),
            "with the name the file gave it",
        );
        proved.expect(
            logText(&scratch).is_empty(),
            "and an intact catalog logs nothing",
        );
        proved.verdict()
    });
}
