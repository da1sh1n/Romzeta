// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

// Romzeta's build tool. Never shipped; nothing it depends on is linked into
// anything a user runs.
//
// It exists because a release is a four-stage sequence whose ordering
// constraint cargo cannot see (release.rs), and getting it wrong produces a
// cartridge that builds cleanly and is then refused by every listener.
//
//   keys.rs      where the signing key is, and which public keys a build trusts
//   manifest.rs  the shared-major version contract, checked before building
//   sign.rs      putting a signature into a binary, and reading one back
//   release.rs   the four stages, in order

#![allow(non_snake_case)] // camelCase functions

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use xtask::constants::{SHIPPED_CRATES, USAGE};
use xtask::{keys, manifest, release, report, sign};

// ########## THE COMMAND LINE ##########

fn main() -> ExitCode {
    let root = repoRoot();
    let mut args = std::env::args().skip(1);
    // `unwrap_or_default` turns "no arguments at all" into the empty string,
    // which the match below already treats as a request for the usage text.
    let command = args.next().unwrap_or_default();
    let rest: Vec<PathBuf> = args.map(PathBuf::from).collect();

    // Every arm hands back the same Result, so the exit code is decided once.
    let result = match command.as_str() {
        "release" => release::run(&root),
        "keygen" => keys::keygen(&root, rest.iter().any(|a| a == Path::new("--release"))),
        "sign" => signAll(&root, &rest),
        "verify" => verifyAll(&root, &rest),
        "version" => showVersions(&root),
        "test" => report::runAll(&root, rest.first().and_then(|arg| arg.to_str())),
        "" | "help" | "-h" | "--help" => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        other => Err(format!("unknown command `{other}`\n\n{USAGE}")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            // stderr, so `xtask verify … > list.txt` keeps the failure visible.
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

// ========== Commands ==========

/// Signs every path in `paths` in place, taking the role from each file's own
/// stem. Fails if the list is empty, the stem is not one xtask recognises, or
/// the signing key cannot be loaded.
fn signAll(root: &Path, paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("nothing to sign — pass one or more paths".to_string());
    }
    // Loaded once, outside the loop: an encrypted key would otherwise prompt
    // for its password per file.
    let key = keys::secretKey(root)?;
    for path in paths {
        let role = sign::roleForPath(path)?;
        sign::sign(path, &key, role)?;
        println!("signed {}", path.display());
    }
    Ok(())
}

/// Verifies every path in `paths` against the repo's public keys and each
/// file's own expected role, printing one line each. Fails if any of them was
/// refused.
fn verifyAll(root: &Path, paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("nothing to verify — pass one or more paths".to_string());
    }
    let anchors = keys::anchors(root);
    // Every path is reported before the first failure decides the exit code:
    // "which of these four is the bad one" is the actual question being asked.
    let mut failed = false;
    for path in paths {
        match sign::roleForPath(path).and_then(|role| sign::verify(path, &anchors, role)) {
            Ok(verified) => println!(
                "ok       {}  [{}]  {}",
                path.display(),
                verified.anchor,
                verified.comment
            ),
            Err(message) => {
                println!("REFUSED  {message}");
                failed = true;
            }
        }
    }
    if failed {
        Err("not everything verified".to_string())
    } else {
        Ok(())
    }
}

/// Prints the project version and every crate's, marking with `!` any shipped
/// crate whose major has drifted. Fails if one has, via the same check a
/// release runs.
fn showVersions(root: &Path) -> Result<(), String> {
    let (project_version, crates) = manifest::read(root)?;
    println!("project version {project_version} — the x in every x.y.z below");
    for c in &crates {
        let ok = if !SHIPPED_CRATES.contains(&c.name.as_str()) {
            " "
        } else {
            match common::version::parse(&c.version) {
                Some(v) if v.major == project_version => " ",
                _ => "!",
            }
        };
        // `{:<10}` left-pads the name so the versions form a column.
        println!("{ok} {:<10} {}", c.name, c.version);
    }
    // The list is printed first either way, so a drift report shows what drifted.
    manifest::check(root).map(|_| ())
}

/// The workspace root: this crate's own directory, one level up. Resolved from
/// `CARGO_MANIFEST_DIR` at compile time, so it holds wherever xtask is run from.
fn repoRoot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in the workspace root")
        .to_path_buf()
}
