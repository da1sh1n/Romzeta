// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! What the wizard is drawn in, and what happens when the system font cannot
//! be found.

#![allow(non_snake_case)] // camelCase functions

// ########## FONTS ##########

mod common;

use common::{checks, runTest};
use egui::FontFamily;
use installer::constants::FALLBACK;
use installer::font::definitions;

/// epaint panics — `FontFamily::… is not bound to any fonts` — the first time a
/// family with nothing behind it is used, and it does that lazily. Nothing in
/// this program asks for monospace today, so the crash would arrive the day
/// something did.
#[test]
fn every_family_has_something_behind_it() {
    runTest(|| {
        let fonts = definitions();
        let mut proved = checks();
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            let chain = fonts
                .families
                .get(&family)
                .unwrap_or_else(|| panic!("{family:?} was not bound at all"));
            proved.expect(
                !chain.is_empty(),
                &format!("{family:?} is bound to something"),
            );
            for name in chain {
                proved.expect(
                    fonts.font_data.contains_key(name),
                    &format!("{family:?}'s {name:?} has font data"),
                );
            }
        }
        proved.verdict()
    });
}

/// The fallback is last, so a glyph the system font lacks still draws.
#[test]
fn the_fallback_is_always_there_and_always_last() {
    runTest(|| {
        let fonts = definitions();
        let mut proved = checks();
        for chain in fonts.families.values() {
            proved.expect(
                chain.last().map(String::as_str) == Some(FALLBACK),
                "every family ends in the fallback",
            );
        }
        proved.verdict()
    });
}

/// The one thing looking at the window cannot tell you. If the face lookup, the
/// registry walk or the file read breaks, the wizard still comes up — drawn in
/// Ubuntu-Light, which is not what this machine uses anywhere else.
///
/// Like the volume tests, this one asserts against the real machine.
#[test]
#[cfg(windows)]
fn the_system_font_is_the_one_actually_used() {
    runTest(|| {
        use common::verdict;
        use installer::constants::SYSTEM;

        let fonts = definitions();
        let chain = &fonts.families[&FontFamily::Proportional];
        verdict(
            chain
                .first()
                .cloned()
                .unwrap_or_else(|| "nothing".to_owned()),
            SYSTEM,
        )
    });
}
