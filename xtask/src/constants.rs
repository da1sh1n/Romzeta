// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Every constant the crate owns, in one file. Section headers name the module
//! each belongs to.

// ########## BUILD TOOL CONSTANTS ##########

// ========== The Command Line (main.rs) ==========

pub const USAGE: &str = "\
Romzeta build tool.

  cargo run -p xtask -- release          build and sign launcher, listener, installer
  cargo run -p xtask -- keygen           generate a dev signing key -> keys/dev.pub
  cargo run -p xtask -- keygen --release the one release key -> keys/romzeta.pub (committed)
  cargo run -p xtask -- sign <exe>...    sign in place
  cargo run -p xtask -- verify <exe>...  check against keys/romzeta.pub and keys/dev.pub
  cargo run -p xtask -- version          show the project version and every crate's
";

// ========== Signing Keys (keys.rs) ==========

/// Overrides the secret key location. Holds a path, not a key.
pub const KEY_VAR: &str = "ROMZETA_SIGNING_KEY";
/// The password for it. Absent means the key is unencrypted, or that you will
/// be asked at the prompt.
pub const PASSWORD_VAR: &str = "ROMZETA_SIGNING_PASSWORD";
