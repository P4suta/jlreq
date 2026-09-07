// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Register several faces, steer fallback, and address one by family.

use std::error::Error;

use jlreq::{DocumentBuilder, FontLibrary, LayoutOptions, SpanStyle};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args().skip(1);
    let primary_path = arguments
        .next()
        .ok_or("pass one or two font paths (Japanese first)")?;
    let secondary_path = arguments.next();

    let mut fonts = FontLibrary::new();
    // register_font derives the family from the font's own name table.
    let primary = fonts.register_font(std::fs::read(&primary_path)?)?;
    let secondary = match secondary_path {
        Some(path) => fonts.register_font(std::fs::read(path)?)?,
        None => primary,
    };
    fonts.set_primary(primary)?;
    fonts.set_fallback_order([primary, secondary])?;
    for font in fonts.fonts() {
        println!("registered {:?} as family {:?}", font.id(), font.family());
    }

    let text = "日本語とLatinの混植";
    let derived_family = fonts
        .get(primary)
        .map(|font| font.family().to_owned())
        .ok_or("primary font metadata")?;
    let mut document = DocumentBuilder::new(text);
    document.span(0..9, SpanStyle::new().with_family(&derived_family))?;
    let layout = jlreq::layout_document(
        &document.build()?,
        &fonts,
        LayoutOptions::try_new(320.0, 16.0)?,
    )?;

    for glyph in layout.glyphs().take(6) {
        println!(
            "{:?} -> font {:?}",
            &text[glyph.source_range()],
            glyph.font_id()
        );
    }
    for diagnostic in layout.diagnostics() {
        println!("diagnostic {}: {}", diagnostic.code(), diagnostic.message());
    }
    Ok(())
}
