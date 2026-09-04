// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Shared active-game lease helpers for launcher/listener coordination.

// ########## ACTIVE GAME LEASE ##########

use std::fs;
use std::io;
use std::path::PathBuf;

use crate::constants::LEASE_FILE;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lease {
    pub pid: u32,
}

pub fn leasePath() -> PathBuf {
    baseDir().join(LEASE_FILE)
}

pub fn writeLease(pid: u32) -> io::Result<()> {
    let path = leasePath();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut payload = String::new();
    payload.push_str("pid=");
    payload.push_str(&pid.to_string());
    payload.push('\n');
    fs::write(path, payload)
}

pub fn readLease() -> Option<Lease> {
    let text = fs::read_to_string(leasePath()).ok()?;
    let mut pid = None;

    for line in text.lines() {
        if let Some(value) = line.strip_prefix("pid=") {
            pid = value.trim().parse::<u32>().ok();
        }
    }

    Some(Lease { pid: pid? })
}

pub fn clearLease() -> io::Result<()> {
    let path = leasePath();
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(windows)]
pub fn processExists(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject};

    const SYNCHRONIZE: u32 = 0x0010_0000;

    unsafe {
        let handle = OpenProcess(SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            return false;
        }
        let wait = WaitForSingleObject(handle, 0);
        CloseHandle(handle);
        wait == WAIT_TIMEOUT
    }
}

#[cfg(not(windows))]
pub fn processExists(pid: u32) -> bool {
    PathBuf::from(format!("/proc/{pid}")).exists()
}

fn baseDir() -> PathBuf {
    crate::paths::romzetaDataDir().unwrap_or_else(|| std::env::temp_dir().join("Romzeta"))
}
