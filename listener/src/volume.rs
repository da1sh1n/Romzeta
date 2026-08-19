// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Handles one volume: verify its launcher, compare project versions, start it.
//! Logs the reason on every path out.
//!
//! ```text
//! read the bytes -> verify the signature -> ask the signature what it is -> run it
//! ```

// ########## HANDLING ONE VOLUME ##########

use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::log::Log;
use crate::{alert, trust, version};

const PROCESS_CHECK_INTERVAL_MS: u64 = 10_000;

struct GateState {
    last_check: Option<Instant>,
    active: bool,
    pid: Option<u32>,
}

impl GateState {
    fn new() -> Self {
        Self {
            last_check: None,
            active: false,
            pid: None,
        }
    }
}

fn gateState() -> &'static Mutex<GateState> {
    static STATE: OnceLock<Mutex<GateState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(GateState::new()))
}

/// Whether this call may raise a dialog, and whether it waits for one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Announce {
    /// Show it and carry on. The trigger's message loop must not stall.
    Detached,
    /// Show it and wait. A one-shot run would exit and take the box with it.
    UntilDismissed,
    /// Say nothing. The startup sweep passes this: a drive left plugged in must
    /// not throw a box at every login.
    Never,
}

impl Announce {
    fn show(self, title: &str, message: &str) {
        match self {
            Announce::Never => {}
            Announce::Detached => drop(alert::warn(title, message)),
            Announce::UntilDismissed => alert::warn(title, message).wait(),
        }
    }
}

/// What became of one volume. Returned as well as logged, because the Windows
/// trigger debounces on it.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The launcher was spawned. The listener does not wait for it.
    Launched,
    /// Not a cartridge, or not one this PC trusts. Either way: leave it alone.
    Ignored,
    /// It *was* a trusted cartridge, but starting the launcher failed.
    Failed,
}

/// Verifies the volume at `root` and, if it carries a launcher this listener
/// trusts and can work with, starts it. Returns what happened.
///
/// Every path out logs its reason first — with no console and, on Windows, no
/// window, the log is the only way to answer "why didn't it start?".
pub fn handleVolume(root: &Path, log: &Log, announce: Announce) -> Outcome {
    if shouldSuppressGlobalLaunch(log) {
        return Outcome::Ignored;
    }

    let launcher = match trust::verifyLauncher(root) {
        Ok(launcher) => launcher,
        Err(reason) => {
            // Every refusal shares one line shape because they share one
            // meaning. "no launcher at the volume root" is by far the most
            // common and is not a problem, but it is logged all the same: when
            // a cartridge *isn't* being detected, "we looked at E:\ and found
            // nothing" is the line that proves the trigger fired at all.
            log.line(&format!("{} ignored: {reason}", root.display()));
            if let Some(explanation) = reason.explain() {
                announce.show("Romzeta — cartridge not started", &explanation);
            }
            return Outcome::Ignored;
        }
    };

    // Straight out of the verified signature — nothing was executed to get it.
    let ours = version::own();
    match version::parse(&launcher.version) {
        // Majors differ: both are genuine, and they cannot work together.
        Some(theirs) if theirs.major != ours.major => {
            log.line(&format!(
                "{} ignored: launcher is project version {} and this listener is {} \
                 (signed by the {} key)",
                root.display(),
                theirs.major,
                ours.major,
                launcher.anchor
            ));
            announce.show(
                "Romzeta — cartridge not compatible",
                &format!(
                    "This cartridge's launcher is version {theirs}, but the Romzeta installed \
                     on this PC is version {ours}.\n\n\
                     They share the same signing key, so both are genuine — but the first \
                     number has to match for them to work together. Update whichever is \
                     older.\n\n\
                     Nothing was started."
                ),
            );
            return Outcome::Ignored;
        }
        Some(theirs) => {
            log.line(&format!(
                "{} verified: {} launcher {theirs}, signed by the {} key",
                root.display(),
                trust::LAUNCHER_NAME,
                launcher.anchor
            ));
        }
        None => {
            // Deliberately not fatal. The signature already proved this is our
            // binary, and refusing a genuine launcher over a comment we cannot
            // parse would turn a cosmetic fault into a dead cartridge. A
            // definite mismatch above is different: that is the signature
            // clearly stating something we cannot work with.
            log.line(&format!(
                "{} launcher's signature carries no usable version ({:?}); starting it anyway",
                root.display(),
                launcher.version
            ));
        }
    }

    // Spawned, never waited on: the Windows build has to get straight back to
    // its message loop, and the Linux build has to exit and leave the launcher
    // running. `current_dir` is the volume root because the launcher resolves
    // its content (catalog.json, images/, games/) relative to where it runs.
    match Command::new(&launcher.path).current_dir(root).spawn() {
        Ok(child) => {
            log.line(&format!(
                "{} launched {} (pid {})",
                root.display(),
                launcher.path.display(),
                child.id()
            ));
            Outcome::Launched
        }
        Err(e) => {
            log.line(&format!(
                "{} FAILED to start {}: {e}",
                root.display(),
                launcher.path.display()
            ));
            Outcome::Failed
        }
    }
}

fn shouldSuppressGlobalLaunch(log: &Log) -> bool {
    let mut state = gateState()
        .lock()
        .expect("global lease gate mutex poisoned");
    let now = Instant::now();
    let check_every = Duration::from_millis(PROCESS_CHECK_INTERVAL_MS);

    if let Some(last_check) = state.last_check
        && now.duration_since(last_check) < check_every
    {
        if state.active
            && let Some(pid) = state.pid
        {
            log.line(&format!(
                "launch suppressed: active global lease pid {pid} (cached under {}ms)",
                PROCESS_CHECK_INTERVAL_MS
            ));
        }
        return state.active;
    }

    state.last_check = Some(now);
    let Some(lease) = common::lease::readLease() else {
        state.active = false;
        state.pid = None;
        return false;
    };

    if common::lease::processExists(lease.pid) {
        state.active = true;
        state.pid = Some(lease.pid);
        log.line(&format!(
            "launch suppressed: active global lease pid {} (checked)",
            lease.pid
        ));
        return true;
    }

    state.active = false;
    state.pid = None;
    if let Err(error) = common::lease::clearLease() {
        log.line(&format!("failed clearing stale global lease: {error}"));
    } else {
        log.line(&format!(
            "cleared stale global lease for pid {} after existence check",
            lease.pid
        ));
    }
    false
}
