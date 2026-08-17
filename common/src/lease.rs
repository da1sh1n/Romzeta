// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Shared active-game lease helpers for launcher/listener coordination.

// ########## ACTIVE GAME LEASE ##########

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const LEASE_FILE: &str = "active_game_lease.txt";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lease {
    pub pid: u32,
    pub cartridge_root: PathBuf,
}

pub fn leasePath() -> PathBuf {
    baseDir().join(LEASE_FILE)
}

pub fn writeLease(pid: u32, cartridge_root: &Path) -> io::Result<()> {
    let path = leasePath();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut payload = String::new();
    payload.push_str("pid=");
    payload.push_str(&pid.to_string());
    payload.push('\n');
    payload.push_str("cartridge_root=");
    payload.push_str(&cartridge_root.to_string_lossy());
    payload.push('\n');
    fs::write(path, payload)
}

pub fn readLease() -> Option<Lease> {
    let text = fs::read_to_string(leasePath()).ok()?;
    let mut pid = None;
    let mut cartridge_root = None;

    for line in text.lines() {
        if let Some(value) = line.strip_prefix("pid=") {
            pid = value.trim().parse::<u32>().ok();
            continue;
        }
        if let Some(value) = line.strip_prefix("cartridge_root=") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                cartridge_root = Some(PathBuf::from(trimmed));
            }
        }
    }

    Some(Lease {
        pid: pid?,
        cartridge_root: cartridge_root?,
    })
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
    #[cfg(windows)]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local).join("Romzeta");
        }
    }

    #[cfg(not(windows))]
    {
        if let Some(state_home) = std::env::var_os("XDG_STATE_HOME") {
            return PathBuf::from(state_home).join("romzeta");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(".local").join("state").join("romzeta");
        }
    }

    std::env::temp_dir().join("Romzeta")
}
