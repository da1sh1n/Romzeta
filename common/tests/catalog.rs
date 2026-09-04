// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! `catalog.json` as the installer writes it, checked against the two rules the
//! launcher reads it by: serde's shape, and the containment filter that decides
//! which rows reach the shelf.

#![allow(non_snake_case)] // camelCase functions

// ########## THE CATALOG CONTRACT ##########

// Only the installer is linked here, never the launcher too. Both build scripts
// embed a VERSION resource into every target of their crate, so a test binary
// holding both libs fails to link with CVT1100 duplicate resource.
mod common;

use ::common::cartridge::{CATALOG_FILE, Entry, isContained};
use common::{Scratch, checks, runTest};

/// The rows a healthy installer write produces, covering both `steam` values
/// and a name whose slug is not its display name.
fn fixture() -> Vec<Entry> {
    vec![
        Entry {
            name: "Celeste".into(),
            exe: "games/celeste/celeste.exe".into(),
            image: "assets/images/celeste.png".into(),
            steam: false,
        },
        Entry {
            name: "Baldur's Gate 3: Definitive Edition".into(),
            exe: "games/baldur_s_gate_3/bin/bg3.exe".into(),
            image: "assets/images/baldur_s_gate_3.png".into(),
            steam: true,
        },
        Entry {
            name: "hollow knight".into(),
            exe: "games/hollow_knight/hollow_knight.exe".into(),
            image: "images/hollow_knight.png".into(),
            steam: false,
        },
    ]
}

#[test]
fn the_installer_writes_a_catalog_it_reads_back_unchanged() {
    runTest(|| {
        let want = fixture();
        let scratch = Scratch::new("the_installer_writes_a_catalog_it_reads_back_unchanged");
        let root = scratch.path();

        let mut proved = checks();
        match installer::catalog::write(root, &want) {
            Ok(()) => proved.expect(true, "the catalog writes"),
            Err(error) => proved.expect(false, &format!("the catalog writes: {error}")),
        }

        let got = match installer::catalog::read(root) {
            Ok(got) => got,
            Err(error) => {
                proved.expect(false, &format!("the catalog reads back: {error}"));
                return proved.verdict();
            }
        };

        // Checked before indexing: `read` answers a missing file with an empty
        // list, so a silent zero must fail here rather than skip the loop.
        proved.expect(
            got.len() == want.len(),
            &format!("all {} rows come back, not {}", want.len(), got.len()),
        );
        if got.len() != want.len() {
            return proved.verdict();
        }

        for (i, want) in want.iter().enumerate() {
            let got = &got[i];
            proved.expect(got.name == want.name, &format!("row {i} keeps its name"));
            proved.expect(got.exe == want.exe, &format!("row {i} keeps its exe"));
            proved.expect(got.image == want.image, &format!("row {i} keeps its image"));
            proved.expect(got.steam == want.steam, &format!("row {i} keeps its steam"));
        }
        proved.verdict()
    });
}

#[test]
fn steam_is_written_only_when_true_and_absent_means_false() {
    runTest(|| {
        let scratch = Scratch::new("steam_is_written_only_when_true_and_absent_means_false");
        let root = scratch.path();
        let mut proved = checks();

        match installer::catalog::write(root, &fixture()) {
            Ok(()) => {}
            Err(error) => {
                proved.expect(false, &format!("the catalog writes: {error}"));
                return proved.verdict();
            }
        }

        let json = match std::fs::read_to_string(root.join(CATALOG_FILE)) {
            Ok(json) => json,
            Err(error) => {
                proved.expect(false, &format!("the written file reads: {error}"));
                return proved.verdict();
            }
        };

        // `steam` carries `skip_serializing_if`, so a DRM-free cartridge gains
        // no key it never needed — and the launcher's `default` is what turns
        // that absence back into `false`.
        proved.expect(
            json.matches("\"steam\"").count() == 1,
            "only the one steam row writes the key",
        );
        proved.expect(json.contains("\"steam\": true"), "and it writes it as true");
        proved.expect(
            !json.contains("\"steam\": false"),
            "a false steam is never written",
        );

        for key in ["\"name\"", "\"exe\"", "\"image\""] {
            proved.expect(
                json.matches(key).count() == 3,
                &format!("every row writes {key}"),
            );
        }

        match installer::catalog::read(root) {
            Ok(got) if got.len() == 3 => {
                proved.expect(!got[0].steam, "the row with no steam key reads as false");
                proved.expect(got[1].steam, "the row that wrote true reads as true");
            }
            Ok(got) => proved.expect(false, &format!("3 rows come back, not {}", got.len())),
            Err(error) => proved.expect(false, &format!("the catalog reads back: {error}")),
        }
        proved.verdict()
    });
}

#[test]
fn every_row_the_installer_writes_survives_the_launchers_filter() {
    runTest(|| {
        // `launcher::catalog::load` drops any row failing this predicate on
        // either field. Linking the launcher here is what the duplicate VERSION
        // resource forbids, so the predicate itself is what gets pinned — a
        // writer that starts emitting a path it rejects fails this test.
        let mut proved = checks();
        for (i, entry) in fixture().iter().enumerate() {
            proved.expect(
                isContained(&entry.exe),
                &format!("row {i}'s exe reaches the shelf"),
            );
            proved.expect(
                isContained(&entry.image),
                &format!("row {i}'s image reaches the shelf"),
            );
        }

        // The same predicate, shown refusing what it exists to refuse — without
        // this the checks above would pass against an `isContained` stuck true.
        proved.expect(
            !isContained("../../Windows/System32/cmd.exe"),
            "an escaping exe is still refused",
        );
        proved.expect(!isContained(""), "an empty exe is still refused");
        proved.verdict()
    });
}
