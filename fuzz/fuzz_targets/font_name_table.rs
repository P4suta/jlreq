// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Feed arbitrary bytes to the facade's font registration path.
//!
//! Registration hand-parses the SFNT `name`, `head`, `hhea`, `OS/2`, and
//! `post` tables to derive a family and design metrics, and those parsers
//! read attacker-controlled bytes. Wrapping the arbitrary input in a
//! syntactically plausible one-table SFNT container steers the fuzzer past
//! HarfRust's outer validation and straight into the table readers, in
//! addition to throwing the raw bytes at registration whole.

#![no_main]

use std::hint::black_box;
use std::sync::Arc;

use jlreq::FontLibrary;
use libfuzzer_sys::fuzz_target;

const TABLE_TAGS: [&[u8; 4]; 5] = [b"name", b"head", b"hhea", b"OS/2", b"post"];

fuzz_target!(|data: &[u8]| {
    let mut whole = FontLibrary::new();
    let _ = black_box(whole.register_font(Arc::<[u8]>::from(data)));

    // A minimal SFNT: one table directory entry per interesting tag, each
    // pointing at the same arbitrary payload.
    let payload = data.get(1..).unwrap_or_default();
    let tag_count = usize::from(data.first().copied().unwrap_or_default() % 6);
    let mut font = Vec::new();
    font.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
    font.extend_from_slice(&u16::try_from(tag_count).unwrap_or_default().to_be_bytes());
    font.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    let header = 12_usize.saturating_add(tag_count.saturating_mul(16));
    for tag in TABLE_TAGS.iter().take(tag_count) {
        font.extend_from_slice(*tag);
        font.extend_from_slice(&[0, 0, 0, 0]);
        font.extend_from_slice(&u32::try_from(header).unwrap_or_default().to_be_bytes());
        font.extend_from_slice(
            &u32::try_from(payload.len())
                .unwrap_or_default()
                .to_be_bytes(),
        );
    }
    font.extend_from_slice(payload);
    let mut wrapped = FontLibrary::new();
    if let Ok(id) = wrapped.register_font(Arc::<[u8]>::from(font.as_slice())) {
        let resource = black_box(wrapped.get(id));
        let _ = black_box(resource.map(|font| (font.family().len(), font.metrics())));
    }
});
