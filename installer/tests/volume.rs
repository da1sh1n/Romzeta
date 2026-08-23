// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Which drives may be written to, and what makes one a cartridge already.

#![allow(non_snake_case)] // camelCase functions

// ########## VOLUMES ##########

mod common;

use std::fs;

use common::{Scratch, checks, runTest};
use installer::constants::LAUNCHER_NAME;
use installer::volume::{ANCHORS, attestedLauncher};

/// The build carries at least one usable anchor. Same assertion as the
/// listener's `this_build_has_something_to_trust` — the two must agree, or a
/// cartridge one of them would accept the other silently would not.
#[test]
fn this_build_has_something_to_trust() {
    runTest(|| {
        let mut proved = checks();
        proved.expect(!ANCHORS.is_empty(), "build.rs produced trust anchors");
        for anchor in ANCHORS {
            proved.expect(
                anchor.isUsable(),
                &format!("keys/{}.pub is a usable minisign public key", anchor.name),
            );
        }
        proved.verdict()
    });
}

/// The finding this change exists to fix: a `launcher.exe` at a drive root is
/// not believed just because it has the right name. Without a signing key this
/// cannot construct something that *does* verify — that round trip is `trust`'s
/// own suite — but every one of these must come back `None` rather than "close
/// enough".
#[test]
fn only_a_verified_signature_makes_a_cartridge() {
    runTest(|| {
        let dir = Scratch::new("volume-attest");
        let mut proved = checks();

        // No file at all.
        proved.expect(
            attestedLauncher(dir.path()).is_none(),
            "an empty drive is not a cartridge",
        );

        // A file with the right name and nothing else — what running it used to
        // accept.
        fs::write(dir.join(LAUNCHER_NAME), b"MZ not signed").expect("write");
        proved.expect(
            attestedLauncher(dir.path()).is_none(),
            "the right name alone is not enough",
        );

        // A well-formed signature block from a key this build does not carry.
        let signature = "untrusted comment: signature from a key we do not have\n\
                         RUQAAAAAAAAAAOaGxHqZQ0KtvVCJ6iKzXG8bFvKZ0V0kZ1qWzKz0hVYQ4rZ8Xk1t\n\
                         trusted comment: romzeta-launcher 9.9.9 2026-07-30\n\
                         AAAA==\n";
        let signed = sigblock::attach(b"MZ signed by someone else", signature);
        fs::write(dir.join(LAUNCHER_NAME), signed).expect("write");
        proved.expect(
            attestedLauncher(dir.path()).is_none(),
            "nor is a stranger's signature",
        );
        proved.verdict()
    });
}

#[cfg(windows)]
#[test]
fn the_drive_windows_is_on_is_refused() {
    runTest(|| {
        use installer::volume::{Eligibility, driveLetter, isSystemDrive, list};
        use std::path::Path;

        // The most important behaviour in this module, asserted against the
        // machine running the test rather than a fixture.
        let system = std::env::var_os("SystemRoot").expect("Windows sets SystemRoot");
        let letter = driveLetter(Path::new(&system)).expect("a drive letter");

        let mut proved = checks();
        proved.expect(
            isSystemDrive(Path::new(&format!("{letter}:\\"))),
            "the system root is the system drive",
        );
        proved.expect(
            isSystemDrive(Path::new(&format!("{}:\\", letter.to_ascii_lowercase()))),
            "and so is the same letter in lower case",
        );
        proved.expect(
            isSystemDrive(Path::new(&format!("{letter}:\\games"))),
            "and a folder on it",
        );

        for volume in list() {
            if driveLetter(&volume.root) == Some(letter) {
                proved.expect(
                    volume.eligibility == Eligibility::SystemDrive,
                    "the system drive is listed as such",
                );
                proved.expect(!volume.allowed(), "and is never offered");
            }
        }
        proved.verdict()
    });
}

#[cfg(windows)]
#[test]
fn nothing_internal_is_ever_allowed() {
    runTest(|| {
        use installer::volume::{isSystemDrive, list};

        let mut proved = checks();
        for volume in list() {
            if volume.allowed() {
                proved.expect(
                    !isSystemDrive(&volume.root),
                    "an offered drive is not the system one",
                );
                proved.expect(
                    matches!(volume.bus, "USB" | "FireWire" | "SD" | "MMC" | "removable"),
                    "an offered drive is on a removable bus",
                );
            }
        }
        proved.verdict()
    });
}
