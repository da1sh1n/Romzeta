// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Takes a named mutex so a second launcher cannot open on top of the first,
//! and releases it when the guard drops.

// ########## SINGLE INSTANCE ##########

#[cfg(windows)]
pub type InstanceGuard = common::win32::InstanceGuard;

/// Returns `Some(guard)` if this is the first instance, `None` if another
/// is already running.
#[cfg(windows)]
pub fn acquire() -> Option<InstanceGuard> {
    common::win32::singleInstance(common::constants::LAUNCHER_INSTANCE_MUTEX)
}

#[cfg(not(windows))]
pub struct InstanceGuard;

#[cfg(not(windows))]
pub fn acquire() -> Option<InstanceGuard> {
    Some(InstanceGuard)
}
