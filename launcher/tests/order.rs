// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! The order the covers appear in, and what happens to a list somebody edited.

#![allow(non_snake_case)] // camelCase functions

// ########## COVER ORDER ##########

mod common;

use common::{checks, runTest, verdict};
use launcher::order::{normalize, promote};

/// An id list as the report prints it: `2, 0, 1, 3`.
fn listed(ids: &[usize]) -> String {
    if ids.is_empty() {
        return "nothing".to_owned();
    }
    ids.iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn an_empty_list_is_plain_catalog_order() {
    runTest(|| {
        // The state every cartridge starts in: nothing played, nothing
        // arranged, so the covers appear the way their author listed them.
        verdict(listed(&normalize(&[], 4)), "0, 1, 2, 3")
    });
}

#[test]
fn a_partial_list_keeps_its_order_and_gains_the_rest() {
    runTest(|| {
        // What a cartridge looks like after one game has been played, and what
        // adding a fourth game to a three-game cartridge leaves behind: the
        // newcomer lands at the end rather than invalidating the list.
        let mut proved = checks();
        proved.expect(
            listed(&normalize(&[2], 4)) == "2, 0, 1, 3",
            "one played game leads",
        );
        proved.expect(
            listed(&normalize(&[3, 1], 4)) == "3, 1, 0, 2",
            "two keep their order and the rest follow",
        );
        proved.verdict()
    });
}

#[test]
fn out_of_range_ids_are_dropped() {
    runTest(|| {
        // A hand-edited list, or one written when the cartridge held more games
        // than it does now. The survivors keep their order.
        verdict(listed(&normalize(&[9, 1, 400], 3)), "1, 0, 2")
    });
}

#[test]
fn repeats_count_only_the_first_time() {
    runTest(|| {
        // Without this the result would be longer than the catalog and the
        // duplicate would be drawn twice.
        verdict(listed(&normalize(&[1, 1, 0, 1], 3)), "1, 0, 2")
    });
}

#[test]
fn an_empty_catalog_normalizes_to_nothing() {
    runTest(|| verdict(listed(&normalize(&[0, 1], 0)), "nothing"));
}

#[test]
fn promoting_moves_one_id_and_disturbs_nothing_else() {
    runTest(|| {
        let mut proved = checks();
        proved.expect(
            listed(&promote(&[0, 1, 2, 3], 4, 2)) == "2, 0, 1, 3",
            "the promoted id moves to the front",
        );
        // Already first: still a no-op rather than a shuffle.
        proved.expect(
            listed(&promote(&[2, 0, 1, 3], 4, 2)) == "2, 0, 1, 3",
            "promoting the first one changes nothing",
        );
        proved.verdict()
    });
}

#[test]
fn promoting_repairs_the_list_it_promotes_into() {
    runTest(|| {
        // The realistic case: a config somebody edited badly, and then a game
        // was played. The write that follows must not carry the mess forward.
        verdict(listed(&promote(&[7, 1, 1], 3, 0)), "0, 1, 2")
    });
}

#[test]
fn promoting_an_id_that_isnt_there_still_normalizes() {
    runTest(|| {
        // `id` out of range can only come from a bug, and the answer is the
        // order unchanged — not a panic, and not an id in the file that names
        // no game.
        verdict(listed(&promote(&[2, 0, 1], 3, 9)), "2, 0, 1")
    });
}
