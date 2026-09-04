// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Build script. Reads `keys/romzeta.pub` and `keys/dev.pub` and writes them
//! into `OUT_DIR` as the `const ANCHORS` this crate compiles in. Both keys
//! absent is a build error.

// ########## BAKING IN THE TRUST ANCHORS ##########

use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR not set: build.rs was not run by cargo"),
    );
    let keys = manifest
        .parent()
        .expect("crate dir has no parent: cannot locate the workspace keys/")
        .join("keys");
    let out = PathBuf::from(
        std::env::var_os("OUT_DIR").expect("OUT_DIR not set: build.rs was not run by cargo"),
    );

    let mut anchors: Vec<(String, String)> = Vec::new();
    for &(name, file) in trust::keyfile::ANCHOR_FILES {
        let path = keys.join(file);
        // Rebuild when a key appears, changes or is deleted — including the
        // "appears" case, which is what happens right after `xtask keygen`.
        println!("cargo::rerun-if-changed={}", path.display());

        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        match trust::keyfile::keyLine(&text) {
            Some(key) => anchors.push((name.to_string(), key.to_string())),
            None => println!(
                "cargo::warning={} exists but holds no key line; ignoring it",
                path.display()
            ),
        }
    }

    if anchors.is_empty() {
        println!(
            "cargo::error=no trusted public key: neither {} nor {} exists.",
            keys.join("romzeta.pub").display(),
            keys.join("dev.pub").display()
        );
        println!(
            "cargo::error=a listener with no trust anchor would refuse every cartridge. \
             Generate a signing key with `cargo run -p xtask -- keygen`."
        );
        return;
    }

    fs::write(
        out.join("trust_anchors.rs"),
        trust::keyfile::anchorsSource(&anchors),
    )
    .expect("could not write trust_anchors.rs into OUT_DIR");

    embed_resources();
}

/// Compiles `assets/listener.ico` and a version block into the exe.
///
/// The icon is what `LoadIconW` in `trigger/windows.rs` pulls the tray icon
/// from, and the version block's `FileDescription` is what Task Manager shows
/// instead of the bare filename — both come from the same resource, built
/// once here rather than requiring a `.rc` file of our own.
#[cfg(windows)]
fn embed_resources() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/listener.ico");
    res.set("FileDescription", "Romzeta Listener");
    res.set("ProductName", "Romzeta");
    res.compile()
        .expect("could not compile the Windows resources: check assets/listener.ico");
}

#[cfg(not(windows))]
fn embed_resources() {}
