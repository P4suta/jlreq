// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Compile- and run-tested adapters for the two upstream layers jlreq deliberately
//! does not own. These dependencies are dev-only and never become jlreq features.

use std::collections::BTreeMap;

use harfrust::GlyphBuffer;
use icu_segmenter::{LineSegmenter, options::LineBreakOptions};
use jlreq_core::{Break, Cluster, Frame, Paragraph, ShapedText, Size, Style};

fn clusters_from_harfrust(
    source: &str,
    glyphs: &GlyphBuffer,
) -> Result<Vec<Cluster>, &'static str> {
    if glyphs.glyph_infos().len() != glyphs.glyph_positions().len() {
        return Err("HarfRust returned different info and position lengths");
    }
    let shaped = glyphs
        .glyph_infos()
        .iter()
        .zip(glyphs.glyph_positions())
        .map(|(info, position)| (info.cluster, position.x_advance));
    aggregate_glyph_clusters(source, shaped)
}

fn aggregate_glyph_clusters(
    source: &str,
    glyphs: impl IntoIterator<Item = (u32, i32)>,
) -> Result<Vec<Cluster>, &'static str> {
    let mut advances = BTreeMap::<usize, i64>::new();
    for (cluster, advance) in glyphs {
        let start = usize::try_from(cluster).map_err(|_| "cluster does not fit usize")?;
        if start >= source.len() || !source.is_char_boundary(start) {
            return Err("cluster is not a source UTF-8 boundary");
        }
        let total = advances.entry(start).or_default();
        *total = total
            .checked_add(i64::from(advance))
            .ok_or("cluster advance overflowed i64")?;
    }

    let starts: Vec<usize> = advances.keys().copied().collect();
    let mut clusters = Vec::with_capacity(starts.len());
    for (index, start) in starts.iter().copied().enumerate() {
        let end = starts
            .get(index.saturating_add(1))
            .copied()
            .unwrap_or(source.len());
        let advance = advances
            .get(&start)
            .copied()
            .and_then(|value| i32::try_from(value).ok())
            .ok_or("cluster advance does not fit i32")?;
        if advance < 0 {
            return Err("cluster advance is negative");
        }
        clusters.push(Cluster::new(start..end, advance));
    }
    Ok(clusters)
}

#[test]
fn icu4x_byte_offsets_feed_breaks_without_conversion() {
    let source = "日本語組版";
    let segmenter = LineSegmenter::new_auto(LineBreakOptions::default());
    let offsets: Vec<usize> = segmenter.segment_str(source).collect();
    assert_eq!(offsets, [0, 3, 6, 9, 12, 15]);

    let clusters = source.char_indices().map(|(start, character)| {
        Cluster::new(start..start.saturating_add(character.len_utf8()), 1_000)
    });
    let text = ShapedText::new(
        source,
        Size::square(1_000).expect("positive size"),
        Frame::FullEm,
        clusters,
    )
    .expect("valid shaped text");
    let paragraph = Paragraph::builder(text, 2_000)
        .breaks(offsets.into_iter().map(Break::allowed))
        .build()
        .expect("ICU4X offsets are accepted verbatim");
    assert_eq!(
        jlreq_core::compose(&paragraph, &Style::default())
            .expect("composition succeeds")
            .lines()
            .len(),
        3
    );
}

#[test]
fn harfrust_infos_and_positions_aggregate_to_jlreq_clusters() {
    let clusters = aggregate_glyph_clusters("日本", [(0, 400), (0, 600), (3, 1_000)])
        .expect("two glyphs in one HarfRust cluster are aggregated");
    assert_eq!(clusters.len(), 2);
    assert_eq!(clusters[0].range(), 0..3);
    assert_eq!(clusters[0].advance(), 1_000);
    assert_eq!(clusters[1].range(), 3..6);
    assert_eq!(clusters[1].advance(), 1_000);

    let adapter: fn(&str, &GlyphBuffer) -> Result<Vec<Cluster>, &'static str> =
        clusters_from_harfrust;
    assert_eq!(
        std::mem::size_of_val(&adapter),
        std::mem::size_of::<usize>()
    );
}
