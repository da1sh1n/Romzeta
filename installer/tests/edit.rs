// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Editing a game already on a cartridge: what changes, and what deliberately
//! does not.

#![allow(non_snake_case)] // camelCase functions

// ########## EDITING A ROW ##########

mod common;

use std::path::PathBuf;

use common::{checks, runTest};
use installer::app::{Details, Edit};
use installer::catalog::Entry;
use installer::detect::Scan;

fn onCartridge() -> Edit {
    let original = Entry {
        name: "Portal 2".into(),
        exe: "games/portal_2/bin/portal2.exe".into(),
        image: "assets/images/portal_2.png".into(),
        steam: true,
    };
    Edit {
        dir: PathBuf::from(r"E:\games\portal_2"),
        slug: "portal_2".into(),
        open: true,
        appid_original: "620".into(),
        details: Details {
            name: original.name.clone(),
            scanning: None,
            scan: Some(Scan {
                candidates: Vec::new(),
                total_bytes: 0,
                file_count: 0,
                cancelled: false,
            }),
            selected: None,
            manual_exe: None,
            exe_fallback: Some(PathBuf::from("bin/portal2.exe")),
            image: None,
            image_warning: None,
            steam: original.steam,
            appid: "620".into(),
            appid_found: None,
        },
        original,
    }
}

#[test]
fn an_untouched_row_changes_nothing() {
    runTest(|| {
        let edit = onCartridge();
        let mut proved = checks();
        proved.expect(!edit.changed(), "nothing is reported as changed");
        proved.expect(
            edit.entry() == edit.original,
            "the entry comes back identical",
        );
        // Nothing to fall back *from*: the exe the catalog names is what the
        // picker reports until the user picks another.
        proved.expect(
            edit.details.exeRelative() == Some(PathBuf::from("bin/portal2.exe")),
            "the catalog's exe is what the picker reports",
        );
        proved.verdict()
    });
}

#[test]
fn a_rename_leaves_every_path_where_it_was() {
    runTest(|| {
        // The whole reason renaming is cheap. Re-slugging the new name would
        // mean moving every file in a folder that can be tens of gigabytes, for
        // a path nobody ever sees.
        let mut edit = onCartridge();
        edit.details.name = "  Portal II  ".into();
        let entry = edit.entry();

        let mut proved = checks();
        proved.expect(edit.changed(), "the rename counts as a change");
        proved.expect(entry.name == "Portal II", "the name is trimmed and kept");
        proved.expect(entry.exe == edit.original.exe, "the exe path is untouched");
        proved.expect(
            entry.image == edit.original.image,
            "and so is the cover path",
        );
        // A rename touches no file, so nothing to rewrite beside the exe.
        proved.expect(edit.appidRewrite().is_none(), "no app id file is rewritten");
        proved.verdict()
    });
}

#[test]
fn a_new_cover_follows_its_own_extension() {
    runTest(|| {
        let mut edit = onCartridge();
        edit.details.image = Some(PathBuf::from(r"C:\art\new.JPG"));

        // Same extension means the same path — the entry is untouched and it is
        // the file underneath that changes, which `changed` has to catch on its
        // own or the new art would never be copied.
        let mut same = onCartridge();
        same.details.image = Some(PathBuf::from(r"C:\art\new.png"));

        let mut proved = checks();
        proved.expect(
            edit.entry().image == "assets/images/portal_2.jpg",
            "a jpg cover is written as a jpg",
        );
        proved.expect(
            same.entry() == same.original,
            "a png over a png leaves the entry alone",
        );
        proved.expect(same.changed(), "but still counts as a change");
        proved.verdict()
    });
}

#[test]
fn the_app_id_file_is_rewritten_only_when_it_would_differ() {
    runTest(|| {
        let mut retyped = onCartridge();
        retyped.details.appid = "400".into();

        // Just ticked: the file may not be there at all yet.
        let mut ticked = onCartridge();
        ticked.original.steam = false;

        // The exe moved, so the file has to appear beside the new one — the old
        // copy is left alone, being one we cannot tell from the game's.
        let mut moved = onCartridge();
        moved.details.manual_exe = Some(PathBuf::from("portal2.exe"));

        // Unticked: the launcher stops starting Steam and no file is touched.
        let mut unticked = onCartridge();
        unticked.details.steam = false;

        let mut proved = checks();
        proved.expect(
            onCartridge().appidRewrite().is_none(),
            "same id and same exe writes nothing",
        );
        proved.expect(
            retyped.appidRewrite() == Some(400),
            "a retyped id is written",
        );
        proved.expect(
            ticked.appidRewrite() == Some(620),
            "a newly ticked box is written",
        );
        proved.expect(
            moved.appidRewrite() == Some(620),
            "a moved exe gets its own copy",
        );
        proved.expect(
            unticked.appidRewrite().is_none(),
            "unticking writes nothing",
        );
        proved.expect(unticked.changed(), "but unticking is still a change");
        proved.expect(!unticked.entry().steam, "and the entry says so");
        proved.verdict()
    });
}
