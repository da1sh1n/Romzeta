// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Spawns a game's executable and decides whether it actually came up — via
//! `WaitForInputIdle` on Windows, or "still alive a moment later" elsewhere —
//! then phrases the outcome for the page.

// ########## STARTING A GAME ##########

use std::path::Path;
use std::process::{Child, Command};
use std::thread;
use std::time::Instant;

use crate::catalog::{self, Game};
use crate::constants::*;
use crate::log;
use crate::steam;

/// What became of one launch, as far as the player is concerned.
pub enum Outcome {
    /// The game is up. The launcher's cue to minimize itself.
    Started(u32),
    /// It isn't, and this is the one short line to put under its cover. The
    /// long version is already in `logs/launcher.log`.
    Failed(String),
}

/// Everything one launch involves, from the click to the verdict.
///
/// Call on a worker thread. All three stages block: a game flagged `steam` waits
/// up to [`STEAM_WAIT`] for the client, and `supervise` waits up to
/// [`WINDOW_WAIT_MS`] for a window.
pub fn run(base: &Path, game: &Game, index: usize, show_console_window: bool) -> Outcome {
    // Before the spawn, not after: the game's DRM asks for the client during its
    // own startup, so a client that arrives late is a client that arrived too
    // late.
    if game.steam
        && let Err(message) = steam::ensureRunning(base)
    {
        return Outcome::Failed(message);
    }
    match spawn(base, game, index, show_console_window) {
        Ok(child) => supervise(base, game, child),
        Err(message) => Outcome::Failed(message),
    }
}

/// Starts `game`'s exe and hands back the running `Child`, which the caller
/// should pass to `supervise` on a worker thread — everything after this
/// blocks. `show_console_window` is `config.toml`'s knob of the same name.
///
/// The working directory is the exe's **own folder**, not the cartridge root:
/// games overwhelmingly resolve assets relative to themselves, and one started
/// from the wrong cwd fails in ways that look like corruption.
fn spawn(
    base: &Path,
    game: &Game,
    index: usize,
    show_console_window: bool,
) -> Result<Child, String> {
    // Re-checked rather than trusted from the catalog load: `game` reaches here
    // by index over IPC (see `crate::ui`), and re-deriving the containment
    // check at the one place that actually spawns something is cheaper than a
    // second path from an untrusted `catalog.json` entry to a running process.
    if !catalog::isContained(&game.exe) {
        log::logLine(
            base,
            &format!("REFUSED {}: exe path escapes the cartridge", game.name),
        );
        return Err("Failed to start — game files missing".to_string());
    }

    let exe = base.join(&game.exe);
    log::logLine(
        base,
        &format!("launching {} ({})", game.name, exe.display()),
    );

    // Checked again here even though the catalog was screened at startup: the
    // cartridge is removable, and the file may be gone since.
    if !exe.is_file() {
        log::logLine(base, &format!("FAILED {}: no such file", exe.display()));
        return Err("Failed to start — game files missing".to_string());
    }

    // The exe's parent always exists here (base.join of a relative path), but
    // fall back to the cartridge root rather than refusing to launch.
    let workdir = exe.parent().unwrap_or(base).to_path_buf();
    let (stdout, stderr) = log::gameOutput(base, game, index);

    let mut command = Command::new(&exe);
    command.current_dir(&workdir).stdout(stdout).stderr(stderr);
    if !show_console_window {
        suppressConsoleWindow(&mut command);
    }

    match command.spawn() {
        Ok(child) => {
            log::logLine(
                base,
                &format!("started pid {} in {}", child.id(), workdir.display()),
            );
            Ok(child)
        }
        Err(e) => {
            log::logLine(base, &format!("FAILED {}: {e}", exe.display()));
            Err(format!("Failed to start — {}", shortReason(&e)))
        }
    }
}

