// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Every test in this crate. Inside the crate so they can reach `MAGIC` and
//! `FORMAT` without those becoming public API.
//! Run with `cargo test -p sigblock`.

// ########## SIGNATURE BLOCK TESTS ##########

// `crate::*` because these tests use the private `MAGIC` and `FORMAT` too.
use crate::constants::{FOOTER_LEN, MAX_BLOCK_LEN};
use crate::*;

const EXE: &[u8] = b"MZ\x90\x00 not really a PE, but neither is anything else here";
const SIG: &str = "untrusted comment: signature from romzeta\nRUQf6LRCGA9i53==\n\
                   trusted comment: romzeta-launcher 0.2.0\nAbCd==\n";

#[test]
fn round_trips() {
    let signed = attach(EXE, SIG);
    let (payload, signature) = split(&signed);
    assert_eq!(payload, EXE);
    assert_eq!(signature, Some(SIG));
}

#[test]
fn an_ordinary_binary_is_unsigned() {
    let (payload, signature) = split(EXE);
    assert_eq!(payload, EXE);
    assert_eq!(signature, None);
    assert!(!isSigned(EXE));
}

#[test]
fn re_signing_replaces_instead_of_nesting() {
    // The trap: if `attach` appended to the already-signed file, the second
    // block's payload would contain the first block, the exe would grow
    // without bound, and verification would be against bytes no linker ever
    // produced.
    let once = attach(EXE, SIG);
    let twice = attach(&once, "untrusted comment: a different one\nZZ==\n");
    let (payload, signature) = split(&twice);
    assert_eq!(payload, EXE);
    assert_eq!(
        signature,
        Some("untrusted comment: a different one\nZZ==\n")
    );
    assert!(twice.len() < once.len() + SIG.len());
}

#[test]
fn a_truncated_block_reads_as_unsigned() {
    let signed = attach(EXE, SIG);
    // Losing the footer is exactly what "truncate the last 16 bytes" does.
    assert!(!isSigned(&signed[..signed.len() - FOOTER_LEN]));
    // Losing one byte of it is subtler and must fail just as quietly.
    assert!(!isSigned(&signed[..signed.len() - 1]));
}

#[test]
fn a_short_file_cannot_panic() {
    for len in 0..=FOOTER_LEN {
        let (payload, signature) = split(&EXE[..len]);
        assert_eq!(payload.len(), len);
        assert_eq!(signature, None);
    }
}

#[test]
fn a_length_larger_than_the_file_is_refused() {
    // The interesting hostile input: a real magic and a length that would
    // slice before the start of the buffer.
    let mut signed = attach(EXE, SIG);
    let at = signed.len() - FOOTER_LEN;
    signed[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(!isSigned(&signed));
}

#[test]
fn a_future_format_is_left_alone() {
    let mut signed = attach(EXE, SIG);
    let at = signed.len() - FOOTER_LEN;
    signed[at + 4..at + 6].copy_from_slice(&2u16.to_le_bytes());
    assert!(!isSigned(&signed));
}

#[test]
fn a_non_utf8_signature_is_refused() {
    let mut signed = attach(EXE, SIG);
    let at = signed.len() - FOOTER_LEN - SIG.len();
    signed[at] = 0xff;
    assert!(!isSigned(&signed));
}

#[test]
fn the_magic_alone_is_not_a_block() {
    // Sixteen bytes of coincidence at the end of some unrelated file.
    let mut coincidence = EXE.to_vec();
    coincidence.extend_from_slice(MAGIC);
    assert!(!isSigned(&coincidence));
}

#[test]
fn an_empty_payload_is_still_addressable() {
    let signed = attach(b"", SIG);
    let (payload, signature) = split(&signed);
    assert!(payload.is_empty());
    assert_eq!(signature, Some(SIG));
}

// ========== Reading From Disk ==========

#[test]
fn the_file_reader_finds_the_same_block() {
    let signed = attach(EXE, SIG);
    let mut file = std::io::Cursor::new(signed);
    assert_eq!(
        readBlock(&mut file).expect("no I/O error"),
        Some(SIG.into())
    );
    assert!(hasBlock(&mut file).expect("no I/O error"));
}

#[test]
fn the_probe_agrees_with_split_on_every_bad_footer() {
    let signed = attach(EXE, SIG);
    let at = signed.len() - FOOTER_LEN;

    let mut truncated = signed.clone();
    truncated.truncate(at);
    let mut wrong_len = signed.clone();
    wrong_len[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    let mut wrong_format = signed.clone();
    wrong_format[at + 4..at + 6].copy_from_slice(&2u16.to_le_bytes());

    for bad in [EXE.to_vec(), truncated, wrong_len, wrong_format] {
        let mut file = std::io::Cursor::new(bad.clone());
        assert_eq!(hasBlock(&mut file).expect("no I/O error"), isSigned(&bad));
    }
}

#[test]
fn the_probe_passes_a_block_only_the_full_read_can_refuse() {
    // A good footer over bytes that are not UTF-8. `hasBlock` reads the footer
    // and nothing else, so it says yes; the callers that use it as a cheap
    // refusal go on to read the file and `split` gives the real answer. Being
    // *less* strict is the only safe direction for a probe to differ in.
    let mut not_utf8 = attach(EXE, SIG);
    let at = not_utf8.len() - FOOTER_LEN - SIG.len();
    not_utf8[at] = 0xff;

    let mut file = std::io::Cursor::new(not_utf8.clone());
    assert!(hasBlock(&mut file).expect("no I/O error"));
    assert_eq!(readBlock(&mut file).expect("no I/O error"), None);
    assert!(!isSigned(&not_utf8));
}

#[test]
fn an_absurd_block_length_is_refused_before_it_is_allocated() {
    // A footer claiming most of the address space, on a file that is long
    // enough for the claim to pass the bounds check. Without the cap this is a
    // multi-gigabyte allocation asked for by a stranger's disk.
    let mut signed = attach(EXE, SIG);
    let at = signed.len() - FOOTER_LEN;
    signed[at..at + 4].copy_from_slice(&(MAX_BLOCK_LEN + 1).to_le_bytes());
    assert!(!isSigned(&signed));
    let mut file = std::io::Cursor::new(signed);
    assert!(!hasBlock(&mut file).expect("no I/O error"));
}

#[test]
fn a_short_file_cannot_panic_on_disk_either() {
    for len in 0..=FOOTER_LEN {
        let mut file = std::io::Cursor::new(EXE[..len].to_vec());
        assert_eq!(readBlock(&mut file).expect("no I/O error"), None);
    }
}
