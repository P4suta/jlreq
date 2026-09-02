// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Discover an OS font family through the opt-in `system-fonts` feature.
//!
//! Run with `cargo run --example system_fonts --features system-fonts -- "Family Name"`.

use std::error::Error;

use jlreq::{FontLibrary, FontStyle, LayoutOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let family = std::env::args()
        .nth(1)
        .ok_or("pass an installed family name, e.g. \"Yu Gothic\"")?;

    let mut fonts = FontLibrary::new();
    let id = fonts.register_system_family(&family, FontStyle::default())?;
    let resource = fonts.get(id).ok_or("the registered face is readable")?;
    println!(
        "registered {:?}: family {:?}, {} byte(s), default axes {:?}, synthesis {:?}",
        id,
        resource.family(),
        resource.bytes().len(),
        resource.default_variations(),
        resource.synthesis(),
    );

    // Once registered, the copied bytes behave exactly like explicit bytes.
    let layout = jlreq::layout(
        "システムフォントの例",
        &fonts,
        LayoutOptions::try_new(240.0, 16.0)?,
    )?;
    println!(
        "{} line(s), {} glyph(s)",
        layout.lines().len(),
        layout.glyphs().count()
    );
    Ok(())
}
