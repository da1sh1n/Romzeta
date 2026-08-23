// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Every constant the crate owns, in one file. Section headers name the module
//! each belongs to.

// ########## SIGNATURE BLOCK CONSTANTS ##########

// ========== The Footer (lib.rs) ==========

/// Identifies the footer. Ten bytes exactly, which keeps the whole thing 16.
pub const MAGIC: &[u8; 10] = b"ROMZETASIG";

/// The only block format that exists. A block declaring anything else is left
/// alone rather than guessed at, since a future format would move these fields.
pub const FORMAT: u16 = 1;

/// Size of the fixed footer: length (4) + format (2) + magic (10).
pub const FOOTER_LEN: usize = 16;

/// Largest block either reader accepts. A minisig is about 200 bytes; the cap
/// is there because a crafted footer must not be able to ask for a
/// gigabyte-sized allocation before a single byte has been verified.
pub const MAX_BLOCK_LEN: u32 = 64 * 1024;
