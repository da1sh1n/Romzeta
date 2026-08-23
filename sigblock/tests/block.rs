// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Putting a signature block into a buffer and taking it back out again.

#![allow(non_snake_case)] // camelCase functions

// ########## THE BLOCK IN MEMORY ##########

mod common;

use common::{checks, runTest, verdict};
use sigblock::constants::{FOOTER_LEN, MAGIC, MAX_BLOCK_LEN};
use sigblock::{attach, isSigned, split};

const EXE: &[u8] = b"MZ\x90\x00 not really a PE, but neither is anything else here";
const SIG: &str = "untrusted comment: signature from romzeta\nRUQf6LRCGA9i53==\n\
                   trusted comment: romzeta-launcher 0.2.0\nAbCd==\n";

/// Whether a buffer still reads as signed, named for the report.
fn signedness(bytes: &[u8]) -> &'static str {
    if isSigned(bytes) {
        "signed"
    } else {
        "unsigned"
    }
}

#[test]
fn round_trips() {
    runTest(|| {
        let signed = attach(EXE, SIG);
        let (payload, signature) = split(&signed);
        let mut proved = checks();
        proved.expect(payload == EXE, "the payload comes back byte for byte");
        proved.expect(
            signature == Some(SIG),
            "the signature comes back as written",
        );
        proved.verdict()
    });
}

#[test]
fn an_ordinary_binary_is_unsigned() {
    runTest(|| {
        let (payload, signature) = split(EXE);
        let mut proved = checks();
        proved.expect(payload == EXE, "an unsigned file is its own payload");
        proved.expect(signature.is_none(), "and carries no signature");
        proved.expect(!isSigned(EXE), "and the probe agrees");
        proved.verdict()
    });
}

#[test]
fn re_signing_replaces_instead_of_nesting() {
    runTest(|| {
        // The trap: if `attach` appended to the already-signed file, the second
        // block's payload would contain the first block, the exe would grow
        // without bound, and verification would be against bytes no linker ever
        // produced.
        const AGAIN: &str = "untrusted comment: a different one\nZZ==\n";
        let once = attach(EXE, SIG);
        let twice = attach(&once, AGAIN);
        let (payload, signature) = split(&twice);

        let mut proved = checks();
        proved.expect(payload == EXE, "the payload is still the original exe");
        proved.expect(
            signature == Some(AGAIN),
            "the second signature replaced the first",
        );
        proved.expect(
            twice.len() < once.len() + SIG.len(),
            "the file did not grow by a whole second block",
        );
        proved.verdict()
    });
}

#[test]
fn a_truncated_block_reads_as_unsigned() {
    runTest(|| {
        let signed = attach(EXE, SIG);
        let mut proved = checks();
        // Losing the footer is exactly what "truncate the last 16 bytes" does.
        proved.expect(
            !isSigned(&signed[..signed.len() - FOOTER_LEN]),
            "a file missing its footer is unsigned",
        );
        // Losing one byte of it is subtler and must fail just as quietly.
        proved.expect(
            !isSigned(&signed[..signed.len() - 1]),
            "so is one missing a single byte of it",
        );
        proved.verdict()
    });
}

#[test]
fn a_short_file_cannot_panic() {
    runTest(|| {
        let mut proved = checks();
        for len in 0..=FOOTER_LEN {
            let (payload, signature) = split(&EXE[..len]);
            proved.expect(payload.len() == len, "a short file is all payload");
            proved.expect(signature.is_none(), "and carries no signature");
        }
        proved.verdict()
    });
}

#[test]
fn a_length_larger_than_the_file_is_refused() {
    runTest(|| {
        // The interesting hostile input: a real magic and a length that would
        // slice before the start of the buffer.
        let mut signed = attach(EXE, SIG);
        let at = signed.len() - FOOTER_LEN;
        signed[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        verdict(signedness(&signed), "unsigned")
    });
}

#[test]
fn a_future_format_is_left_alone() {
    runTest(|| {
        let mut signed = attach(EXE, SIG);
        let at = signed.len() - FOOTER_LEN;
        signed[at + 4..at + 6].copy_from_slice(&2u16.to_le_bytes());
        verdict(signedness(&signed), "unsigned")
    });
}

#[test]
fn a_non_utf8_signature_is_refused() {
    runTest(|| {
        let mut signed = attach(EXE, SIG);
        let at = signed.len() - FOOTER_LEN - SIG.len();
        signed[at] = 0xff;
        verdict(signedness(&signed), "unsigned")
    });
}

#[test]
fn the_magic_alone_is_not_a_block() {
    runTest(|| {
        // Sixteen bytes of coincidence at the end of some unrelated file.
        let mut coincidence = EXE.to_vec();
        coincidence.extend_from_slice(MAGIC);
        verdict(signedness(&coincidence), "unsigned")
    });
}

#[test]
fn an_empty_payload_is_still_addressable() {
    runTest(|| {
        let signed = attach(b"", SIG);
        let (payload, signature) = split(&signed);
        let mut proved = checks();
        proved.expect(payload.is_empty(), "the payload is empty");
        proved.expect(signature == Some(SIG), "the signature is still readable");
        proved.verdict()
    });
}

#[test]
fn an_absurd_block_length_is_refused_before_it_is_allocated() {
    runTest(|| {
        // A footer claiming most of the address space, on a file that is long
        // enough for the claim to pass the bounds check. Without the cap this is
        // a multi-gigabyte allocation asked for by a stranger's disk.
        let mut signed = attach(EXE, SIG);
        let at = signed.len() - FOOTER_LEN;
        signed[at..at + 4].copy_from_slice(&(MAX_BLOCK_LEN + 1).to_le_bytes());
        verdict(signedness(&signed), "unsigned")
    });
}
