// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! The shared-major version contract, checked against the real manifests.

#![allow(non_snake_case)] // camelCase functions

// ########## THE VERSION CONTRACT ##########

mod common;

use std::fs;
use std::path::Path;

use common::{Scratch, checks, runTest, verdict};
use xtask::manifest::{check, read};

/// xtask/ -> the workspace root.
fn repoRoot() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent")
}

/// A throwaway workspace declaring `project_version` 0, with one crate per
/// `(name, version)`. Fixtures rather than the real tree, so a version the
/// checker must reject can be written down without Cargo refusing to parse it.
fn workspace(scratch: &Scratch, crates: &[(&str, &str)]) {
    let members = crates
        .iter()
        .map(|(name, _)| format!("    \"{name}\","))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        scratch.join("Cargo.toml"),
        format!(
            "[workspace]\nmembers = [\n{members}\n]\n\n\
             [workspace.metadata.romzeta]\nproject_version = 0\n"
        ),
    )
    .expect("write the workspace manifest");

    for (name, version) in crates {
        let dir = scratch.join(name);
        fs::create_dir_all(&dir).expect("create the member directory");
        fs::write(
            dir.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\n"),
        )
        .expect("write the member manifest");
    }
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
fn an_unparsable_version_in_a_shipped_crate_fails() {
    runTest(|| {
        // The build-time parser used to take the leading number and shrug at the
        // rest, so a crate versioned "0.2" passed here, got signed with "0.2" in
        // its trusted comment, and reached the listener's "no usable version"
        // arm — which starts it anyway.
        let scratch = Scratch::new("an_unparsable_version_in_a_shipped_crate_fails");
        workspace(&scratch, &[("launcher", "0.7.0"), ("keeper", "0.2.0-rc1")]);

        let mut proved = checks();
        match check(scratch.path()) {
            Ok(_) => proved.expect(false, "a version that is not x.y.z is refused"),
            Err(why) => {
                proved.expect(why.contains("keeper"), "the refusal names the crate");
                proved.expect(
                    why.contains("0.2.0-rc1"),
                    "and quotes the version it could not parse",
                );
            }
        }
        proved.verdict()
    });
}

#[test]
fn a_helper_crate_is_not_held_to_the_project_major() {
    runTest(|| {
        // `testkit` and `xtask` ship in nothing and are compared to nothing at
        // runtime. Holding them to the project major would mean bumping them
        // for no reason every time a generation changes.
        let scratch = Scratch::new("a_helper_crate_is_not_held_to_the_project_major");
        workspace(&scratch, &[("launcher", "0.7.0"), ("testkit", "9.9.9")]);

        let mut proved = checks();
        match check(scratch.path()) {
            Ok(version) => proved.expect(version == 0, "the shipped crate alone decides"),
            Err(why) => proved.expect(false, &format!("testkit was held to the gate: {why}")),
        }
        proved.verdict()
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
