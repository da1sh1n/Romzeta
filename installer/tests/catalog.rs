// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! What goes into `catalog.json`, and which files on the cartridge an entry
//! claims as its own.

#![allow(non_snake_case)] // camelCase functions

// ########## THE CATALOG ##########

mod common;

use std::path::{Path, PathBuf};

use ::common::cartridge::{exeRelative, gameDir, imageFile, slug, slugOf};
use common::{checks, runTest};
use installer::catalog::{Entry, imagePath};

/// An entry with everything filled in, for tests that vary one field.
fn anEntry() -> Entry {
    Entry {
        name: "celeste".into(),
        exe: "games/celeste/celeste.exe".into(),
        image: "assets/images/celeste.png".into(),
        steam: false,
    }
}

#[test]
fn the_steam_flag_is_written_only_when_it_is_set() {
    runTest(|| {
        // Both halves are the contract with the launcher's `Game`. Writing
        // `"steam": false` into every entry would add a key to catalogs that
        // never needed one, and failing to read a catalog that lacks it would
        // break every cartridge written before the checkbox existed.
        let plain = anEntry();
        let json = serde_json::to_string(&plain).expect("serialize");
        let ticked = serde_json::to_string(&Entry {
            steam: true,
            ..plain.clone()
        })
        .expect("serialize");

        let older = r#"[{"name":"celeste","exe":"games/celeste/celeste.exe",
                        "image":"assets/images/celeste.png"}]"#;
        let read: Vec<Entry> = serde_json::from_str(older).expect("deserialize");

        let mut proved = checks();
        proved.expect(!json.contains("steam"), "an unticked entry writes no key");
        proved.expect(ticked.contains(r#""steam":true"#), "a ticked one writes it");
        proved.expect(read == vec![plain], "a catalog without the key still reads");
        proved.verdict()
    });
}

#[test]
fn covers_are_written_under_assets() {
    runTest(|| {
        // The path this produces goes into catalog.json and is what the launcher
        // asks its app:// protocol for, so the prefix is a contract between the
        // two crates rather than a detail of this one.
        let mut proved = checks();
        proved.expect(
            imagePath("bg3", Path::new(r"C:\art\cover.png")) == "assets/images/bg3.png",
            "a cover lands under assets/images",
        );
        // The source extension is kept: the webview goes by content, not name,
        // and renaming a jpg to png only makes the cartridge harder to read.
        proved.expect(
            imagePath("celeste", Path::new(r"C:\art\cover.JPG")) == "assets/images/celeste.jpg",
            "the source extension is kept and lowercased",
        );
        proved.verdict()
    });
}

#[test]
fn a_cover_written_by_an_older_installer_still_resolves() {
    runTest(|| {
        // Cartridges made before covers moved under assets/ say `images/...` in
        // their catalog. Nothing migrates them — the launcher serves both
        // prefixes — so removal has to keep finding the file too, or editing an
        // old cartridge would leave its art behind.
        let root = Path::new(r"E:\");
        let legacy = Entry {
            name: "bg3".into(),
            exe: "games/bg3/bg3.exe".into(),
            image: "images/bg3.png".into(),
            steam: false,
        };
        let current = Entry {
            image: "assets/images/bg3.png".into(),
            ..legacy.clone()
        };

        let mut proved = checks();
        proved.expect(
            imageFile(root, &legacy.image) == Some(root.join("images").join("bg3.png")),
            "an images/ cover still resolves",
        );
        proved.expect(
            imageFile(root, &current.image)
                == Some(root.join("assets").join("images").join("bg3.png")),
            "and so does an assets/images/ one",
        );
        proved.verdict()
    });
}

#[test]
fn an_entry_says_which_folder_and_exe_are_its_own() {
    runTest(|| {
        let nested = Entry {
            name: "Portal 2".into(),
            exe: "games/portal_2/bin/portal2.exe".into(),
            image: "assets/images/portal_2.png".into(),
            steam: true,
        };
        // The slug is read off the path, never re-derived from the name — the
        // two part company the moment a game is renamed, and the folder is what
        // the files are actually in.
        let renamed = Entry {
            name: "Something Else Entirely".into(),
            ..nested.clone()
        };

        let mut proved = checks();
        proved.expect(
            slugOf(&nested.exe).as_deref() == Some("portal_2"),
            "the slug is the folder",
        );
        proved.expect(
            exeRelative(&nested.exe) == Some(PathBuf::from("bin/portal2.exe")),
            "the exe is named relative to that folder",
        );
        proved.expect(
            slugOf(&renamed.exe).as_deref() == Some("portal_2"),
            "renaming the game does not move its folder",
        );
        // Anything not under games/<slug>/<file> names no folder of its own,
        // which is the same refusal `gameDir` makes.
        for exe in ["games/loose.exe", "elsewhere/x/y.exe", "games/portal_2"] {
            proved.expect(
                exeRelative(exe).is_none(),
                &format!("{exe} names no folder of its own"),
            );
        }
        proved.verdict()
    });
}

#[test]
fn slugs_are_safe_on_any_filesystem() {
    runTest(|| {
        // A game's name becomes a folder name, so this is the point where a name
        // someone typed turns into a path.
        let mut proved = checks();
        for (name, expected) in [
            ("Baldur's Gate 3", "baldur_s_gate_3"),
            ("Hollow Knight", "hollow_knight"),
            ("  NieR:Automata™  ", "nier_automata"),
            ("!!!", "game"),
            ("", "game"),
        ] {
            proved.expect(
                slug(name) == expected,
                &format!("{name:?} becomes {expected}"),
            );
        }
        proved.verdict()
    });
}

#[test]
fn removal_paths_stay_inside_the_cartridge() {
    runTest(|| {
        let root = Path::new(r"E:\");
        let escape = Entry {
            name: "evil".into(),
            exe: "../../Windows/System32/cmd.exe".into(),
            image: "../../Windows/x.png".into(),
            steam: false,
        };
        let ok = Entry {
            name: "bg3".into(),
            exe: "games/bg3/bin/bg3.exe".into(),
            image: "images/bg3.png".into(),
            steam: false,
        };
        // An exe sitting directly in games/ names no folder to delete.
        let shallow = Entry {
            name: "loose".into(),
            exe: "games/loose.exe".into(),
            image: "images/loose.png".into(),
            steam: false,
        };

        let mut proved = checks();
        proved.expect(
            gameDir(root, &escape.exe).is_none(),
            "an escaping exe names no folder",
        );
        proved.expect(
            imageFile(root, &escape.image).is_none(),
            "nor does an escaping cover",
        );
        proved.expect(
            gameDir(root, &ok.exe) == Some(root.join("games").join("bg3")),
            "an ordinary entry names its own folder",
        );
        proved.expect(
            imageFile(root, &ok.image) == Some(root.join("images").join("bg3.png")),
            "and its own cover",
        );
        proved.expect(
            gameDir(root, &shallow.exe).is_none(),
            "an exe loose in games/ names no folder",
        );
        proved.verdict()
    });
}
