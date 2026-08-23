// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Reading a cover's dimensions out of its bytes, whatever its name says.

#![allow(non_snake_case)] // camelCase functions

// ########## IMAGE HEADERS ##########

mod common;

use common::{checks, runTest, verdict};
use installer::image::parse;

/// The size `parse` found, as the report prints it.
fn size(bytes: &[u8]) -> String {
    parse(bytes).map_or_else(|| "unreadable".to_owned(), |(w, h)| format!("{w}x{h}"))
}

fn pngHeader(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    bytes.extend_from_slice(&13u32.to_be_bytes());
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes
}

fn riff(chunk: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(b"WEBP");
    bytes.extend_from_slice(chunk);
    bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
    bytes.extend_from_slice(body);
    bytes
}

#[test]
fn reads_a_png() {
    runTest(|| verdict(size(&pngHeader(600, 900)), "600x900"));
}

#[test]
fn reads_an_animated_webp() {
    runTest(|| {
        // VP8X — the shape the covers in this project actually are, and the
        // reason the format is sniffed from bytes rather than trusted from the
        // `.png` name they are usually saved under.
        let mut body = vec![0x10, 0, 0, 0]; // flags: has animation
        body.extend_from_slice(&[0x57, 0x02, 0x00]); // 600-1, 24-bit LE
        body.extend_from_slice(&[0x83, 0x03, 0x00]); // 900-1
        verdict(size(&riff(b"VP8X", &body)), "600x900")
    });
}

#[test]
fn reads_a_lossless_webp() {
    runTest(|| {
        let bits: u32 = 599 | (899 << 14);
        let mut body = vec![0x2f];
        body.extend_from_slice(&bits.to_le_bytes());
        body.extend_from_slice(&[0; 8]);
        verdict(size(&riff(b"VP8L", &body)), "600x900")
    });
}

#[test]
fn reads_a_lossy_webp() {
    runTest(|| {
        let mut body = vec![0x00, 0x00, 0x00, 0x9d, 0x01, 0x2a];
        body.extend_from_slice(&600u16.to_le_bytes());
        body.extend_from_slice(&900u16.to_le_bytes());
        body.extend_from_slice(&[0; 8]);
        verdict(size(&riff(b"VP8 ", &body)), "600x900")
    });
}

#[test]
fn reads_a_gif() {
    runTest(|| {
        let mut bytes = b"GIF89a".to_vec();
        bytes.extend_from_slice(&600u16.to_le_bytes());
        bytes.extend_from_slice(&900u16.to_le_bytes());
        verdict(size(&bytes), "600x900")
    });
}

#[test]
fn reads_a_jpeg_behind_a_metadata_segment() {
    runTest(|| {
        let mut bytes = vec![0xff, 0xd8];
        // An APP1 (EXIF) segment the size walk has to step over.
        bytes.extend_from_slice(&[0xff, 0xe1, 0x00, 0x10]);
        bytes.extend_from_slice(&[0u8; 14]);
        // SOF0: length, precision, height, width.
        bytes.extend_from_slice(&[0xff, 0xc0, 0x00, 0x11, 0x08]);
        bytes.extend_from_slice(&900u16.to_be_bytes());
        bytes.extend_from_slice(&600u16.to_be_bytes());
        bytes.extend_from_slice(&[0u8; 8]);
        verdict(size(&bytes), "600x900")
    });
}

#[test]
fn an_unrecognised_file_is_not_a_failure() {
    runTest(|| {
        let mut proved = checks();
        proved.expect(
            parse(b"this is not a picture at all").is_none(),
            "a text file has no size",
        );
        proved.expect(parse(&[]).is_none(), "nor does an empty one");
        proved.verdict()
    });
}
