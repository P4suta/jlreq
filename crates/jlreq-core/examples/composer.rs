// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reuse one composer, handle a typed resource error, and continue composing afterward.

use jlreq_core::{
    Cluster, Composer, CompositionLimits, CompositionResource, Frame, Paragraph, ShapedText, Size,
    Style,
};

fn paragraph(source: &str) -> Result<Paragraph, jlreq_core::InputError> {
    let clusters = source.char_indices().map(|(start, character)| {
        Cluster::new(start..start.saturating_add(character.len_utf8()), 1_000)
    });
    let text = ShapedText::new(source, Size::square(1_000)?, Frame::FullEm, clusters)?;
    Paragraph::builder(text, 4_000).build()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let first = paragraph("日本")?;
    let second = paragraph("組版")?;
    let mut composer = Composer::new();
    let first_layout = composer.compose(&first, &Style::jlreq_2020())?;
    let second_layout = composer.compose(&second, &Style::book_2020())?;
    assert_eq!(first_layout.lines().len(), 1);
    assert_eq!(second_layout.lines().len(), 1);

    composer.set_limits(CompositionLimits::default().with_max_clusters(1));
    let Err(error) = composer.compose(&first, &Style::jlreq_2020()) else {
        return Err("the two-cluster paragraph unexpectedly fit its configured limit".into());
    };
    assert_eq!(error.code(), "compose.cluster-limit");
    assert_eq!(error.resource(), CompositionResource::Clusters);
    assert_eq!((error.limit(), error.observed()), (1, 2));

    composer.set_limits(CompositionLimits::default());
    assert!(composer.compose(&first, &Style::jlreq_2020()).is_ok());
    Ok(())
}
