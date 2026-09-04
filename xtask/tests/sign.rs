// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Putting a signature into a file and reading it back, against the real
//! libraries rather than a stand-in.

#![allow(non_snake_case)] // camelCase functions

// ########## SIGNING AND VERIFYING ##########

mod common;

use std::fs;

use common::{Scratch, checks, runTest, verdict};
use xtask::keys::{Anchor, base64Line};
use xtask::sign::{sign, verify};

/// An anchor list holding `pair`'s public half, as a build's baked-in anchors
/// would look.
fn anchors(name: &'static str, pair: &minisign::KeyPair) -> Vec<Anchor> {
    vec![Anchor {
        name,
        base64: base64Line(&pair.pk.to_box().expect("pk").to_string()).expect("base64 line"),
    }]
}

/// The full round trip: generate a key, sign a buffer, verify it, then break it
/// and watch verification fail.
#[test]
fn signs_and_verifies_a_binary() {
    runTest(|| {
        let dir = Scratch::new("xtask-sign");
        let pair = minisign::KeyPair::generate_unencrypted_keypair().expect("keypair");
        let trusted = anchors("dev", &pair);

        let exe = dir.join("fake.exe");
        fs::write(&exe, b"MZ pretend this is a launcher").expect("write");
        sign(&exe, &pair.sk, "romzeta-launcher 0.2.0").expect("sign");

        let mut proved = checks();
        let verified = verify(&exe, &trusted, "romzeta-launcher").unwrap_or_else(|e| panic!("{e}"));
        proved.expect(
            verified.anchor == "dev",
            "the dev key is the one that matched",
        );
        proved.expect(
            verified.comment.starts_with("romzeta-launcher 0.2.0 "),
            "the trusted comment says what was signed",
        );

        // Signing twice must replace, not nest — and must still verify.
        let once = fs::metadata(&exe).expect("stat").len();
        sign(&exe, &pair.sk, "romzeta-launcher 0.2.1").expect("re-sign");
        proved.expect(
            fs::metadata(&exe).expect("stat").len() <= once + 8,
            "re-signing replaced the block instead of nesting one",
        );
        proved.expect(
            verify(&exe, &trusted, "romzeta-launcher").is_ok(),
            "and it still verifies",
        );

        // One flipped byte in the payload, and it is no longer ours.
        let mut bytes = fs::read(&exe).expect("read");
        bytes[1] ^= 0xff;
        fs::write(&exe, &bytes).expect("write");
        proved.expect(
            verify(&exe, &trusted, "romzeta-launcher").is_err(),
            "a flipped payload byte breaks it",
        );
        proved.verdict()
    });
}

#[test]
fn an_unsigned_binary_is_reported_as_such() {
    runTest(|| {
        let dir = Scratch::new("xtask-unsigned");
        let exe = dir.join("bare.exe");
        fs::write(&exe, b"MZ and nothing else").expect("write");

        let error = verify(&exe, &[], "romzeta-launcher").expect_err("unsigned");
        verdict(
            if error.contains("no signature block") {
                "no signature block"
            } else {
                &error
            },
            "no signature block",
        )
    });
}

#[test]
fn a_signature_from_another_key_is_refused() {
    runTest(|| {
        let dir = Scratch::new("xtask-otherkey");
        let ours = minisign::KeyPair::generate_unencrypted_keypair().expect("keypair");
        let theirs = minisign::KeyPair::generate_unencrypted_keypair().expect("keypair");

        let exe = dir.join("theirs.exe");
        fs::write(&exe, b"MZ signed by someone else").expect("write");
        sign(&exe, &theirs.sk, "romzeta-launcher 0.2.0").expect("sign");

        let error =
            verify(&exe, &anchors("romzeta", &ours), "romzeta-launcher").expect_err("not our key");
        verdict(
            if error.contains("not by any key this tree trusts") {
                "not by any key this tree trusts"
            } else {
                &error
            },
            "not by any key this tree trusts",
        )
    });
}
