// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Load an explicit font and print the size of a draw-ready layout.

use std::error::Error;

use jlreq::{FontLibrary, LayoutOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("pass a TTF, OTF, or TTC path")?;
    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(path)?)?;
    let layout = jlreq::layout(
        "日本語組版 — draw-ready glyphs",
        &fonts,
        LayoutOptions::try_new(240.0, 16.0)?,
    )?;
    println!(
        "{} line(s), {} glyph(s), {} diagnostic(s)",
        layout.lines().len(),
        layout.glyphs().count(),
        layout.diagnostics().len()
    );
    Ok(())
}
