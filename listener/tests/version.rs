// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! This listener's own `x.y.z`, and the parser every Romzeta program
//! reads the others with.

#![allow(non_snake_case)] // camelCase functions

// ########## VERSIONS ##########

mod common;

// `::common` is the workspace crate; plain `common` is the module above.
use ::common::version::parse;
use common::{runTest, verdict};
use listener::version::own;

/// What `parse` made of `text`, for the report.
fn parsed(text: &str) -> String {
    match parse(text) {
        Some(version) => version.to_string(),
        None => "none".to_owned(),
    }
}

#[test]
fn parses_a_bare_version() {
    runTest(|| {
        let padded = match parse("  12.3.45  \r\n") {
            Some(version) => version.major.to_string(),
            None => "none".to_owned(),
        };
        verdict(
            format!("{}, major {padded}", parsed("0.2.0")),
            "0.2.0, major 12",
        )
    });
}

#[test]
fn refuses_anything_that_is_not_three_numbers() {
    runTest(|| {
        // The shapes a well-meaning change might introduce. Each would be a
        // guess about what the signature meant, so each is refused. The first is
        // the one that matters now: `parse` is fed the *version field* of a
        // trusted comment, never the whole comment.
        let accepted: Vec<String> = [
            "romzeta-launcher 0.2.0",
            "0.2",
            "0.2.0.1",
            "0.2.0-rc1",
            "v0.2.0",
            "",
            "not a version",
        ]
        .iter()
        .filter_map(|text| parse(text).map(|version| format!("{text:?} parsed as {version}")))
        .collect();

        let result = if accepted.is_empty() {
            "none parse".to_owned()
        } else {
            accepted.join("; ")
        };
        verdict(result, "none parse")
    });
}

#[test]
fn our_own_version_parses() {
    runTest(|| {
        // If this fails, `--version` is printing something no other Romzeta
        // program could read back.
        let own = own().to_string();
        verdict(parsed(&own), own.clone())
    });
}

#[test]
fn the_version_field_of_a_signed_comment_parses() {
    runTest(|| {
        // The shape `xtask sign` writes, split by `trust::attest` into role and
        // version. This is the contract between the two crates: if xtask's
        // comment format ever changes, this is what should notice.
        let comment = "romzeta-launcher 0.2.1 2026-07-30";
        let mut parts = comment.split_whitespace();
        let role = parts.next().unwrap_or("none");
        let minor = match parts.next().and_then(parse) {
            Some(version) => version.minor.to_string(),
            None => "none".to_owned(),
        };
        verdict(
            format!("{role}, minor {minor}"),
            format!("{}, minor 2", ::trust::constants::LAUNCHER_ROLE),
        )
    });
}
