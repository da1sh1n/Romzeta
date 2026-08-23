// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! This listener's own `x.y.z` and printing it. The type and parser come from
//! `common::version`.

// ########## THIS LISTENER'S VERSION ##########

use common::version::{Version, parse};

/// This listener's own version, read from its Cargo manifest at compile time.
/// `Cargo.toml` is the only place it is written.
pub fn own() -> Version {
    parse(env!("CARGO_PKG_VERSION"))
        .expect("CARGO_PKG_VERSION is not x.y.z: fix version in listener/Cargo.toml")
}

/// Prints the `--version` line: `x.y.z`, no program name and no `v`.
pub fn print() {
    println!("{}", own());
}
