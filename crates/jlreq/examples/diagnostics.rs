// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Distinguish recoverable diagnostics from hard errors and stay reusable.

use std::error::Error;

use jlreq::{DocumentBuilder, FontLibrary, LayoutEngine, LayoutOptions, ResourceLimits, SpanStyle};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("pass a TTF, OTF, or TTC path")?;
    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(path)?)?;
    let mut engine = LayoutEngine::new();

    // Recoverable conditions accompany a complete layout: a glyph no face
    // covers, and a span family no face declares.
    let mut document = DocumentBuilder::new("covered \u{10ffff}");
    document.span(0..7, SpanStyle::new().with_family("No Such Family"))?;
    let layout = engine.layout_document(
        &document.build()?,
        &fonts,
        LayoutOptions::try_new(240.0, 16.0)?,
    )?;
    for diagnostic in layout.diagnostics() {
        println!(
            "diagnostic {} ({:?}) at {:?}: {}",
            diagnostic.code(),
            diagnostic.severity(),
            diagnostic.range(),
            diagnostic.message(),
        );
    }

    // Exhausted limits are hard errors with stable codes and messages, and
    // they never poison the engine: the next call is independent.
    let starved = LayoutOptions::try_new(240.0, 16.0)?
        .with_limits(ResourceLimits::default().with_max_glyphs(1));
    match engine.layout("組版", &fonts, starved) {
        Ok(_) => println!("unexpectedly fit within one glyph"),
        Err(error) => println!(
            "error {}: {}",
            error.code(),
            error.message().unwrap_or("(no message)")
        ),
    }
    let recovered = engine.layout("組版", &fonts, LayoutOptions::try_new(240.0, 16.0)?)?;
    println!("engine reused: {} glyph(s)", recovered.glyphs().count());
    Ok(())
}
