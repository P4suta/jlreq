// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Drive the editor toolkit: hit testing, caret motion, and word selection.

use std::error::Error;

use jlreq::{DocumentBuilder, FontLibrary, LayoutOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("pass a TTF, OTF, or TTC path")?;
    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(path)?)?;

    let text = "これは日本語の文章です。ルビ付きの漢字もあります。";
    let mut document = DocumentBuilder::new(text);
    let kanji = text.find("漢字").ok_or("substring")?;
    document.group_ruby(kanji..kanji.saturating_add(6), "かんじ")?;
    let document = document.build()?;
    let layout = jlreq::layout_document(&document, &fonts, LayoutOptions::try_new(160.0, 16.0)?)?;

    // A click lands on a byte offset plus the visual side it selected.
    let hit = layout.hit_test_xy(40.0, 8.0)?;
    println!(
        "hit: byte {} ({:?}), line {:?}",
        hit.byte_offset(),
        hit.affinity(),
        layout.line_index_at(hit.byte_offset())
    );
    if let Some(caret) = layout.caret_rect(hit.byte_offset(), hit.affinity()) {
        println!("caret rect: {:?}", caret.as_26_6());
    }

    // Arrow keys: visual motion within the surface, and line-to-line motion.
    if let Some(next) = layout.next_visual_caret(hit.byte_offset(), hit.affinity()) {
        println!(
            "visual right -> byte {} ({:?})",
            next.byte_offset(),
            next.affinity()
        );
    }
    if let Some(below) = layout.caret_next_line(hit.byte_offset(), hit.affinity()) {
        println!(
            "next line    -> byte {} ({:?})",
            below.byte_offset(),
            below.affinity()
        );
    }

    // Double-click: dictionary-backed Japanese word segmentation.
    if let Some(word) = layout.word_range_at(hit.byte_offset()) {
        println!(
            "word under the caret: {:?} = {:?}",
            word.clone(),
            &text[word]
        );
    }

    // Select the whole ruby construct from any of its glyphs.
    if let Some(glyph) = layout.glyphs().find(|glyph| glyph.construct().is_some()) {
        let ordinal = glyph.construct().ok_or("construct glyph")?;
        let construct = document.construct(ordinal).ok_or("construct read-back")?;
        println!("construct under glyph: {:?}", construct.range());
        let filled = layout.selection_rects_filled(construct.range());
        println!("filled selection rects: {}", filled.len());
    }
    Ok(())
}
