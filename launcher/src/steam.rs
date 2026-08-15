// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Brings the Steam client up without a window, for games whose DRM refuses to
//! run without it, and waits until it can actually serve the handshake.

// ########## THE STEAM CLIENT ##########

use std::path::Path;
#[cfg(windows)]
use std::path::PathBuf;
#[cfg(windows)]
use std::process::{Command, Stdio};
#[cfg(windows)]
use std::thread;
#[cfg(windows)]
use std::time::Instant;

#[cfg(windows)]
use common::reg::{self, HKEY_CURRENT_USER as HKCU, HKEY_LOCAL_MACHINE as HKLM};

#[cfg(windows)]
use crate::constants::{STEAM_POLL, STEAM_WAIT};
use crate::log;

/// Where the client records itself for the current user. `SteamExe` here is a
/// full path, written with forward slashes.
#[cfg(windows)]
const USER_KEY: &str = r"Software\Valve\Steam";

/// The liveness key. This is what `steam_api.dll` itself reads to find the
/// client, so polling it asks the handshake rather than guessing at it.
#[cfg(windows)]
const ACTIVE_KEY: &str = r"Software\Valve\Steam\ActiveProcess";

/// Machine-wide fallbacks, for a profile that has never run Steam. The client
/// is 32-bit, so on 64-bit Windows it lands under WOW6432Node.
#[cfg(windows)]
const MACHINE_KEYS: [&str; 2] = [r"SOFTWARE\WOW6432Node\Valve\Steam", r"SOFTWARE\Valve\Steam"];

/// Starts Steam silently and blocks until it is ready to answer a game, or
/// returns the one line to put under that game's cover. `Ok(())` also covers
/// "it was already running", which is the common case and costs nothing.
///
/// Call on a worker thread: the wait runs to [`STEAM_WAIT`].
#[cfg(windows)]
pub fn ensureRunning(base: &Path) -> Result<(), String> {
    if isReady() {
        log::line(base, "steam is already up");
        return Ok(());
    }

    let Some(exe) = steamExe() else {
        log::line(
            base,
            "FAILED steam: no SteamExe or InstallPath in the registry",
        );
        return Err("Failed to start — Steam isn't installed".to_string());
    };

    // `-silent` is the whole point: Steam goes to the tray and never puts a
    // window in front of the launcher.
    log::line(
        base,
        &format!("starting steam silently ({})", exe.display()),
    );
    let mut command = Command::new(&exe);
    command
        .arg("-silent")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    crate::launch::suppressConsoleWindow(&mut command);

    match command.spawn() {
        // Deliberately dropped rather than waited on. Steam's first process
        // re-execs itself after an update, so its exit says nothing about
        // whether the client came up — `isReady` is the only answer.
        Ok(child) => drop(child),
        Err(e) => {
            log::line(base, &format!("FAILED steam: {e}"));
            return Err("Failed to start — Steam would not start".to_string());
        }
    }

    let deadline = Instant::now() + STEAM_WAIT;
    loop {
        if isReady() {
            log::line(base, "steam is up");
            return Ok(());
        }
        if Instant::now() >= deadline {
            log::line(base, "FAILED steam: still not ready after the wait");
            return Err("Failed to start — Steam didn't finish starting".to_string());
        }
        thread::sleep(STEAM_POLL);
    }
}

/// The readiness check reads Steam's own registry keys, which exist only on
/// Windows. Rather than start a client nothing can then verify, this says so and
/// lets the player see why.
#[cfg(not(windows))]
pub fn ensureRunning(base: &Path) -> Result<(), String> {
    log::line(base, "FAILED steam: only supported on Windows");
    Err("Failed to start — Steam support needs Windows".to_string())
}

/// Whether a game could complete its DRM handshake right now.
///
/// All three values matter. A `pid` alone survives a crash, and a client that is
/// up but not signed in yet fails the handshake exactly as if it were absent —
/// which is the whole reason this waits rather than sleeping a fixed while.
#[cfg(windows)]
fn isReady() -> bool {
    let Some(key) = reg::open(HKCU, ACTIVE_KEY, reg::READ) else {
        return false;
    };
    let pid = reg::queryDword(&key, Some("pid")).unwrap_or(0);
    let user = reg::queryDword(&key, Some("ActiveUser")).unwrap_or(0);
    if pid == 0 || user == 0 || !processAlive(pid) {
        return false;
    }
    // Either name is enough: the 32-bit client always writes the first, and only
    // 64-bit hosts write the second.
    ["SteamClientDll", "SteamClientDll64"]
        .iter()
        .filter_map(|name| reg::querySz(&key, Some(name)))
        .any(|path| Path::new(&path).is_file())
}

/// `steam.exe`, from the user's own key first and the machine-wide install path
/// after it. Resolved at launch time, never at install time — the cartridge
/// moves between machines that keep Steam in different places.
#[cfg(windows)]
fn steamExe() -> Option<PathBuf> {
    if let Some(key) = reg::open(HKCU, USER_KEY, reg::READ)
        && let Some(exe) = reg::querySz(&key, Some("SteamExe"))
        && Path::new(&exe).is_file()
    {
        return Some(PathBuf::from(exe));
    }
    MACHINE_KEYS.iter().find_map(|path| {
        let key = reg::open(HKLM, path, reg::READ)?;
        let dir = reg::querySz(&key, Some("InstallPath"))?;
        let exe = PathBuf::from(dir).join("steam.exe");
        exe.is_file().then_some(exe)
    })
}

/// Whether `pid` still names a running process.
#[cfg(windows)]
fn processAlive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED, GetLastError};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if !handle.is_null() {
        unsafe { CloseHandle(handle) };
        return true;
    }
    // A Steam running elevated is one we may not open but which is very much
    // alive. Only "no such process" means gone.
    unsafe { GetLastError() == ERROR_ACCESS_DENIED }
}
