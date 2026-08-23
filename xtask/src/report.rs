// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Runs a crate's tests, then merges the per-category tables they leave in
//! `tests/logs/` into one table and appends it to today's log.
//!
//! This exists because nothing runs after the last test binary: cargo hands out
//! no end-of-run signal, and a binary cannot tell that it is the last one. The
//! merge has to happen from outside the run.

// ########## RUNNING A SUITE AND MERGING ITS REPORT ##########

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::constants::{CATEGORY_COLUMN, TEST_LOGS};
use crate::manifest;

/// Runs the tests of every workspace member that has a `tests/` folder, or of
/// just `only` when one is named. Each crate keeps its own report.
pub fn runAll(root: &Path, only: Option<&str>) -> Result<(), String> {
    let packages = testable(root, only)?;
    if packages.is_empty() {
        return match only {
            Some(name) => Err(format!("{name} has no tests/ folder")),
            None => Err("no crate in this workspace has a tests/ folder".to_owned()),
        };
    }

    // Every crate is run before any failure is reported: stopping at the first
    // would leave the crates after it without a report, which is the opposite
    // of what a failing run needs.
    let mut failed = Vec::new();
    for package in &packages {
        if run(root, package).is_err() {
            failed.push(package.as_str());
        }
    }
    match failed.len() {
        0 => Ok(()),
        _ => Err(format!("tests failed in {}", failed.join(", "))),
    }
}

/// The members worth running: those with a `tests/` folder, in manifest order.
fn testable(root: &Path, only: Option<&str>) -> Result<Vec<String>, String> {
    let (_, crates) = manifest::read(root)?;
    Ok(crates
        .into_iter()
        .map(|member| member.name)
        .filter(|name| only.is_none_or(|wanted| wanted == name))
        .filter(|name| root.join(name).join("tests").is_dir())
        .collect())
}

/// Runs one crate's tests and appends the merged table to its own log.
///
/// `--no-fail-fast` because cargo otherwise stops at the first test binary that
/// fails, and the categories after it would be missing from the report — the
/// run you most want a report for is the one that failed.
fn run(root: &Path, package: &str) -> Result<(), String> {
    let logs = root.join(package).join("tests").join(TEST_LOGS);
    clearCategories(&logs)?;

    let status = Command::new(env!("CARGO"))
        .current_dir(root)
        .args(["test", "-p", package, "--no-fail-fast"])
        .status()
        .map_err(|e| format!("could not run cargo test: {e}"))?;

    // Merged whether or not the tests passed, then the failure is reported.
    merge(&logs, package)?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{package}: some tests failed — see {}",
            today(&logs).display()
        ))
    }
}

// ========== The Merge ==========

/// Appends one table holding every category's rows, with the category last.
fn merge(logs: &Path, package: &str) -> Result<(), String> {
    let mut categories = categoryFiles(logs)?;
    categories.sort();

    let mut head: Option<(String, String)> = None;
    let mut rows: Vec<String> = Vec::new();
    let mut width = CATEGORY_COLUMN.len();

    for path in &categories {
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("{}: unreadable name", path.display()))?;
        width = width.max(name.len());

        let text = fs::read_to_string(path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        let mut lines = text.lines().filter(|line| line.starts_with('|'));

        // The first two table lines are the header and its rule. Every category
        // writes them at the same widths, so one set serves the merged table.
        let (Some(header), Some(rule)) = (lines.next(), lines.next()) else {
            continue; // a category that recorded nothing
        };
        head.get_or_insert_with(|| (header.to_owned(), rule.to_owned()));
        rows.extend(lines.map(|line| format!("{line} {name} |")));
    }

    let Some((header, rule)) = head else {
        return Err(format!(
            "no category reports in {} — did the tests run?",
            logs.display()
        ));
    };
    if rows.is_empty() {
        return Ok(());
    }

    // Re-pad the category cell now that the longest name is known.
    let padded: Vec<String> = rows.iter().map(|row| repad(row, width)).collect();

    let table = format!(
        "\n## {}\n\n{header} {CATEGORY_COLUMN:<width$} |\n{rule}{}|\n{}\n",
        &common::time::timestamp()[11..],
        "-".repeat(width + 2),
        padded.join("\n"),
    );
    appendToday(logs, package, &table)?;
    // Folded into today's log now, so the per-category copies would only be a
    // second, staler record of the same run sitting beside it. A plain
    // `cargo test` writes them again and leaves them; only a merge clears them.
    clearCategories(logs)
}

/// Widens a row's trailing category cell to `width`.
fn repad(row: &str, width: usize) -> String {
    match row.trim_end().strip_suffix('|') {
        Some(body) => match body.trim_end().rsplit_once('|') {
            Some((front, name)) => format!("{front}| {:<width$} |", name.trim()),
            None => row.to_owned(),
        },
        None => row.to_owned(),
    }
}

// ========== The Files ==========

/// Today's log, the one the merged tables accumulate in.
fn today(logs: &Path) -> PathBuf {
    logs.join(format!("{}.md", common::time::today()))
}

/// The per-category reports: every `.md` in the folder that is not a dated log.
fn categoryFiles(logs: &Path) -> Result<Vec<PathBuf>, String> {
    let Ok(entries) = fs::read_dir(logs) else {
        return Ok(Vec::new()); // no folder yet: the run below will make one
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "md") && !isDated(&path) {
            found.push(path);
        }
    }
    Ok(found)
}

/// Whether a name is `YYYY-MM-DD.md` rather than a category's.
fn isDated(path: &Path) -> bool {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| {
            stem.len() == 10
                && stem.chars().enumerate().all(|(i, c)| {
                    if i == 4 || i == 7 {
                        c == '-'
                    } else {
                        c.is_ascii_digit()
                    }
                })
        })
}

/// Removes the per-category reports. Run before the suite, so a category that
/// was deleted or renamed cannot leave a stale table to be merged into this
/// run's, and again after the merge, once today's log holds their rows.
fn clearCategories(logs: &Path) -> Result<(), String> {
    for path in categoryFiles(logs)? {
        fs::remove_file(&path).map_err(|e| format!("could not clear {}: {e}", path.display()))?;
    }
    Ok(())
}

/// Appends to today's log, never replacing it: the day's runs accumulate, each
/// under its own time.
fn appendToday(logs: &Path, package: &str, table: &str) -> Result<(), String> {
    let path = today(logs);
    let mut text = fs::read_to_string(&path).unwrap_or_default();
    if text.is_empty() {
        text.push_str(&format!("# {package} — test report\n"));
    }
    text.push_str(table);
    fs::write(&path, text).map_err(|e| format!("could not write {}: {e}", path.display()))
}
