// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Everything the listener does, minus the entry point. A library rather than
//! modules of the binary so `tests/` can reach it: an integration test cannot
//! see inside a `[[bin]]`-only crate.

#![allow(non_snake_case)] // camelCase functions

// ########## THE LISTENER ##########

pub mod alert;
pub mod constants;
pub mod log;
pub mod trigger;
pub mod trust;
pub mod version;
pub mod volume;
