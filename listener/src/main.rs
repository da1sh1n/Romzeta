// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

// Entry point. Parses the command line into a `Mode` and dispatches: print a
// version or signature, run the core once against one volume, or hand off to
// the platform trigger. Also resolves the folder the exe and its log live in.
//
// No console window: this waits in the background, it is not a CLI tool.
#![windows_subsystem = "windows"]
#![allow(non_snake_case)] // camelCase functions

mod alert;
mod constants;
mod log;
mod trigger;
mod trust;
mod version;
mod volume;

#[cfg(test)]
mod tests;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use log::Log;

// ########## STARTUP ##########

/// The mode the exe is running in, as determined by the command line.
enum Mode {
    /// Print `x.y.z` and exit.
    Version,
    /// Print this exe's own signature block and the keys it trusts, then exit.
    Signature,
    /// Run the core once against one volume, then exit.
    Check(PathBuf),
    /// Print command line usage and exit. Also what a bare `--check` falls to.
    Help,
    /// Wait for cartridges. The default, and what the login entry starts.
    Trigger,
}

fn main() {
    // So Task Manager groups this under "Romzeta" with the other processes.
    common::aumid::set();

    // Process the command line into a `Mode` and dispatch.
    match mode() {
        Mode::Version => {
            sigblock::cli::attachConsole();
            version::print();
        }
        Mode::Signature => {
            sigblock::cli::attachConsole();
            sigblock::cli::printSignature();
            println!("trusts: {}", trust::anchorNames());
        }
        Mode::Check(root) => {
            let logger: Log = start();
            volume::handleVolume(&root, &logger, volume::Announce::UntilDismissed);
        }
        Mode::Help => {
            sigblock::cli::attachConsole();
            printUsage();
        }
        Mode::Trigger => trigger::run(start()),
    }
}

/// Sets the deployment folder up and opens the log. Shared by the two modes
/// that actually do something.
fn start() -> Log {
    let dir: PathBuf = resolveBaseDir();
    let _ = fs::create_dir_all(&dir);
    Log::open(Some(log::defaultLogPath(&dir)))
}

/// Reads the command line into a `Mode`. `--version`, `--signature` and
/// `--help` return immediately and win over everything else. `--check` with
/// no path following it falls to `Mode::Help` rather than `Mode::Trigger` —
/// a mistyped invocation is a mistake to report, not silence to wait in.
fn mode() -> Mode {
    // `args_os` so a non-UTF-8 argument cannot panic us before we have started.
    let mut args = env::args_os().skip(1);
    let mut check: Option<PathBuf> = None;
    let mut sawCheck = false;

    while let Some(arg) = args.next() {
        if arg == "--version" {
            return Mode::Version;
        }
        if arg == "--signature" {
            return Mode::Signature;
        }
        if arg == "--help" {
            return Mode::Help;
        }
        if arg == "--check" && check.is_none() {
            sawCheck = true;
            check = args.next().map(PathBuf::from);
        }
    }
    match check {
        Some(root) => Mode::Check(root), // `--check` with a path wins over `--help`.
        None if sawCheck => Mode::Help,  // `--check` with no path falls to `--help`.
        None => Mode::Trigger,           // `--check` not present, the default.
    }
}

/// What `--help` prints, and what a bare `--check` with no path falls back to.
fn printUsage() {
    println!("romzeta-listener {}", version::own());
    println!();
    println!("Waits for cartridges and launches their keeper. Run with no");
    println!("arguments to do this; the options below are for diagnostics.");
    println!();
    println!("USAGE:");
    println!("    listener.exe [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --check <PATH>   Run the core once against one volume, then exit.");
    println!("    --signature      Print this exe's signature block and trusted keys.");
    println!("    --version        Print the version and exit.");
    println!("    --help           Print this message and exit.");
}

// ========== The Deployment Folder ==========

/// The folder holding `listener.exe` and its log: the exe's own parent folder.
fn resolveBaseDir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}
