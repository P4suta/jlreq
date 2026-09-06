// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What the constructs JLReq sets *smaller* are actually set at, pinned as it
//! is rather than as it should be.
//!
//! `crates/jlreq-core/tests/frame_normalization.rs` does this for ADR 0017's
//! unimplemented half, and for the same reason: a defect nobody can see is
//! worse than one a test states, and a test that fails the moment the behaviour
//! is corrected is how the correction announces itself.
//!
//! # The finding
//!
//! The facade hands `jlreq-core` the paragraph's own em for every cluster it
//! composes, and it has no way to be told otherwise. Two constructs JLReq sets
//! at a reduced size therefore come out at full size, in two different shapes:
//!
//! - **A warichu is set at full size.** JLReq §3.4 sets a 割注 in characters
//!   smaller than the surrounding text, two lanes inside the space one line
//!   takes, and `jlreq-core` places it that way: `place_warichu_segment` puts
//!   the two lanes half an em to either side of the line's block origin, and
//!   the line reserves one em of block extent for the pair. Given full-em
//!   clusters, each lane overhangs its line by half an em — onto the line
//!   beside it. `jlreq::verify` reports both lanes as leaving their line, and
//!   it is meant to.
//! - **A tate-chu-yoko run widens its line.** The members of a 縦中横 stand
//!   side by side across the column at their own advances, and the line's block
//!   extent is the sum of them. Nothing fits that sum to the em, so a run of
//!   two digits makes its line wider than every other line in the paragraph.
//!   Nothing escapes anything, so `verify` is silent about it: the line really
//!   is that wide. It is visible only as a column that bulges, which is why it
//!   is pinned here by measurement rather than by fault.
//!
//! Ruby is not affected — `annotation_options` halves the size for every
//! annotation stream — because ruby text is an annotation the builder shapes,
//! while a warichu's and a tate-chu-yoko's text is body text the builder only
//! marks.
//!
//! # Why they are pinned rather than fixed
//!
//! Choosing the size is the same open question as
//! [ADR 0027](../../../docs/adr/0027-the-layout-is-the-editor-surface.md)'s
//! deferred anisotropic sizes and the ruby size §3.3.3 leaves open: ADR 0019
//! settles that a size the caller measured is carried by the measurement, and
//! the facade offers no way to state one for either construct. It hard-codes an
//! em, the way it hard-codes half an em for every annotation. Giving the caller
//! that knob is a design with its own consequences for the draw contract, and
//! all of it belongs in one piece of work.
//!
//! # About the fixture
//!
//! One face, registered alone, and documents that are nothing but the construct
//! and the text around it. That is deliberate: the exact 26.6 coordinates below
//! belong to *this* fixture, and the same construct measured against a
//! different set of faces gives different numbers — `document_trace.rs`
//! registers four faces and its `constructs` scenario is composed at a
//! different measure, so its lanes land elsewhere. That file therefore records
//! the defect by *kind* and *count*, which survives a fixture change, while
//! this one pins the coordinates, which does not.
//!
//! The subset face covers none of this text, so every advance here is a
//! `.notdef` advance — a square em for Noto Sans JP. That is what makes the
//! numbers so round, and it is why the tate-chu-yoko test also states the
//! finding in the font-independent form: the line's block extent is the sum of
//! the run's member advances, whatever those advances happen to be. Measured
//! against a face that really covers ASCII digits the sum is smaller — about
//! 1.1 em for two digits of Yu Gothic — and still not the one em the construct
//! is supposed to occupy.

use std::sync::Arc;

use jlreq::{DocumentBuilder, FontLibrary, FontStyle, LayoutOptions, WritingMode};

/// 16 pt in 26.6 fixed point: the em every cluster in these documents is set
/// at, and the width a line of them takes.
const EM: i32 = 1024;

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
                .all(|glyph| glyph.font_size_26_6() == EM),
            "{mode:?}: a lane was reduced, so this file is out of date"
        );

        // The overhang is on the block axis — onto the neighbouring line — and
        // the measure is met, so exactly one of the two statements fires.
        let report = jlreq::verify::inspect(&layout);
        let leaving: Vec<&'static str> = report
            .faults()
            .iter()
            .map(jlreq::verify::Fault::kind)
            .collect();
        assert_eq!(
            leaving,
            ["cell-escapes-its-line", "cell-escapes-its-line"],
            "{mode:?}: the checker reports exactly the two lanes"
        );
    }
    Ok(())
}

/// The run's members stand side by side at their own advances and the line
/// grows to hold them, so a paragraph whose other lines are one em wide gets
/// one line that is two.
#[test]
fn a_tate_chu_yoko_run_widens_its_line_instead_of_fitting_the_em()
-> Result<(), Box<dyn std::error::Error>> {
    let fonts = fixture()?;
    let mut builder = DocumentBuilder::new("あ12あ\nああ");
    builder.tate_chu_yoko(3..5)?;
    let document = builder.build()?;
    let layout = jlreq::layout_document(
        &document,
        &fonts,
        LayoutOptions::try_new(200.0, 16.0)?.with_writing_mode(WritingMode::VerticalRl),
    )?;

    let (run, plain) = (&layout.lines()[0], &layout.lines()[1]);
    assert_eq!(
        plain.block_extent_26_6(),
        EM,
        "a line of body text is an em"
    );
    assert_eq!(run.block_extent_26_6(), 2 * EM, "the run doubled its line");

    // The font-independent half of the finding: the line is exactly as wide as
    // the run's members are, summed. Nothing fitted the run to the em.
    let run_total: i32 = run
        .glyphs()
        .iter()
        .filter(|glyph| glyph.transform() == jlreq::GlyphTransform::TateChuYoko)
        .map(|glyph| glyph.geometry_26_6().2.abs())
        .sum();
    assert_eq!(run_total, run.block_extent_26_6());
    assert!(
        run_total > EM,
        "the run fits the em now, so this file is out of date"
    );

    // Nothing escapes: the line is honestly that wide, which is why this
    // finding needs a measurement rather than a fault.
    assert!(jlreq::verify::inspect(&layout).is_sound());
    Ok(())
}
