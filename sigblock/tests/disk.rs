// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Finding the same block by seeking through a file instead of holding all of
//! it in memory.

#![allow(non_snake_case)] // camelCase functions

// ########## THE BLOCK ON DISK ##########

mod common;

use std::io::Cursor;

use common::{checks, runTest};
use sigblock::constants::{FOOTER_LEN, MAX_BLOCK_LEN};
use sigblock::{attach, hasBlock, isSigned, readBlock};

const EXE: &[u8] = b"MZ\x90\x00 not really a PE, but neither is anything else here";
const SIG: &str = "untrusted comment: signature from romzeta\nRUQf6LRCGA9i53==\n\
                   trusted comment: romzeta-launcher 0.2.0\nAbCd==\n";

#[test]
fn the_file_reader_finds_the_same_block() {
    runTest(|| {
        let mut file = Cursor::new(attach(EXE, SIG));
        let mut proved = checks();
        proved.expect(
            readBlock(&mut file).expect("no I/O error") == Some(SIG.into()),
            "the whole signature is read back",
        );
        proved.expect(
            hasBlock(&mut file).expect("no I/O error"),
            "and the probe finds it too",
        );
        proved.verdict()
    });
}

#[test]
fn the_probe_agrees_with_split_on_every_bad_footer() {
    runTest(|| {
        let signed = attach(EXE, SIG);
        let at = signed.len() - FOOTER_LEN;

        let mut truncated = signed.clone();
        truncated.truncate(at);
        let mut wrong_len = signed.clone();
        wrong_len[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let mut wrong_format = signed.clone();
        wrong_format[at + 4..at + 6].copy_from_slice(&2u16.to_le_bytes());

        let mut proved = checks();
        for bad in [EXE.to_vec(), truncated, wrong_len, wrong_format] {
            let mut file = Cursor::new(bad.clone());
            proved.expect(
                hasBlock(&mut file).expect("no I/O error") == isSigned(&bad),
                "the probe and the split reach the same answer",
            );
        }
        proved.verdict()
    });
}

#[test]
fn the_probe_passes_a_block_only_the_full_read_can_refuse() {
    runTest(|| {
        // A good footer over bytes that are not UTF-8. `hasBlock` reads the
        // footer and nothing else, so it says yes; the callers that use it as a
        // cheap refusal go on to read the file and `split` gives the real
        // answer. Being *less* strict is the only safe direction for a probe to
        // differ in.
        let mut not_utf8 = attach(EXE, SIG);
        let at = not_utf8.len() - FOOTER_LEN - SIG.len();
        not_utf8[at] = 0xff;
        let mut file = Cursor::new(not_utf8.clone());

        let mut proved = checks();
        proved.expect(
            hasBlock(&mut file).expect("no I/O error"),
            "the cheap probe accepts it",
        );
        proved.expect(
            readBlock(&mut file).expect("no I/O error").is_none(),
            "the full read refuses it",
        );
        proved.expect(!isSigned(&not_utf8), "and so does the in-memory split");
        proved.verdict()
    });
}

#[test]
fn an_absurd_block_length_is_refused_on_disk_too() {
    runTest(|| {
        let mut signed = attach(EXE, SIG);
        let at = signed.len() - FOOTER_LEN;
        signed[at..at + 4].copy_from_slice(&(MAX_BLOCK_LEN + 1).to_le_bytes());
        let mut file = Cursor::new(signed);

        let mut proved = checks();
        proved.expect(
            !hasBlock(&mut file).expect("no I/O error"),
            "the probe refuses a length past the cap",
        );
        proved.verdict()
    });
}

#[test]
fn a_short_file_cannot_panic_on_disk_either() {
    runTest(|| {
        let mut proved = checks();
        for len in 0..=FOOTER_LEN {
            let mut file = Cursor::new(EXE[..len].to_vec());
            proved.expect(
                readBlock(&mut file).expect("no I/O error").is_none(),
                "a file too short to hold a footer reads as unsigned",
            );
        }
        proved.verdict()
    });
}
