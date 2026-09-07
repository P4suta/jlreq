// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Lay the same text out horizontally and vertically and compare geometry.

use std::error::Error;

use jlreq::{FontLibrary, GlyphTransform, LayoutOptions, WritingMode};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("pass a TTF, OTF, or TTC path")?;
    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(path)?)?;
    let text = "縦書きとLatin混在の例";

    let horizontal = jlreq::layout(text, &fonts, LayoutOptions::try_new(200.0, 16.0)?)?;
    let vertical = jlreq::layout(
        text,
        &fonts,
        LayoutOptions::try_new(200.0, 16.0)?.with_writing_mode(WritingMode::VerticalRl),
    )?;

    println!(
        "horizontal: {} line(s); vertical: {} column(s)",
        horizontal.lines().len(),
        vertical.lines().len()
    );
    let rotated = vertical
        .glyphs()
        .filter(|glyph| glyph.transform() == GlyphTransform::RotateClockwise)
        .count();
    println!("vertical Latin glyphs rotated clockwise: {rotated}");
    for line in vertical.lines() {
        println!(
            "column {} at x={:.1} covers bytes {:?}",
            line.index(),
            line.origin().x(),
            line.range()
        );
    }
    Ok(())
}
