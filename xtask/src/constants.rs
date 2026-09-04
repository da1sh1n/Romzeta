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
  cargo run -p xtask -- test [crate]     run tests and merge each crate's report
                                         (every crate with a tests/ folder, or just one)
  cargo run -p xtask -- version          show the project version and every crate's
";

// ========== The Test Report (report.rs) ==========

/// Where a crate's tests leave their per-category tables, under `<crate>/tests/`.
pub const TEST_LOGS: &str = "logs";

/// The column `xtask test` adds when it merges those tables into one.
pub const CATEGORY_COLUMN: &str = "Category";

// ========== The Version Contract (manifest.rs, release.rs) ==========

/// The crates that ship to a user and are held to `project_version`'s major.
/// Everything else in the workspace is still read for its name and version,
/// but a helper crate like `testkit` never ships and would need bumping for no
/// reason if it were held to the same contract.
pub const SHIPPED_CRATES: &[&str] = &["launcher", "listener", "keeper", "installer"];

// ========== Signing Keys (keys.rs) ==========

/// Overrides the secret key location. Holds a path, not a key.
pub const KEY_VAR: &str = "ROMZETA_SIGNING_KEY";
/// The password for it. Absent means the key is unencrypted, or that you will
/// be asked at the prompt.
pub const PASSWORD_VAR: &str = "ROMZETA_SIGNING_PASSWORD";
