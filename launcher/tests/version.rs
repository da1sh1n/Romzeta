// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! The shape of this launcher's own version number.

#![allow(non_snake_case)] // camelCase functions

// ########## THIS LAUNCHER'S VERSION ##########

mod common;

use common::{checks, runTest};

#[test]
fn our_version_is_a_bare_three_part_number() {
    runTest(|| {
        // What the listener's parser accepts, asserted from this side too: if
        // this ever became "v0.2.0" or "0.2", every cartridge built from it
        // would stop being launchable and the log would say only that the
        // version was unreadable.
        let version = env!("CARGO_PKG_VERSION");
        let parts: Vec<&str> = version.split('.').collect();

        let mut proved = checks();
        proved.expect(parts.len() == 3, "the version has three parts");
        proved.expect(
            parts.iter().all(|part| part.parse::<u64>().is_ok()),
            "every part is a plain number",
        );
        proved.verdict()
    });
}
