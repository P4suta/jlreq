// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Minimal horizontal composition with explicit legal break positions.

use jlreq_core::{Break, Cluster, Frame, Paragraph, ShapedText, Size, Style};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = "日本語組版";
    let clusters = source.char_indices().map(|(start, character)| {
        Cluster::new(start..start.saturating_add(character.len_utf8()), 1_000)
    });
    let text = ShapedText::new(source, Size::square(1_000)?, Frame::FullEm, clusters)?;
    let paragraph = Paragraph::builder(text, 4_000)
        .breaks(
            source
                .char_indices()
                .skip(1)
                .map(|(offset, _)| Break::allowed(offset)),
        )
        .build()?;
    let layout = jlreq_core::compose(&paragraph, &Style::book_2020())?;

    assert_eq!(layout.lines().len(), 2);
    Ok(())
}
