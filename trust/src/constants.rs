// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Every constant the crate owns, in one file. Section headers name the module
//! each belongs to.

// ########## TRUST CONSTANTS ##########

// ========== Role Names (lib.rs) ==========

/// The role names `xtask` writes into the trusted comment and `attest` matches
/// against. Defined here, where both the signer and the checker can see them: a
/// disagreement about this string makes a cartridge every listener ignores.
pub const LAUNCHER_ROLE: &str = "romzeta-launcher";
pub const LISTENER_ROLE: &str = "romzeta-listener";
pub const INSTALLER_ROLE: &str = "romzeta-installer";
pub const KEEPER_ROLE: &str = "romzeta-keeper";

// ========== Stem -> Role (xtask/src/sign.rs) ==========

/// The role for a binary's file stem — `"launcher"`, `"listener"`,
/// `"installer"` or `"keeper"` — or `None` for anything xtask does not sign or
/// verify. `xtask sign` and `xtask verify` both derive the role this way, so
/// the two commands cannot disagree about what a file is.
pub fn roleForStem(stem: &str) -> Option<&'static str> {
    match stem {
        "launcher" => Some(LAUNCHER_ROLE),
        "listener" => Some(LISTENER_ROLE),
        "installer" => Some(INSTALLER_ROLE),
        "keeper" => Some(KEEPER_ROLE),
        _ => None,
    }
}
