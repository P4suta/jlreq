// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What a warichu is actually set at, pinned as it is rather than as it should
//! be.
//!
//! `crates/jlreq-core/tests/frame_normalization.rs` does this for ADR 0017's
//! unimplemented half, and for the same reason: a defect nobody can see is
//! worse than one a test states, and a test that fails the moment the behaviour
//! is corrected is how the correction announces itself.
//!
//! # The finding
//!
//! JLReq §3.4 sets a 割注 in characters smaller than the surrounding text, two
//! lanes inside the space one line would take. `jlreq-core` places it that way:
//! `place_warichu_segment` puts the two lanes half an em to either side of the
//! line's block origin, and the line reserves one em of block extent for the
//! whole construct.
//!
//! The facade never reduces the size. `DocumentBuilder::warichu` marks a range
//! and the clusters inside it reach the composer at the paragraph's own em, so
//! two full-em lanes are placed in the em the line reserved. Each lane overhangs
//! its line by half an em, on opposite sides, and the two lanes overlap the text
//! above and below. `examples/render_svg.rs` draws it: the `割` and the `注` sit
//! on top of the neighbouring lines rather than inside their own.
//!
//! Ruby is not affected — `annotation_options` halves the size for every
//! annotation stream — because ruby text is an annotation the builder shapes,
//! while a warichu's text is body text the builder only marks.
//!
//! # Why it is pinned rather than fixed
//!
//! Choosing the size is the same open question as
//! [ADR 0027](../../../docs/adr/0027-the-layout-is-the-editor-surface.md)'s
//! deferred anisotropic sizes and the ruby size §3.3.3 leaves open: ADR 0019
//! settles that a size the caller measured is carried by the measurement, and
//! the facade offers no way to state one for a warichu at all. It hard-codes an
//! em, the way it hard-codes half an em for every annotation. Giving the caller
//! that knob is a design with its own consequences for the draw contract, and
//! belongs in the same piece of work as the ruby size.
//!
//! Until then this file states the geometry, and `jlreq::verify` reports the two
//! lanes as leaving their line, which
//! `crates/jlreq/tests/document_trace.rs` records as a known fault by name.
//!
//! # About the fixture
//!
//! One face, registered alone, and a document that is nothing but the warichu
//! and the text around it. That is deliberate: the exact 26.6 coordinates below
//! belong to *this* fixture, and the same construct measured against a
//! different set of faces gives different numbers — `document_trace.rs`
//! registers four faces and its `constructs` scenario is composed at a
//! different measure, so its lanes land elsewhere. That file therefore records
//! the defect by *kind* and *count*, which survives a fixture change, while
//! this one pins the coordinates, which does not. A failure here means either
//! the size was chosen at last or the fixture moved; a failure there means the
//! set of geometric defects moved.

use std::sync::Arc;

use jlreq::{DocumentBuilder, FontLibrary, FontStyle, LayoutOptions, WritingMode};

fn fixture() -> Result<FontLibrary, Box<dyn std::error::Error>> {
    let mut fonts = FontLibrary::new();
    fonts.register_face(
        Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF),
        0,
        "Noto Sans JP",
        FontStyle::default(),
    )?;
    Ok(fonts)
}

/// Both lanes are set at the paragraph's own em, and each overhangs its line by
/// half of one.
#[test]
fn a_warichu_is_set_at_full_size_and_overhangs_its_line() -> Result<(), Box<dyn std::error::Error>>
{
    let fonts = fixture()?;
    for (mode, lanes, line_block) in [
        // (lane cells, the line's own block extent)
        (WritingMode::HorizontalTb, [(3072, -512), (3072, 512)], 1024),
        (WritingMode::VerticalRl, [(-512, 3072), (-1536, 3072)], 1024),
    ] {
        let mut builder = DocumentBuilder::new("注釈と割注。");
        builder.warichu(9..15)?;
        let document = builder.build()?;
        let layout = jlreq::layout_document(
            &document,
            &fonts,
            LayoutOptions::try_new(160.0, 16.0)?.with_writing_mode(mode),
        )?;

        let line = &layout.lines()[0];
        assert_eq!(line.block_extent_26_6(), line_block, "{mode:?}");

        let placed: Vec<(i32, i32)> = line
            .glyphs()
            .iter()
            .filter(|glyph| {
                let range = glyph.source_range();
                range.start >= 9 && range.end <= 15
            })
            .map(|glyph| {
                let (x, y, _, _) = glyph.cell_bounds().as_26_6();
                (x, y)
            })
            .collect();
        assert_eq!(placed, lanes, "{mode:?}");

        // Every glyph is at the paragraph's em: nothing was reduced.
        assert!(
            line.glyphs()
                .iter()
                .all(|glyph| glyph.font_size_26_6() == 1024),
            "{mode:?}: a lane was reduced, so this file is out of date"
        );

        let report = jlreq::verify::inspect(&layout);
        let leaving: Vec<&'static str> = report
            .faults()
            .iter()
            .map(jlreq::verify::Fault::kind)
            .collect();
        assert_eq!(
            leaving,
            [
                "cell-escapes-the-measure-silently",
                "cell-escapes-the-measure-silently"
            ],
            "{mode:?}: the checker reports exactly the two lanes"
        );
    }
    Ok(())
}
