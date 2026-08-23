// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! What the installer carries inside itself, and whether it survives being
//! unpacked.

#![allow(non_snake_case)] // camelCase functions

// ########## THE PAYLOAD ##########

mod common;

use common::{checks, runTest};
use installer::payload::{LAUNCHER_BYTES, LISTENER_BYTES, launcher, listener};

/// The launcher is carried compressed and written uncompressed, and the
/// minisign signature riding inside it *is* the cartridge's identity. A single
/// byte off and the cartridge still looks perfect, still contains a launcher of
/// the right name and size, and is silently ignored by every listener — with no
/// symptom but nothing happening.
///
/// So this checks the thing that cannot be checked by looking at the drive:
/// that what comes out of the payload is still signed.
#[test]
fn what_unpacks_is_still_signed() {
    runTest(|| {
        let mut proved = checks();
        for (name, unpacked, expected) in [
            ("launcher.exe", launcher(), LAUNCHER_BYTES),
            ("listener.exe", listener(), LISTENER_BYTES),
        ] {
            let bytes = unpacked.unwrap_or_else(|e| panic!("{name} did not unpack: {e}"));
            proved.expect(
                bytes.len() as u64 == expected,
                &format!("{name} unpacks to the size it claims"),
            );
            proved.expect(
                sigblock::isSigned(&bytes),
                &format!("{name} still carries its signature"),
            );
        }
        proved.verdict()
    });
}
