// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

// Functions are camelCase in this project while variables stay snake_case,
// which rustc's default lints object to. Silenced once, at the script's root.
#![allow(non_snake_case)]

//! Build script. Embeds FileDescription/ProductName so Task Manager shows
//! "Romzeta Keeper" instead of the bare filename.

// ########## EMBEDDING VERSION INFO ##########

fn main() {
    embedResources();
}

#[cfg(windows)]
fn embedResources() {
    let mut res = winres::WindowsResource::new();
    res.set("FileDescription", "Romzeta Keeper");
    res.set("ProductName", "Romzeta");
    res.compile()
        .expect("compile Windows resources (version info)");
}

#[cfg(not(windows))]
fn embedResources() {}
