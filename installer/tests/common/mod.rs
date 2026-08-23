// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! This crate's end of `testkit`: the two things the harness cannot know for
//! itself. Not a test target — a bare `common.rs` here would be compiled as one.

// Each test binary in this directory compiles this module whole, so anything
// only one of them uses is dead code — or an unused re-export — in the others.
#![allow(dead_code, unused_imports)]

// ########## THIS CRATE'S TEST HARNESS ##########

pub use testkit::{Checks, Scratch, Verdict, appLogPath, checks, verdict};

/// The longest test name in this crate, setting the report's first column.
/// Holding the name rather than a width says which test decided it. A longer
/// one stops the run and asks for this to be widened, rather than quietly
/// pushing that row's columns out of line.
const LONGEST_TEST_NAME: &str = "the_app_id_file_is_rewritten_only_when_it_would_differ";

/// `CARGO_MANIFEST_DIR` has to be read here rather than inside `testkit`, where
/// it would name testkit's own folder instead of this crate's.
pub fn runTest<F: FnOnce() -> Verdict>(body: F) {
    testkit::runTest(env!("CARGO_MANIFEST_DIR"), LONGEST_TEST_NAME, body)
}
