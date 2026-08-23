// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Runs one test and records what it decided. Every crate's `tests/` folder
//! goes through here, so a run leaves the same report wherever it happened.
//!
//! ```text
//! runTest -> catch a panic -> compare -> a row in the category's table
//! ```

#![allow(non_snake_case)] // camelCase functions

// ########## RUNNING ONE TEST ##########

mod paths;
mod report;

use std::any::{Any, type_name};
use std::panic::{self, AssertUnwindSafe};
use std::sync::OnceLock;

pub use paths::{Scratch, appLogPath};

/// The crate whose tests are running, and the widest name its report allows.
/// Both are the caller's to know: `CARGO_MANIFEST_DIR` inside this crate would
/// point here, not at them.
pub(crate) static CRATE_DIR: OnceLock<String> = OnceLock::new();
pub(crate) static LONGEST_TEST_NAME: OnceLock<String> = OnceLock::new();

/// Which test binary is running, so its table can be named after it.
pub(crate) static SUITE: OnceLock<String> = OnceLock::new();

// ========== One Test's Verdict ==========

/// What a test found, beside what it wanted. Equal is a pass; anything else is
/// a failure in either direction — a case that should have been refused and was
/// accepted fails exactly as loudly as the reverse.
pub struct Verdict {
    pub(crate) result: String,
    pub(crate) expected: String,
}

pub fn verdict(result: impl Into<String>, expected: impl Into<String>) -> Verdict {
    Verdict {
        result: result.into(),
        expected: expected.into(),
    }
}

/// Runs one test body and records what it decided.
///
/// The body is a closure so its own type carries the names: `type_name` on it
/// gives `volume::a_volume_with_no_launcher_is_ignored::{{closure}}`, which is
/// the test binary and the test. Nothing is typed twice, and a rename cannot
/// leave a stale label behind.
///
/// A panic inside the body is caught, recorded and re-raised, so a test that
/// dies in its own setup still leaves an entry rather than a hole.
pub fn runTest<F: FnOnce() -> Verdict>(crate_dir: &str, longest: &str, body: F) {
    let _ = CRATE_DIR.set(crate_dir.to_owned());
    let _ = LONGEST_TEST_NAME.set(longest.to_owned());

    let (suite, test) = namesOf::<F>();
    let _ = SUITE.set(suite);
    paths::clearAppLog();

    match panic::catch_unwind(AssertUnwindSafe(body)) {
        Ok(v) if v.result == v.expected => report::record(&test, &v.result, "", true),
        Ok(v) => {
            report::record(&test, &v.result, &v.expected, false);
            panic!("{test}: result {}, expected {}", v.result, v.expected);
        }
        Err(payload) => {
            let message = panicMessage(payload.as_ref());
            report::record(
                &test,
                &format!("panicked: {message}"),
                "the test to reach a verdict",
                false,
            );
            panic::resume_unwind(payload);
        }
    }
}

/// The test binary's name and the test's, out of the body closure's type path.
///
/// `type_name`'s exact shape is documented as unspecified, so a path this
/// cannot split degrades to a usable label rather than breaking the suite.
fn namesOf<F>() -> (String, String) {
    let path = type_name::<F>();
    let mut parts = path.split("::");
    match (parts.next(), parts.next()) {
        (Some(suite), Some(test)) if !test.is_empty() && test != "{{closure}}" => {
            (suite.to_owned(), test.to_owned())
        }
        _ => ("tests".to_owned(), path.to_owned()),
    }
}

/// What a caught panic was about. The two shapes `panic!` produces, and a
/// stand-in for anything else that was thrown.
fn panicMessage(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_owned();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "a panic carrying no message".to_owned()
}

// ========== Several Checks, One Verdict ==========

/// For a test that asks more than one question. Each answer is recorded with
/// what it was checking, and the verdict names the first that came out wrong —
/// a count alone would say a row is red without saying why.
pub struct Checks {
    total: usize,
    failed: Option<String>,
}

pub fn checks() -> Checks {
    Checks {
        total: 0,
        failed: None,
    }
}

impl Checks {
    /// `describe` says what a passing check proves, so its negation reads as
    /// the failure: "0.2 is refused" becomes "0.2 was not refused".
    pub fn expect(&mut self, held: bool, describe: &str) {
        self.total += 1;
        if !held && self.failed.is_none() {
            self.failed = Some(describe.to_owned());
        }
    }

    pub fn verdict(self) -> Verdict {
        let expected = format!("all {} checks pass", self.total);
        match self.failed {
            Some(what) => verdict(format!("not true that {what}"), expected),
            None => verdict(expected.clone(), expected),
        }
    }
}
