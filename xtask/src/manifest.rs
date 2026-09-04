// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Reads the workspace and member manifests, and checks each shipped crate's
//! major against `project_version`.

// ########## THE VERSION CONTRACT ##########

use std::fs;
use std::path::Path;

use crate::constants::SHIPPED_CRATES;

/// One workspace member's name and version, as its own manifest states them.
pub struct Crate {
    pub name: String,
    pub version: String,
}

/// Reads the project version from `[workspace.metadata.romzeta]` and every
/// member's own version. Fails with a sentence naming the manifest at fault.
pub fn read(root: &Path) -> Result<(u64, Vec<Crate>), String> {
    let text = readManifest(&root.join("Cargo.toml"))?;
    let table: toml::Table = text
        .parse()
        .map_err(|e| format!("the workspace Cargo.toml is not valid TOML: {e}"))?;

    let workspace = table
        .get("workspace")
        .and_then(|v| v.as_table())
        // `ok_or` on an Option turns "key missing" into the error `?` wants.
        .ok_or("the root Cargo.toml has no [workspace]")?;

    let project_version = workspace
        .get("metadata")
        .and_then(|v| v.get("romzeta"))
        .and_then(|v| v.get("project_version"))
        .and_then(|v| v.as_integer())
        .ok_or(
            "the root Cargo.toml has no [workspace.metadata.romzeta] project_version — \
             it is the declared truth every crate's major has to match",
        )?;

    let members = workspace
        .get("members")
        .and_then(|v| v.as_array())
        .ok_or("the root Cargo.toml lists no workspace members")?;

    let crates = members
        .iter()
        .filter_map(|m| m.as_str())
        .map(|member| {
            let text = readManifest(&root.join(member).join("Cargo.toml"))?;
            let table: toml::Table = text
                .parse()
                .map_err(|e| format!("{member}/Cargo.toml is not valid TOML: {e}"))?;
            let package = table
                .get("package")
                .ok_or_else(|| format!("{member}/Cargo.toml has no [package]"))?;
            Ok(Crate {
                // A package may be named something other than its folder;
                // the folder name is the fallback, not the answer.
                name: package
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(member)
                    .to_string(),
                version: package
                    .get("version")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| format!("{member}/Cargo.toml has no version"))?
                    .to_string(),
            })
        })
        // Collecting into a Result stops at the first bad manifest and hands
        // back its error instead of a Vec of Results.
        .collect::<Result<Vec<_>, String>>()?;

    Ok((project_version as u64, crates))
}

/// Fails when a shipped crate's major differs from the declared project
/// version — `SHIPPED_CRATES` only; a helper crate like `testkit` never ships
/// and is not held to it. Returns the project version when everything shipped
/// agrees with it.
///
/// Reports *every* mismatch rather than the first, because the fix is usually
/// "these three are behind" and finding that out one rebuild at a time wastes
/// a release.
pub fn check(root: &Path) -> Result<u64, String> {
    let (project_version, crates) = read(root)?;

    let mut drifted = Vec::new();
    for c in crates
        .iter()
        .filter(|c| SHIPPED_CRATES.contains(&c.name.as_str()))
    {
        match common::version::parse(&c.version) {
            Some(v) if v.major == project_version => {}
            Some(_) => drifted.push(format!(
                "  {} is {} — expected {project_version}.y.z",
                c.name, c.version
            )),
            None => drifted.push(format!(
                "  {} has a version xtask cannot parse: {:?}",
                c.name, c.version
            )),
        }
    }

    if !drifted.is_empty() {
        return Err(format!(
            "these crates disagree with the project version ({project_version}):\n{}\n\
             Either bump them to {project_version}.y.z, or bump project_version in the root \
             Cargo.toml if this really is a new compatibility generation.",
            drifted.join("\n")
        ));
    }
    Ok(project_version)
}

/// Reads a manifest, turning an IO error into a message naming the file.
fn readManifest(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("could not read {}: {e}", path.display()))
}