/// Blocks until the game is up (or clearly isn't). Call on a worker thread —
/// never on the UI thread, which has a window to keep repainting.
fn supervise(base: &Path, game: &Game, mut child: Child) -> Outcome {
    let pid = child.id();
    match waitForWindow(&child) {
        // A window came up — but hold on briefly before believing it. A game
        // that flashes an error box and quits satisfies WaitForInputIdle just
        // as well as one that is genuinely running, and so does one that was
        // already dead when it was asked.
        Window::Ready => match outlives(&mut child, READY_CONFIRM) {
            None => {
                log::logLine(base, &format!("{} is up", game.name));
                Outcome::Started(pid)
            }
            Some(status) => finishedEarly(base, game, status, pid),
        },
        Window::TimedOut => {
            // Not a failure. Saying otherwise would punish slow games, and the
            // player can see for themselves whether one is coming up.
            log::logLine(
                base,
                &format!(
                    "{} has no window yet after {WINDOW_WAIT_MS}ms; assuming slow start",
                    game.name
                ),
            );
            Outcome::Started(pid)
        }
        // WaitForInputIdle refuses non-GUI processes, and there's nothing to
        // ask on other platforms. Fall back to plain survival.
        Window::Unsupported => match outlives(&mut child, LIVENESS_GRACE) {
            None => {
                log::logLine(base, &format!("{} is running", game.name));
                Outcome::Started(pid)
            }
            Some(status) => finishedEarly(base, game, status, pid),
        },
    }
}

/// A game that was gone before we ever reported it as started.
fn finishedEarly(base: &Path, game: &Game, status: std::process::ExitStatus, pid: u32) -> Outcome {
    // Exit code 0 in the first couple of seconds is odd but not an error — a
    // stub that hands off to another process does exactly this.
    if status.success() {
        log::logLine(base, &format!("{} exited immediately, cleanly", game.name));
        return Outcome::Started(pid);
    }
    // The exit code goes to the log, not under the cover: it means nothing to
    // a player, and a line long enough to wrap is worse than a short one.
    log::logLine(
        base,
        &format!("FAILED {}: exited immediately ({status})", game.name),
    );
    Outcome::Failed("Failed to start — closed immediately".to_string())
}

/// `Some(status)` if the process has already exited, `None` if it's running.
/// A wait error is treated as "running": the game is not the thing at fault.
fn exited(child: &mut Child) -> Option<std::process::ExitStatus> {
    child.try_wait().ok().flatten()
}

/// Polls for `window`, returning early with the exit status if the process dies
/// inside it and `None` if it's still alive at the end.
fn outlives(child: &mut Child, window: std::time::Duration) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + window;
    loop {
        if let Some(status) = exited(child) {
            return Some(status);
        }
        if Instant::now() >= deadline {
            return None;
        }
        thread::sleep(LIVENESS_POLL);
    }
}

enum Window {
    /// The process is up and waiting for input — its window exists.
    Ready,
    /// Still no window after [`WINDOW_WAIT_MS`].
    TimedOut,
    /// The question doesn't apply here (console program, or not Windows).
    Unsupported,
}

/// Waits for the process to finish initialising and start pumping messages,
/// which in practice means "its first window is on screen".
#[cfg(windows)]
fn waitForWindow(child: &Child) -> Window {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Threading::WaitForInputIdle;

    // Documented return values. Spelled out rather than imported because
    // WaitForInputIdle's success value (0) has no name of its own.
    const WAIT_TIMEOUT: u32 = 0x0000_0102;

    let handle = child.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE;
    match unsafe { WaitForInputIdle(handle, WINDOW_WAIT_MS) } {
        0 => Window::Ready,
        WAIT_TIMEOUT => Window::TimedOut,
        // WAIT_FAILED — in practice ERROR_NOT_GUI_PROCESS, i.e. a console
        // program. Not an error about the game, just the wrong question.
        _ => Window::Unsupported,
    }
}

#[cfg(not(windows))]
fn waitForWindow(_child: &Child) -> Window {
    Window::Unsupported
}

/// Stops a console-subsystem exe from opening a console window of its own. A
/// GUI game has no console to begin with, so this has no visible effect on
/// one — only a console program (typically a stand-in cover, since a real
/// game is GUI) is ever affected.
#[cfg(windows)]
pub(crate) fn suppressConsoleWindow(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    // windows_sys::Win32::System::Threading::CREATE_NO_WINDOW, spelled out
    // rather than pulled in for one flag.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub(crate) fn suppressConsoleWindow(_command: &mut Command) {}

/// Maps an OS spawn error to something worth showing a player. Anything beyond
/// the two everyday causes points at the log rather than guessing.
fn shortReason(error: &std::io::Error) -> &'static str {
    match error.kind() {
        std::io::ErrorKind::NotFound => "file not found",
        std::io::ErrorKind::PermissionDenied => "access denied",
        _ => "see logs/launcher.log",
    }
}
