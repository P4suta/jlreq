// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Center a heading, indent body text, and narrow a quotation in one document.

use std::error::Error;

use jlreq::{Alignment, DocumentBuilder, FontLibrary, LayoutOptions, ParagraphStyle, Widow};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("pass a TTF, OTF, or TTC path")?;

    let heading = "組版の見出し";
    let body = "本文の段落は一文字下げから始まり、行末の孤立を避けます。";
    let quote = "引用は狭い版面で組まれます。";
    let text = format!("{heading}\n{body}\n{quote}");

    let heading_range = 0..heading.len();
    let body_start = heading.len().saturating_add(1);
    let body_range = body_start..body_start.saturating_add(body.len());
    let quote_range = body_range.end.saturating_add(1)..text.len();

    let mut document = DocumentBuilder::new(&text);
    document.paragraph_style(
        heading_range,
        ParagraphStyle::new().with_alignment(Alignment::Center),
    )?;
    document.paragraph_style(
        body_range,
        ParagraphStyle::new()
            .with_first_line_indent(16.0)?
            .with_widow(Widow::MinimumClusters(2)),
    )?;
    document.paragraph_style(quote_range, ParagraphStyle::new().with_line_extent(200.0)?)?;

    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(path)?)?;
    let layout = jlreq::layout_document(
        &document.build()?,
        &fonts,
        LayoutOptions::try_new(320.0, 16.0)?,
    )?;

    for line in layout.lines() {
        let marker = if line.is_first_in_paragraph() {
            "¶"
        } else {
            " "
        };
        println!(
            "{marker} paragraph {} line {}: bytes {:?}, starts at x={:.1}",
            line.paragraph_index(),
            line.index(),
            line.range(),
            line.glyphs().first().map_or(0.0, jlreq::GlyphPlacement::x),
        );
    }
    Ok(())
}
