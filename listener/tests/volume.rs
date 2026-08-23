// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Handling one volume end to end: verify, compare, start or refuse.

#![allow(non_snake_case)] // camelCase functions

// ########## HANDLING ONE VOLUME ##########

mod common;

use common::{Scratch, appLog, runTest, verdict};
use listener::constants::LAUNCHER_NAME;
use listener::volume::{Announce, Outcome, handleVolume};
use std::fs;

/// Names an `Outcome` for the report. A plain match on the public variants, so
/// the app's enum needs no derives to be testable.
fn outcomeName(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Launched => "Launched",
        Outcome::Ignored => "Ignored",
        Outcome::Failed => "Failed",
    }
}

/// Builds a fake volume in this test's fixture folder.
fn fakeVolume(name: &str, launcher: Option<&[u8]>) -> Scratch {
    let dir = Scratch::new(name);
    if let Some(bytes) = launcher {
        fs::write(dir.join(LAUNCHER_NAME), bytes)
            .expect("could not write the fake volume launcher");
    }
    dir
}

#[test]
fn a_volume_with_no_launcher_is_ignored() {
    runTest(|| {
        let dir = fakeVolume("volume-plain", None);
        verdict(
            outcomeName(handleVolume(dir.path(), &appLog(), Announce::Never)),
            "Ignored",
        )
    });
}

#[test]
fn an_unsigned_launcher_is_never_started() {
    runTest(|| {
        // The whole point of the change: a binary sitting at a volume root with
        // the right *name* gets nowhere without the right signature.
        let dir = fakeVolume("volume-unsigned", Some(b"MZ nobody signed this"));
        verdict(
            outcomeName(handleVolume(dir.path(), &appLog(), Announce::Never)),
            "Ignored",
        )
    });
}

#[test]
fn a_launcher_signed_by_a_stranger_is_never_started() {
    runTest(|| {
        let signature = "untrusted comment: signature from a key we do not have\n\
                         RUQAAAAAAAAAAOaGxHqZQ0KtvVCJ6iKzXG8bFvKZ0V0kZ1qWzKz0hVYQ4rZ8Xk1t\n\
                         trusted comment: romzeta-launcher 0.2.0\n\
                         AAAA==\n";
        let signed = sigblock::attach(b"MZ signed by someone else", signature);
        let dir = fakeVolume("volume-stranger", Some(&signed));
        verdict(
            outcomeName(handleVolume(dir.path(), &appLog(), Announce::Never)),
            "Ignored",
        )
    });
}
