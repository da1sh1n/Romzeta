// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Whether a path out of the catalog stays on the cartridge it came from.

#![allow(non_snake_case)] // camelCase functions

// ########## CONTAINMENT ##########

mod common;

use ::common::cartridge::isContained;
use common::{checks, runTest};

#[test]
fn ordinary_relative_paths_stay_inside() {
    runTest(|| {
        let mut proved = checks();
        proved.expect(isContained("games/bg3/bg3.exe"), "a game path is contained");
        proved.expect(isContained("images/bg3.png"), "so is an image path");
        proved.expect(isContained("./games/bg3/bg3.exe"), "and a leading ./");
        proved.verdict()
    });
}

#[test]
fn a_drive_letter_escapes() {
    runTest(|| {
        // `Path::join` with this discards the base entirely — the whole reason
        // this check exists rather than trusting `join` to contain it.
        let mut proved = checks();
        proved.expect(
            !isContained(r"C:\Windows\System32\cmd.exe"),
            "a backslash drive path escapes",
        );
        proved.expect(
            !isContained("C:/Windows/System32/cmd.exe"),
            "and a forward-slash one",
        );
        proved.verdict()
    });
}

#[test]
fn a_unc_path_escapes() {
    runTest(|| {
        let mut proved = checks();
        proved.expect(
            !isContained(r"\attacker.example\share\payload.exe"),
            "a UNC share escapes",
        );
        proved.verdict()
    });
}

#[test]
fn a_leading_root_escapes() {
    runTest(|| {
        let mut proved = checks();
        proved.expect(!isContained("/etc/passwd"), "an absolute path escapes");
        proved.verdict()
    });
}

#[test]
fn a_parent_dir_component_escapes() {
    runTest(|| {
        let mut proved = checks();
        proved.expect(!isContained("../../evil.exe"), "a leading .. escapes");
        proved.expect(
            !isContained("games/../../evil.exe"),
            "so does one part way in",
        );
        // Buried in an otherwise-ordinary-looking path — the shape most likely
        // to slip past a reviewer's eye rather than a machine's.
        proved.expect(
            !isContained("games/bg3/../../../evil.exe"),
            "and one buried deep",
        );
        proved.verdict()
    });
}
