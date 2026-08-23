// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! The shared-major version contract, checked against the real manifests.

#![allow(non_snake_case)] // camelCase functions

// ########## THE VERSION CONTRACT ##########

mod common;

use std::path::Path;

use common::{checks, runTest, verdict};
use xtask::manifest::{check, major, read};

/// xtask/ -> the workspace root.
fn repoRoot() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent")
}

#[test]
fn this_workspace_agrees_with_itself() {
    runTest(|| {
        // The whole point, asserted against the real manifests: if someone bumps
        // one crate's major and not the others, this fails in `cargo test`
        // rather than at release time.
        let agree = "the majors agree";
        match check(repoRoot()) {
            Ok(_) => verdict(agree, agree),
            Err(why) => verdict(why, agree),
        }
    });
}

#[test]
fn every_crate_is_accounted_for() {
    runTest(|| {
        let (_, crates) = read(repoRoot()).expect("read the manifests");
        let names: Vec<&str> = crates.iter().map(|c| c.name.as_str()).collect();

        let mut proved = checks();
        for expected in [
            "launcher",
            "listener",
            "installer",
            "common",
            "sigblock",
            "xtask",
        ] {
            proved.expect(names.contains(&expected), expected);
        }
        proved.verdict()
    });
}

#[test]
fn reads_the_leading_number() {
    runTest(|| {
        let mut proved = checks();
        proved.expect(major("0.2.0") == Some(0), "0.2.0 is major 0");
        proved.expect(major("12.0.1") == Some(12), "12.0.1 is major 12");
        proved.expect(major(" 1.0.0 ") == Some(1), "surrounding space is ignored");
        proved.expect(major("not-a-version").is_none(), "a word has no major");
        proved.expect(major("").is_none(), "nor does an empty string");
        proved.verdict()
    });
}
