// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Whether a launcher on a volume is one this PC may run.

#![allow(non_snake_case)] // camelCase functions

// ########## VERIFYING A LAUNCHER ##########

mod common;

use common::{Scratch, runTest, verdict};
use listener::constants::LAUNCHER_NAME;
use listener::trust::{ANCHORS, Refusal, verifyLauncher};
use std::fs;

/// Names what `verifyLauncher` decided, for the report. A plain match on the
/// public variants, so neither enum needs a derive to be testable.
fn verdictOn(dir: &Scratch) -> String {
    match verifyLauncher(dir.path()) {
        Ok(_) => "Accepted".to_owned(),
        Err(Refusal::NoLauncher) => "NoLauncher".to_owned(),
        Err(Refusal::Unreadable(_)) => "Unreadable".to_owned(),
        Err(Refusal::Signature(reason)) => format!("Signature({})", signatureName(&reason)),
    }
}

fn signatureName(reason: &::trust::Refusal) -> &'static str {
    match reason {
        ::trust::Refusal::Unsigned => "Unsigned",
        ::trust::Refusal::Malformed(_) => "Malformed",
        ::trust::Refusal::Untrusted => "Untrusted",
        ::trust::Refusal::WrongRole { .. } => "WrongRole",
    }
}

/// A volume with nothing on it is the ordinary case, and the one that has to
/// stay cheap: this is every USB stick anyone ever plugs in.
#[test]
fn a_volume_with_no_launcher_is_not_a_cartridge() {
    runTest(|| {
        let dir = Scratch::new("trust-empty");
        verdict(verdictOn(&dir), "NoLauncher")
    });
}

#[test]
fn an_unsigned_launcher_is_refused() {
    runTest(|| {
        let dir = Scratch::new("trust-unsigned");
        fs::write(dir.join(LAUNCHER_NAME), b"MZ but nobody signed it")
            .expect("could not write the unsigned launcher fixture");
        verdict(verdictOn(&dir), "Signature(Unsigned)")
    });
}

/// The handle `verifyLauncher` holds has to deny writers without denying
/// readers — the image loader needs one to start the process, so a lock that
/// excluded everything would make every genuine cartridge fail to launch.
/// Verified on a file we control, since the signed path cannot be reached
/// here without a signing key.
#[cfg(windows)]
#[test]
fn the_lock_denies_writers_and_allows_readers() {
    runTest(|| {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 0x0000_0001;

        let dir = Scratch::new("trust-lock");
        let path = dir.join("locked.bin");
        fs::write(&path, b"MZ").expect("could not write the lock fixture");

        let _held = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&path)
            .expect("could not open the fixture with the share mode verifyLauncher uses");

        // A reader must still get in; a writer must not, since that is what
        // stops the file being swapped between verifying it and running it.
        let reader = if fs::read(&path).is_ok() { "in" } else { "out" };
        let writer = if fs::OpenOptions::new().write(true).open(&path).is_ok() {
            "in"
        } else {
            "out"
        };
        let deleter = if fs::remove_file(&path).is_ok() {
            "in"
        } else {
            "out"
        };

        verdict(
            format!("reader {reader}, writer {writer}, deleter {deleter}"),
            "reader in, writer out, deleter out",
        )
    });
}

#[test]
fn a_launcher_signed_by_a_stranger_is_refused() {
    runTest(|| {
        // The signature is real and well-formed; it is simply not ours. This is
        // the case the whole module exists for, and the one that used to be
        // "copy the key off the cartridge and write it into your own".
        let dir = Scratch::new("trust-stranger");

        let signature = "untrusted comment: signature from a key we do not have\n\
                         RUQAAAAAAAAAAOaGxHqZQ0KtvVCJ6iKzXG8bFvKZ0V0kZ1qWzKz0hVYQ4rZ8Xk1t\
                         Yy0jVQhJZ0kZ1qWzKz0hVYQ4rZ8Xk1tYy0jVQ==\n\
                         trusted comment: romzeta-launcher 9.9.9\n\
                         AAAA==\n";
        let signed = sigblock::attach(b"MZ signed by someone else", signature);
        fs::write(dir.join(LAUNCHER_NAME), signed)
            .expect("could not write the stranger-signed launcher fixture");

        // Either it fails to decode or it fails to verify; both are refusals and
        // neither launches anything. What must never happen is Accepted.
        let refused = if verdictOn(&dir) == "Accepted" {
            "accepted"
        } else {
            "refused"
        };
        verdict(refused, "refused")
    });
}

#[test]
fn this_build_has_something_to_trust() {
    runTest(|| {
        // A listener compiled with no anchors would refuse every cartridge in
        // existence, one puzzled log line at a time. build.rs makes that a build
        // error; this makes sure the generated file is actually wired up.
        let state = if ANCHORS.is_empty() {
            "no anchors".to_owned()
        } else {
            match ANCHORS.iter().find(|anchor| !anchor.isUsable()) {
                Some(bad) => format!("keys/{}.pub is not a usable minisign public key", bad.name),
                None => "all anchors usable".to_owned(),
            }
        };
        verdict(state, "all anchors usable")
    });
}
