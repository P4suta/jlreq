// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Build all nine typed inline constructs and inspect what came back.

use std::error::Error;

use jlreq::{DocumentBuilder, FontLibrary, LayoutOptions, ScriptPosition};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("pass a TTF, OTF, or TTC path")?;

    let text = "漢字12 注記 割注 振分 字取 * H2O x+y";
    let mut document = DocumentBuilder::new(text);
    document.group_ruby(0..6, "かんじ")?;
    document.tate_chu_yoko(6..8)?;
    document.emphasis_dots(9..15, '・')?;
    document.warichu(16..22)?;
    // Furawake balances its own sublines; add mandatory_break calls inside
    // the range only for explicit splits.
    document.furawake(23..29, 2, 1.0)?;
    document.jidori(30..36, 4)?;
    document.reference_mark(37..38, "※")?;
    document.script(39..42, "2", ScriptPosition::Subscript)?;
    document.formula(43..46)?;
    let document = document.build()?;

    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(path)?)?;
    let layout = jlreq::layout_document(&document, &fonts, LayoutOptions::try_new(240.0, 16.0)?)?;

    // The document reads back what the builder accepted, and every glyph
    // reports the construct it belongs to.
    for (ordinal, construct) in document.constructs().enumerate() {
        let glyphs = layout
            .glyphs()
            .filter(|glyph| glyph.construct() == Some(ordinal))
            .count();
        println!("construct {ordinal}: {construct:?} -> {glyphs} glyph(s)");
    }
    println!(
        "{} line(s), {} glyph(s) in total",
        layout.lines().len(),
        layout.glyphs().count()
    );
    Ok(())
}
