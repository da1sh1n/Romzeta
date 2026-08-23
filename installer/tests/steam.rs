// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! The Steam app id: reading one, finding one, and where the file naming it
//! has to land.

#![allow(non_snake_case)] // camelCase functions

// ########## STEAM APP IDS ##########

mod common;

use std::path::{Path, PathBuf};

use common::{checks, runTest};
use installer::app::Details;
use installer::detect::Scan;
use installer::steam::{appidFileIn, manifest, parse};

/// A game past every earlier blocker, so what the app id does to `blocker` is
/// the only thing under test.
fn ready(appid: &str, steam: bool) -> Details {
    Details {
        name: "Portal 2".into(),
        scanning: None,
        scan: Some(Scan {
            candidates: Vec::new(),
            total_bytes: 0,
            file_count: 0,
            cancelled: false,
        }),
        selected: None,
        manual_exe: Some(PathBuf::from("portal2.exe")),
        exe_fallback: None,
        image: Some(PathBuf::from(r"C:\art\cover.png")),
        image_warning: None,
        steam,
        appid: appid.into(),
        appid_found: None,
    }
}

#[test]
fn an_app_id_is_digits_and_not_zero() {
    runTest(|| {
        let mut proved = checks();
        proved.expect(parse("620") == Some(620), "a plain id parses");
        // Whatever a file or a paste brought along with it.
        proved.expect(
            parse(" 620\r\n") == Some(620),
            "surrounding whitespace is ignored",
        );
        proved.expect(parse("").is_none(), "an empty string is not an id");
        proved.expect(parse("abc").is_none(), "nor a word");
        proved.expect(
            parse("620a").is_none(),
            "nor digits with a letter after them",
        );
        // What an empty or malformed manifest would otherwise parse to.
        proved.expect(parse("0").is_none(), "nor zero");
        proved.verdict()
    });
}

#[test]
fn a_manifest_gives_up_its_id_and_folder() {
    runTest(|| {
        let acf = "\"AppState\"\n\
                   {\n\
                   \t\"appid\"\t\t\"620\"\n\
                   \t\"universe\"\t\t\"1\"\n\
                   \t\"name\"\t\t\"Portal 2\"\n\
                   \t\"installdir\"\t\t\"Portal 2\"\n\
                   }\n";
        let mut proved = checks();
        proved.expect(
            manifest(acf) == Some((620, "Portal 2".into())),
            "a whole manifest gives up both",
        );
        // Half a manifest is no answer at all — the folder is what says this
        // manifest is the right one out of a library holding fifty.
        proved.expect(
            manifest("\t\"appid\"\t\t\"620\"\n").is_none(),
            "an id with no folder is not an answer",
        );
        proved.expect(manifest("").is_none(), "nor is an empty file");
        proved.verdict()
    });
}

#[test]
fn the_id_file_lands_beside_the_exe() {
    runTest(|| {
        // The rule that matters: steam_api.dll reads the file next to the module
        // it is loaded into, so a nested exe does not get one at the game
        // folder's root.
        let destination = Path::new(r"E:\games\portal2");
        let mut proved = checks();
        proved.expect(
            appidFileIn(destination, Path::new("bin/portal2.exe"))
                == destination.join("bin").join("steam_appid.txt"),
            "a nested exe gets one in its own folder",
        );
        proved.expect(
            appidFileIn(destination, Path::new("portal2.exe"))
                == destination.join("steam_appid.txt"),
            "a top-level exe gets one beside itself",
        );
        proved.verdict()
    });
}

#[test]
fn a_ticked_steam_box_needs_an_id() {
    runTest(|| {
        let blocker = |appid, steam| {
            ready(appid, steam)
                .blocker(true)
                .unwrap_or_else(|| "nothing".to_owned())
        };
        let mut proved = checks();
        proved.expect(
            blocker("", true) == "needs a Steam app id",
            "ticked with no id is blocked",
        );
        proved.expect(
            blocker("not a number", true) == "has a Steam app id that isn't a number",
            "ticked with a word is blocked",
        );
        proved.expect(
            blocker("620", true) == "nothing",
            "ticked with an id is not",
        );
        // Unticked, the field is nobody's business either way.
        proved.expect(
            blocker("", false) == "nothing",
            "unticked with no id is fine",
        );
        proved.expect(
            blocker("nonsense", false) == "nothing",
            "unticked with nonsense in the box is too",
        );
        proved.verdict()
    });
}
