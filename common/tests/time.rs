// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! The UTC clock every Romzeta program stamps its log lines with.

#![allow(non_snake_case)] // camelCase functions

// ########## UTC DATE AND TIME ##########

mod common;

use ::common::time::{civilFromDays, timestamp, today};
use common::{checks, runTest};

#[test]
fn known_dates_round_trip() {
    runTest(|| {
        // Day 0 is the epoch itself, and the three after it are the boundaries
        // the March-based shift is most likely to get wrong.
        let mut proved = checks();
        proved.expect(civilFromDays(0) == (1970, 1, 1), "day 0 is 1970-01-01");
        proved.expect(civilFromDays(59) == (1970, 3, 1), "day 59 is 1970-03-01");
        // 2000 was a leap year (divisible by 400) where 1900 was not.
        proved.expect(
            civilFromDays(11_016) == (2000, 2, 29),
            "day 11016 is 2000-02-29",
        );
        proved.expect(civilFromDays(-1) == (1969, 12, 31), "day -1 is 1969-12-31");
        proved.verdict()
    });
}

#[test]
fn the_printed_shapes_are_fixed_width() {
    runTest(|| {
        // The listener greps these out of a log by eye, and `xtask verify`
        // prints the date beside a filename — both want a column that lines up.
        let stamp = timestamp();
        let mut proved = checks();
        proved.expect(stamp.len() == 20, "a timestamp is 20 characters");
        proved.expect(stamp.ends_with('Z'), "a timestamp ends in Z");
        proved.expect(today().len() == 10, "a date is 10 characters");
        // Same clock, so the date half of one is the whole of the other unless
        // the two calls straddle midnight.
        proved.expect(
            stamp[..10] == today()[..10],
            "a timestamp opens with today's date",
        );
        proved.verdict()
    });
}
