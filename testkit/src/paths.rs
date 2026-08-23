// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Everywhere a test is allowed to write, all of it inside the crate under
//! test: `tests/env/` for what a test builds, `tests/logs/` for what it reports.

// ########## WHERE A TEST MAY WRITE ##########

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Once;

use crate::{CRATE_DIR, SUITE};

/// Clears the app log once per test binary, not once per test.
static APP_LOG_CLEARED: Once = Once::new();

/// A folder inside the crate under test's `tests/`, created on demand.
///
/// Everything a test writes goes under here rather than the system temp folder:
/// a test's leavings belong to the project that made them, and a run that dies
/// mid-test leaves them somewhere you will look.
pub(crate) fn testTree(sub: &str) -> PathBuf {
    let root = CRATE_DIR
        .get()
        .expect("testkit used outside runTest: the crate directory is not known yet");
    Path::new(root).join("tests").join(sub)
}

/// Which test binary is writing, for naming its files after itself.
pub(crate) fn suite() -> &'static str {
    SUITE.get().map_or("tests", String::as_str)
}

// ========== The Log The Program Writes ==========

/// Where the code under test should send its own log:
/// `tests/env/tests-<suite>.log`.
///
/// A real log rather than a discarded one, so the logging path is exercised
/// too. One file per test binary because cargo runs them one after another and
/// each clears its own at the start — a shared file could not be cleared
/// without one binary erasing another's lines. Opening it is the caller's job,
/// since every crate has its own log type.
pub fn appLogPath() -> PathBuf {
    testTree("env").join(format!("tests-{}.log", suite()))
}

/// Empties the app log before the first test in this binary writes to it, so it
/// holds one run rather than a pile of them.
pub(crate) fn clearAppLog() {
    APP_LOG_CLEARED.call_once(|| {
        let path = appLogPath();
        if path.exists() {
            fs::remove_file(&path).expect("could not clear the app log from the last run");
        }
    });
}

// ========== A Fixture's Directory ==========

/// One fixture's directory under `tests/env/`, named after the test that asked
/// for it.
///
/// Emptied when the test starts, not when it ends: what a failing test built is
/// the first thing worth looking at, and a run that wipes up after itself leaves
/// you re-reading the assertion instead of the fixture. The next run of the same
/// test clears it, so nothing accumulates.
///
/// The name has to be unique across the crate's whole suite — cargo runs the
/// test binaries in sequence but the tests inside one in parallel, and two
/// sharing a name would each clear the other's fixture out from under it.
pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new(name: &str) -> Scratch {
        let dir = testTree("env").join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("could not create the fixture directory");
        Scratch(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
