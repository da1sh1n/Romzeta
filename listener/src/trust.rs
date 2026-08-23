// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Verifies `<root>/launcher.exe` against the anchors compiled into this build,
//! holding the file open while it is checked, and phrases the refusal when it
//! fails. Reads the bytes; runs nothing.

// ########## IS THIS VOLUME A CARTRIDGE ##########

use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use trust::Anchor;

use crate::constants::LAUNCHER_NAME;

// `ANCHORS: &[Anchor]`, written by build.rs from keys/*.pub and pasted in here at compile time.
// Compiled in rather than read from disk: an anchor sitting in a writable file beside the exe
// would let anything able to edit it grant itself auto-run.
include!(concat!(env!("OUT_DIR"), "/trust_anchors.rs"));

/// A launcher that verified, what vouched for it, and what it says it is.
pub struct Trusted {
    pub path: PathBuf,
    /// Which baked-in key accepted it — `release` or `dev`.
    pub anchor: String,
    /// The `x.y.z` from the signed comment. Authenticated, so it is the
    /// launcher's version without the launcher having been asked.
    pub version: String,
    /// The open handle the bytes were read through, kept alive so the file
    /// cannot be swapped between verifying it and running it. Never read; the
    /// leading `_` marks it as held for its lifetime alone.
    _lock: File,
}

/// Why a volume is not a cartridge worth launching. Every variant ends the same
/// way, but they stay distinct because the log is this program's only
/// diagnostic and "ordinary USB stick" must not read like "someone tampered".
pub enum Refusal {
    /// Launcher missing
    NoLauncher,
    /// Launcher unreadable
    Unreadable(String),
    /// Launcher with unsafe or missing signature.
    Signature(trust::Refusal),
}

impl fmt::Display for Refusal {
    // `fmt` is fixed by the Display trait, so it keeps rustc's spelling.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::NoLauncher => write!(f, "no {LAUNCHER_NAME} at the volume root"),
            Refusal::Unreadable(e) => write!(f, "{LAUNCHER_NAME} could not be read: {e}"),
            Refusal::Signature(trust::Refusal::Untrusted) => write!(
                f,
                "{LAUNCHER_NAME} is signed, but not by a key this listener trusts ({})",
                anchorNames()
            ),
            Refusal::Signature(reason) => write!(f, "{LAUNCHER_NAME}: {reason}"),
        }
    }
}

impl Refusal {
    /// The sentence to show the user, or `None` when this refusal is not theirs
    /// to act on. An ordinary drive is not a fault, and a read that failed on a
    /// volume still settling is not one they can answer either.
    pub fn explain(&self) -> Option<String> {
        let reason = match self {
            Refusal::NoLauncher | Refusal::Unreadable(_) => return None,
            Refusal::Signature(trust::Refusal::Unsigned) => format!(
                "The {LAUNCHER_NAME} on this cartridge carries no signature.\n\n\
                Romzeta only starts a launcher signed with a key this PC trusts."
            ),
            Refusal::Signature(trust::Refusal::Malformed(_)) => format!(
                "The signature on this cartridge's {LAUNCHER_NAME} could not be read.\n\n\
                The file is damaged, or the copy onto the cartridge did not finish."
            ),
            Refusal::Signature(trust::Refusal::Untrusted) => format!(
                "This cartridge's {LAUNCHER_NAME} is properly signed, but not by a key this \
                PC trusts.\n\n\
                It trusts: {}.",
                anchorNames()
            ),
            Refusal::Signature(trust::Refusal::WrongRole { expected, found }) => format!(
                "The file at this cartridge's root is a signed {found}, and a {expected} was \
                expected.\n\n\
                It is genuine, but it is not the program that starts a cartridge."
            ),
        };
        Some(format!("{reason}"))
    }
}

/// Verifies `<root>/launcher.exe` against every baked-in anchor, returning the
/// held-open file and what its signature says, or why it was refused.
pub fn verifyLauncher(root: &Path) -> Result<Trusted, Refusal> {
    let path = root.join(LAUNCHER_NAME);
    if !path.is_file() {
        return Err(Refusal::NoLauncher);
    }

    let mut file = openLocked(&path).map_err(|e| Refusal::Unreadable(e.to_string()))?;

    // A seek and sixteen bytes, before the file goes anywhere near memory: the
    // thing at a volume root named launcher.exe can be any size at all, and an
    // unsigned one is refused the same way whether or not it was read first.
    if !sigblock::hasBlock(&mut file).map_err(|e| Refusal::Unreadable(e.to_string()))? {
        return Err(Refusal::Signature(trust::Refusal::Unsigned));
    }

    let bytes = readAll(&mut file).map_err(|e| Refusal::Unreadable(e.to_string()))?;

    let attested = trust::attest(&bytes, ANCHORS, trust::constants::LAUNCHER_ROLE)
        .map_err(Refusal::Signature)?;

    Ok(Trusted {
        path,
        anchor: attested.anchor,
        version: attested.version,
        _lock: file,
    })
}

/// Opens `path` so nothing else can write to or delete it while the handle
/// lives.
///
/// Verifying bytes and then executing a *path* is two different files if
/// anything can change the disk in between, and the disk was plugged in by a
/// stranger.
#[cfg(windows)]
fn openLocked(path: &Path) -> std::io::Result<File> {
    // The extension trait that adds `share_mode` to OpenOptions.
    use std::os::windows::fs::OpenOptionsExt;

    /// `windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ`, spelled out
    /// rather than pulling that crate onto a path this file otherwise shares.
    const FILE_SHARE_READ: u32 = 0x0000_0001;

    // Reads shared, writes and deletes refused: the image loader still needs a
    // read handle to start the process.
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(path)
}

/// Unix has no share modes — an open handle excludes nothing — so this is a
/// plain open. The handle is still returned and still held, so both builds have
/// one shape rather than two.
#[cfg(not(windows))]
fn openLocked(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

/// Reads the whole file from the top, wherever the footer probe left the cursor.
fn readAll(file: &mut File) -> std::io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// The anchors this build carries, comma-separated, for the log line and --signature`.
pub fn anchorNames() -> String {
    ANCHORS
        .iter()
        .map(|a| a.name)
        .collect::<Vec<_>>()
        .join(", ")
}
