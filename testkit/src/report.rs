// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! One row per test, written twice: as Markdown into this category's table, and
//! with ANSI colour onto the console. `xtask test` merges the category tables
//! into the crate's dated log once the whole run has finished.

// ########## THE REPORT ##########

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Once;

use crate::LONGEST_TEST_NAME;
use crate::paths::{suite, testTree};

/// Widest a value column gets before its row starts pushing the table out of
/// line. Long enough for a folded checklist verdict.
const LONGEST_VALUE: &str = "reader in, writer out, deleter out";

/// Green and red that stay readable on a light background and a dark one.
const PASSED_COLOUR: &str = "#3fb950";
const FAILED_COLOUR: &str = "#e5534b";

/// Starts this test binary's table, once.
static RUN_BEGUN: Once = Once::new();

/// Records one test in this category's table and on the console, in a single
/// write each, so entries never interleave half-written.
///
/// `expected` is empty on a pass: it equals `result` there by definition, so
/// only the failures carry two values and they are the rows that stand out.
///
/// A report that cannot be written is worth failing over — it is the thing
/// being asked for.
pub(crate) fn record(test: &str, result: &str, expected: &str, passed: bool) {
    assert!(
        test.len() <= names(),
        "{test} is longer than this crate's LONGEST_TEST_NAME; widen it in tests/common/mod.rs"
    );
    beginRun();

    let (names, values) = (names(), LONGEST_VALUE.len());
    let rest = format!("{test:<names$} | {result:<values$} | {expected:<values$} |");

    appendReport(&format!("| {} | {rest}\n", verdictCell(passed)));
    toConsole(&format!("| {} | {rest}\n", verdictAnsi(passed)));
}

/// The verdict as the report carries it. Both cells come out the same length —
/// the two colours are seven characters and the two words six — so the column
/// needs no padding and the raw table stays rectangular with it in front.
/// Changing either word or either colour to a different length breaks that.
fn verdictCell(passed: bool) -> String {
    let (word, colour) = verdictOf(passed);
    format!("<span style=\"color:{colour}\">{word}</span>")
}

/// The same for the console, where colour is an escape sequence. Its length
/// does not matter: escapes take no columns on screen, so what lines up is the
/// word, and both words are the same width.
fn verdictAnsi(passed: bool) -> String {
    let (word, _) = verdictOf(passed);
    let ansi = if passed { "\x1b[32m" } else { "\x1b[1;31m" };
    format!("{ansi}{word:<7}\x1b[0m")
}

fn verdictOf(passed: bool) -> (&'static str, &'static str) {
    if passed {
        ("passed", PASSED_COLOUR)
    } else {
        ("FAILED", FAILED_COLOUR)
    }
}

/// The width of the test-name column, set by the calling crate.
fn names() -> usize {
    LONGEST_TEST_NAME
        .get()
        .expect("testkit used outside runTest: the column width is not known yet")
        .len()
}

// ========== Starting A Category's Table ==========

/// Starts this category's report, once, replacing what the last run left.
///
/// One file per test binary, holding that category's table and nothing else.
/// `xtask test` merges them into the crate's dated log when the whole suite has
/// finished — the only moment anything knows the run is over, since cargo hands
/// out no such signal and the last binary cannot tell that it is last.
fn beginRun() {
    RUN_BEGUN.call_once(|| {
        fs::create_dir_all(testTree("logs")).expect("could not create the test report folder");

        let (names, values) = (names(), LONGEST_VALUE.len());
        let verdicts = verdictCell(true).len();
        let columns = format!(
            "{:<names$} | {:<values$} | {:<values$} |",
            "Test", "Result", "Expected"
        );
        let rules = format!(
            "{}|{}|{}|",
            "-".repeat(names + 2),
            "-".repeat(values + 2),
            "-".repeat(values + 2),
        );

        // In the report the verdict column is as wide as the cells below it —
        // markup and all — so the raw table is rectangular from the first `|`.
        // On screen those cells are six characters, escapes taking no columns.
        let report = format!(
            "| {:<verdicts$} | {columns}\n|{}|{rules}\n",
            "Verdict",
            "-".repeat(verdicts + 2)
        );
        let console = format!("| Verdict | {columns}\n|---------|{rules}\n");

        let time = &common::time::timestamp()[11..];
        fs::write(reportPath(), format!("# {} — {time}\n\n{report}", suite()))
            .expect("could not start this category's report");
        toConsole(&format!("\n{time} — {}\n{console}", suite()));
    });
}

/// This category's own report. Named after the test binary, so the categories
/// never collide and `xtask test` can tell which rows came from where.
fn reportPath() -> PathBuf {
    testTree("logs").join(format!("{}.md", suite()))
}

fn appendReport(text: &str) {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(reportPath())
        .expect("could not open the test report");
    file.write_all(text.as_bytes())
        .expect("could not write the test report entry");
}

/// Writes straight to the handle rather than through `println!`, which is the
/// path libtest captures and replays only for tests that failed. This is what
/// puts the table in front of you on a plain `cargo test`.
fn toConsole(text: &str) {
    let mut out = io::stdout().lock();
    let _ = out.write_all(text.as_bytes());
    let _ = out.flush();
}
