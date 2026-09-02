// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Compare the published JLReq 2020 composition-policy profiles.

use std::error::Error;

use jlreq::{FontLibrary, LayoutOptions, Style};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("pass a TTF, OTF, or TTC path")?;
    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(path)?)?;

    // Long punctuation-heavy text makes the profiles' kinsoku and spacing
    // choices visible as different line counts and extents.
    let text = "「組版とは、」——文字を、行に、頁に、整えることです。！？";
    let profiles = [
        ("jlreq_2020 (default)", Style::jlreq_2020()),
        ("book_2020", Style::book_2020()),
        ("magazine_2020", Style::magazine_2020()),
        ("newspaper_2020", Style::newspaper_2020()),
        ("jis_reading_2020", Style::jis_reading_2020()),
    ];
    for (name, style) in profiles {
        let layout = jlreq::layout(
            text,
            &fonts,
            LayoutOptions::try_new(160.0, 16.0)?.with_style(style),
        )?;
        let widest = layout
            .lines()
            .iter()
            .map(jlreq::TextLine::inline_extent)
            .fold(0.0_f32, f32::max);
        println!(
            "{name:>22}: {} line(s), widest {widest:.1}",
            layout.lines().len()
        );
    }
    Ok(())
}
