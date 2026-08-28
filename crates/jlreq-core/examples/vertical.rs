// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Compose a vertical line and inspect the Latin cluster's coordinate transform.

use jlreq_core::{
    Cluster, CoordinateTransform, Frame, Paragraph, ShapedText, Size, Style, WritingMode,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = "縦A";
    let clusters = source.char_indices().map(|(start, character)| {
        Cluster::new(start..start.saturating_add(character.len_utf8()), 1_000)
    });
    let text = ShapedText::new(source, Size::square(1_000)?, Frame::FullEm, clusters)?;
    let paragraph = Paragraph::builder(text, 4_000)
        .writing_mode(WritingMode::VerticalRl)
        .build()?;
    let layout = jlreq_core::compose(&paragraph, &Style::jlreq_2020())?;

    assert_eq!(
        layout.lines()[0].clusters()[1].transform(),
        CoordinateTransform::RotateClockwise
    );
    Ok(())
}
