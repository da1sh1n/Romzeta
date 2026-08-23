// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Reading and writing `config.toml` without disturbing what a person wrote in
//! it. The round trip needs a real file, since what is checked is what ends up
//! on disk.

#![allow(non_snake_case)] // camelCase functions

// ########## THE CONFIG FILE ##########

mod common;

use std::fs;

use common::{Scratch, checks, runTest};
use launcher::config::{ids, load, store};
use launcher::constants::DEFAULT_ORDER_MODE;

const HAND_WRITTEN: &str = "\
# A comment somebody wrote.
show_captions = true

# Another one, about the order.
usage_order = [1, 0]

# And a trailing note.
border_gap = 36
";

#[test]
fn storing_a_key_leaves_the_rest_of_the_file_alone() {
    runTest(|| {
        // The property the whole persistence design rests on. config.toml is
        // mostly prose written for a person; a launcher that reformatted it as a
        // side effect of somebody starting a game would be answering a question
        // nobody asked.
        let scratch = Scratch::new("config-store-preserves");
        fs::write(scratch.join("config.toml"), HAND_WRITTEN).expect("write config");

        store(scratch.path(), "usage_order", ids(&[2, 1, 0]));
        let after = fs::read_to_string(scratch.join("config.toml")).expect("read config");

        let mut proved = checks();
        proved.expect(
            after.contains("usage_order = [2, 1, 0]"),
            "the new order was written",
        );
        proved.expect(
            after.contains("# A comment somebody wrote."),
            "the first comment survived",
        );
        proved.expect(
            after.contains("# Another one, about the order."),
            "the comment above the key survived",
        );
        proved.expect(
            after.contains("# And a trailing note."),
            "the trailing comment survived",
        );
        proved.expect(
            after.contains("show_captions = true"),
            "another key survived",
        );
        proved.expect(after.contains("border_gap = 36"), "and so did the last one");

        // And it is still readable — the check the ones above cannot make on
        // their own, since they only look for text.
        let config = load(scratch.path());
        proved.expect(
            config.usage_order == vec![2, 1, 0],
            "it reads back as written",
        );
        proved.expect(config.show_captions, "and the other settings still load");
        proved.verdict()
    });
}

#[test]
fn storing_a_key_the_file_never_had_appends_it() {
    runTest(|| {
        // A cartridge set up before the setting existed. It has to arrive
        // documented, the same way `syncDefaults` would have introduced it.
        let scratch = Scratch::new("config-store-appends");
        fs::write(scratch.join("config.toml"), "border_gap = 36\n").expect("write config");

        store(scratch.path(), "order_mode", "alphabetic".into());
        let after = fs::read_to_string(scratch.join("config.toml")).expect("read config");

        let mut proved = checks();
        proved.expect(
            after.contains("order_mode = \"alphabetic\""),
            "the new key was appended",
        );
        proved.expect(after.contains("# Cover order:"), "with its documentation");
        proved.expect(
            load(scratch.path()).order_mode == "alphabetic",
            "and it reads back",
        );
        proved.verdict()
    });
}

#[test]
fn a_bad_value_costs_only_that_setting() {
    runTest(|| {
        // The rule `load` is built around, asserted for the two new readers: an
        // unknown mode and a list holding things that aren't ids leave the
        // default in place without taking the rest of the file down with them.
        let scratch = Scratch::new("config-bad-values");
        fs::write(
            scratch.join("config.toml"),
            "order_mode = \"nonsense\"\nusage_order = [0, \"x\", 2]\nshow_captions = true\n",
        )
        .expect("write config");

        let config = load(scratch.path());
        let mut proved = checks();
        proved.expect(
            config.order_mode == DEFAULT_ORDER_MODE,
            "an unknown mode falls back to the default",
        );
        proved.expect(
            config.usage_order == vec![0, 2],
            "the non-id in the list is dropped and the rest kept",
        );
        proved.expect(config.show_captions, "the settings beside them still load");
        proved.verdict()
    });
}
