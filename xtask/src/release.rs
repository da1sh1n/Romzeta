// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Runs the release sequence: build launcher, listener and keeper, sign them,
//! build the installer around the signed launcher and listener, sign it, then
//! verify all four.

// ########## THE RELEASE SEQUENCE ##########

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::constants::SHIPPED_CRATES;
use crate::{keys, manifest, sign};

/// Runs all four stages against the workspace at `root`, then verifies what it
/// just signed. Fails at the first stage that does, leaving nothing half-signed
/// beyond that point.
pub fn run(root: &Path) -> Result<(), String> {
    // Before anything is built: a set of programs that disagree about their own
    // compatibility generation is not a release, and finding that out after two
    // link steps helps nobody.
    let project_version = manifest::check(root)?;
    let (_, crates) = manifest::read(root)?;
    // A closure rather than a map: three lookups over six crates is not worth
    // building an index for, and this keeps `crates` the single source.
    let version = |name: &str| {
        crates
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.version.clone())
            .unwrap_or_default()
    };

    // Loaded before the first build: an encrypted key prompts, and a prompt
    // twenty minutes into a link step is a prompt nobody is sitting there for.
    let key = keys::secretKey(root)?;
    let release = root.join("target").join("release");

    println!("== building launcher, listener and keeper");
    cargo(
        root,
        &[
            "build",
            "--release",
            "-p",
            "launcher",
            "-p",
            "listener",
            "-p",
            "keeper",
        ],
    )?;

    println!("== signing them");
    let launcher = release.join(exe("launcher"));
    let listener = release.join(exe("listener"));
    let keeper = release.join(exe("keeper"));
    sign::sign(
        &launcher,
        &key,
        &trust::comment::build(trust::constants::LAUNCHER_ROLE, &version("launcher")),
    )?;
    sign::sign(
        &listener,
        &key,
        &trust::comment::build(trust::constants::LISTENER_ROLE, &version("listener")),
    )?;
    sign::sign(
        &keeper,
        &key,
        &trust::comment::build(trust::constants::KEEPER_ROLE, &version("keeper")),
    )?;

    // Deliberately after signing, and deliberately a separate cargo invocation:
    // in one `--workspace` build cargo is free to run the installer's build
    // script while launcher.exe is still linking, since no dependency edge
    // orders them.
    println!("== building the installer around them");
    cargo(root, &["build", "--release", "-p", "installer"])?;

    println!("== signing the installer");
    let installer = release.join(exe("installer"));
    sign::sign(
        &installer,
        &key,
        &trust::comment::build(trust::constants::INSTALLER_ROLE, &version("installer")),
    )?;

    println!();
    println!("project version {project_version} — these four are compatible with each other:");
    let anchors = keys::anchors(root);
    for &name in SHIPPED_CRATES {
        let path = release.join(exe(name));
        // Verifying what we just signed is not ceremony. It is the only thing
        // that proves the secret key in use actually corresponds to a public key
        // baked into the listener we just built — if it does not, every cartridge
        // from this release would be refused, and this is where we find out.
        let role = sign::roleForPath(&path)?;
        let verified = sign::verify(&path, &anchors, role)?;
        println!(
            "  {}  [{}]  {}",
            path.display(),
            verified.anchor,
            verified.comment
        );
    }

    if anchors.iter().all(|a| a.name != "dev") {
        println!();
        println!("Signed with the release key. Ship it.");
    } else {
        println!();
        println!(
            "Note: keys/dev.pub exists, so listeners built here also trust that key. \
             That is expected for local builds and must not be true of anything you publish."
        );
    }
    Ok(())
}

/// Runs cargo with `args` in `root`, inheriting stdio so its progress and
/// errors reach the terminal unchanged. Fails if cargo cannot be started or
/// exits non-zero.
fn cargo(root: &Path, args: &[&str]) -> Result<(), String> {
    // `CARGO` is set when we were started by cargo, which is the only supported
    // way in; the fallback is for someone running the binary directly.
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(&cargo)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|e| format!("could not run cargo: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("`cargo {}` failed", args.join(" ")))
    }
}

/// `stem` as a binary filename for the host platform — `.exe` on Windows, bare
/// everywhere else.
fn exe(stem: &str) -> PathBuf {
    // `cfg!` is a runtime bool the optimiser folds away, so both arms have to
    // type-check on both platforms — unlike `#[cfg]`, which deletes one.
    PathBuf::from(if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    })
}
