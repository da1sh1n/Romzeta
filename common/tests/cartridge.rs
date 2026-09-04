// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Which files on the cartridge a catalog entry may claim, and the command line
//! the launcher hands the keeper.

#![allow(non_snake_case)] // camelCase functions

// ########## THE CARTRIDGE LAYOUT ##########

mod common;

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use ::common::cartridge::{
    KeeperArgs, exeRelative, gameDir, imageFile, isContained, parseKeeperArgs, slug, slugOf,
    uniqueSlug,
};
use common::{checks, runTest};

#[test]
fn a_path_that_escapes_is_refused() {
    runTest(|| {
        // The one check standing between a hand-edited catalog.json and
        // `..\..\Windows`, so it is asked in every shape that reaches it.
        let mut proved = checks();
        proved.expect(
            isContained("games/celeste/celeste.exe"),
            "a plain relative path is contained",
        );
        proved.expect(
            isContained("./assets/images/celeste.png"),
            "a leading ./ is contained",
        );
        proved.expect(
            !isContained("../../Windows/System32"),
            "a leading .. is refused",
        );
        proved.expect(
            !isContained("games/../../Windows/notepad.exe"),
            "a .. further along is refused",
        );
        proved.expect(!isContained("/etc/passwd"), "an absolute path is refused");
        proved.expect(!isContained(""), "a path naming nothing is refused");
        // A drive letter or UNC root is only a prefix on Windows; elsewhere the
        // same string is one ordinary filename that stays inside the root.
        if cfg!(windows) {
            proved.expect(
                !isContained(r"C:\Windows\System32\cmd.exe"),
                "a drive prefix is refused",
            );
            proved.expect(
                !isContained(r"\\server\share\evil.exe"),
                "a UNC prefix is refused",
            );
        }
        proved.verdict()
    });
}

#[test]
fn only_a_full_games_path_names_a_folder() {
    runTest(|| {
        // `gameDir` is what remove deletes, so anything shallower than
        // games/<slug>/<exe> has to name nothing rather than name games/ itself.
        let root = Path::new(r"E:\");
        let celeste = root.join("games").join("celeste");

        let mut proved = checks();
        proved.expect(
            gameDir(root, "games/celeste/celeste.exe") == Some(celeste.clone()),
            "games/<slug>/<exe> names its folder",
        );
        proved.expect(
            gameDir(root, "games/celeste/bin/celeste.exe") == Some(celeste),
            "an exe nested deeper names the same folder",
        );
        proved.expect(
            gameDir(root, "games/celeste.exe").is_none(),
            "a two-part path names no folder",
        );
        proved.expect(
            gameDir(root, "other/celeste/celeste.exe").is_none(),
            "a path outside games/ names no folder",
        );
        proved.expect(
            gameDir(root, "games/../../Windows/notepad.exe").is_none(),
            "an escaping path names no folder",
        );
        proved.expect(
            slugOf("games/celeste/bin/celeste.exe").as_deref() == Some("celeste"),
            "the slug is read back out of the exe path",
        );
        proved.expect(
            exeRelative("games/celeste/bin/celeste.exe")
                == Some(PathBuf::from("bin").join("celeste.exe")),
            "the exe comes back relative to its own folder",
        );
        proved.expect(
            imageFile(root, "assets/images/celeste.png")
                == Some(root.join("assets").join("images").join("celeste.png")),
            "a cover resolves under the root",
        );
        proved.expect(
            imageFile(root, "../cover.png").is_none(),
            "an escaping cover resolves to nothing",
        );
        proved.verdict()
    });
}

#[test]
fn slugs_are_safe_and_disambiguated() {
    runTest(|| {
        // A slug has to survive a JSON file, a webview fetch and FAT32, so
        // nothing but ASCII alphanumerics and `_` may come out of it.
        let mut proved = checks();
        proved.expect(
            slug("Baldur's Gate 3") == "baldur_s_gate_3",
            "punctuation and spaces collapse to _",
        );
        proved.expect(
            slug("  Celeste\u{2122}  ") == "celeste",
            "leading and trailing filler is trimmed",
        );
        proved.expect(
            slug("\u{2122}") == "game",
            "a name with nothing usable still yields a name",
        );

        // Two differently named games that squash to the same slug: the second
        // must not be handed the first one's folder.
        let mut taken = HashSet::new();
        let first = uniqueSlug("Game: II", &mut taken);
        let second = uniqueSlug("Game II", &mut taken);
        proved.expect(
            first == "game_ii",
            "the first of two collisions keeps the slug",
        );
        proved.expect(second == "game_ii_2", "the second is suffixed");
        proved.verdict()
    });
}

#[test]
fn keeper_args_round_trip_through_argv() {
    runTest(|| {
        // Renaming a flag used to be silent: the keeper started, recognised
        // nothing and returned. Both sides now spell it in exactly one place.
        let with_playtime = KeeperArgs {
            pid: 4242,
            base_dir: PathBuf::from(r"E:\"),
            playtime_path: Some(PathBuf::from(r"E:\games\celeste\counter.txt")),
        };
        let without = KeeperArgs {
            playtime_path: None,
            ..with_playtime.clone()
        };

        // Built by dropping halves of a real argv, so the flags stay unspelled
        // here too.
        let mut no_pid = with_playtime.toArgv();
        no_pid.drain(..2);
        let no_base: Vec<OsString> = with_playtime.toArgv().into_iter().take(2).collect();

        let mut proved = checks();
        proved.expect(
            parseKeeperArgs(with_playtime.toArgv()).as_ref() == Some(&with_playtime),
            "every field survives argv",
        );
        proved.expect(
            parseKeeperArgs(without.toArgv()).as_ref() == Some(&without),
            "an absent playtime path survives too",
        );
        proved.expect(
            without.toArgv().len() == 4,
            "no playtime path means no flag for it",
        );
        proved.expect(
            parseKeeperArgs(no_pid).is_none(),
            "a missing pid is refused",
        );
        proved.expect(
            parseKeeperArgs(no_base).is_none(),
            "a missing base directory is refused",
        );
        proved.verdict()
    });
}
