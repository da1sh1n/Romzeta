// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Crate root. Declares the shared modules: the crate's constants, the
//! AppUserModelID call, log writing, the cross-process game lease, UTC time,
//! the `x.y.z` version type, the registry wrapper, and Win32 UTF-16 conversion.

// Functions are camelCase in this project while variables stay snake_case, which
// is the opposite of what rustc expects. Silenced once, at the root, so no item
// below needs its own attribute.
#![allow(non_snake_case)]

// ########## SHARED PIECES ##########

pub mod aumid;
pub mod constants;
pub mod lease;
pub mod log;
pub mod time;
pub mod version;

// Only the Windows builds ever call a `…W` entry point, and on Linux these
// modules would be dead code the compiler warns about.
#[cfg(windows)]
pub mod reg;
#[cfg(windows)]
pub mod utf16;
