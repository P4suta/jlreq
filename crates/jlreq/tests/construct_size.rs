// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What the constructs that are not plain body text are actually set at and
//! placed at, pinned as it is rather than as it should be.
//!
//! `crates/jlreq-core/tests/frame_normalization.rs` does this for ADR 0017's
//! unimplemented half, and for the same reason: a defect nobody can see is
//! worse than one a test states, and a test that fails the moment the behaviour
//! is corrected is how the correction announces itself.
//!
//! # The finding
//!
//! Two constructs come out wrong, for two different reasons, and the second is
//! not the facade's:
//!
//! - **A warichu is set at full size.** JLReq §3.4 sets a 割注 in characters
//!   smaller than the surrounding text, two lanes inside the space one line
//!   takes, and `jlreq-core` places it that way: `place_warichu_segment` puts
//!   the two lanes half an em to either side of the line's block origin, and
//!   the line reserves one em of block extent for the pair. The facade hands
//!   `jlreq-core` the paragraph's own em for every cluster it composes and has
//!   no way to be told otherwise, so each lane overhangs its line by half an em
//!   — onto the line beside it. `jlreq::verify` reports both lanes as leaving
//!   their line, and it is meant to.
//! - **A tate-chu-yoko run is not centred in its line.** JLReq §3.2.5 asks for
//!   the string to be set solid left to right and then centred in the vertical
//!   line. The line's block extent is `max(em, members × advance)`, which is
//!   right; the run's position in it is not. The group is centred on the line's
//!   block *origin* rather than on its centre, which displaces it by
//!   `(members − 2) × advance / 2`. At two members that is zero, which is the
//!   count every other test in this workspace uses; at one, three, four and
//!   five the run leaves its own line, and `jlreq::verify` says so.
//!
//! Ruby is not affected by the first — `annotation_options` halves the size for
//! every annotation stream — because ruby text is an annotation the builder
//! shapes, while a warichu's and a tate-chu-yoko's text is body text the
//! builder only marks.
//!
//! A run *widening* its line is not among the findings. It is what §3.2.5's own
//! deferral-ledger entry records as owned, conformance-measured behaviour, and
//! the section asks for nothing narrower; an earlier revision of this file said
//! JLReq requires the run to fit one em, which the primary text does not say.
//! A typesetter would reach for half-width digit forms, and choosing those is
//! shaping, which ADR 0001 and ADR 0002 place with the caller.
//!
//! # Why they are pinned rather than fixed
//!
//! **The size.** Choosing it is the same open question as
//! [ADR 0027](../../../docs/adr/0027-the-layout-is-the-editor-surface.md)'s
//! deferred anisotropic sizes and the ruby size §3.3.3 leaves open: ADR 0019
//! settles that a size the caller measured is carried by the measurement, and
//! the facade offers no way to state one for a warichu at all. It hard-codes an
//! em, the way it hard-codes half an em for every annotation. Giving the caller
//! that knob is a design with its own consequences for the draw contract.
//!
//! **The centring.** It is `jlreq-core`'s, not the facade's: the block
//! coordinates arrive displaced and the facade maps them faithfully. Correcting
//! it changes composed output, which the 122,199-request differential census
//! cannot be re-run here to clear, and the current coordinates are what
//! conformance case `3.2.5/tate-chu-yoko-solid-centered-group` and
//! `tate_chu_yoko_is_one_centered_solid_item_in_a_vertical_line` both expect —
//! at three members, with the same displacement. Changing a case is a claim
//! about conformance and belongs to whoever owns that claim.
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
//! numbers so round. It is not what makes the findings: measured against Yu
//! Gothic, whose digits advance about 0.556 em, the warichu overhang and the
//! tate-chu-yoko displacement are both the same fractions of the advances that
//! face supplies.

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

/// A tate-chu-yoko run is centred in its column only when it holds exactly two
/// members. Every other count puts part of the run outside its own line.
///
/// JLReq §3.2.5: "first set from left to right using solid setting, then align
/// the whole string to the center of the vertical line". The line's block
/// extent is `max(em, members × advance)`, which is right — a run wider than
/// the em widens the line, and the deferral ledger records that as owned,
/// conformance-measured behaviour. What is not right is where the run is put in
/// it: the group is centred on the line's block **origin** rather than on its
/// centre, so it is displaced by `(members − 2) × advance / 2`. That is zero at
/// two members, which is what every test in this workspace used, and it is one
/// whole member at four.
///
/// `jlreq::verify` reports the escape, so the numbers below are stated as
/// coordinates *and* as a soundness expectation: at two members the layout is
/// sound, at every other count it is not. Nothing here needs a font with real
/// metrics — the subset face's square `.notdef` advance makes the arithmetic
/// plain, and a face whose digits are proportional shows the same displacement
/// scaled to its own advance.
#[test]
fn a_tate_chu_yoko_run_is_centred_only_when_it_holds_two_members()
-> Result<(), Box<dyn std::error::Error>> {
    let fonts = fixture()?;
    for (members, group) in [
        // (members, the run's cells from first edge to last, in x)
        (1_i32, (-3 * EM / 2, -EM / 2)),
        (2, (-2 * EM, 0)),
        (3, (-5 * EM / 2, EM / 2)),
        (4, (-3 * EM, EM)),
    ] {
        let count = usize::try_from(members)?;
        let digits: String = "1234".chars().take(count).collect();
        let text = format!("あ{digits}あ");
        let mut builder = DocumentBuilder::new(&text);
        builder.tate_chu_yoko(3..3 + count)?;
        let document = builder.build()?;
        let layout = jlreq::layout_document(
            &document,
            &fonts,
            LayoutOptions::try_new(300.0, 16.0)?.with_writing_mode(WritingMode::VerticalRl),
        )?;

        // The line is as wide as the run needs, which is the part that is right.
        let line = &layout.lines()[0];
        assert_eq!(
            line.block_extent_26_6(),
            EM.max(members.saturating_mul(EM)),
            "{members} member(s): the line no longer takes its width from the run"
        );
        assert_eq!(
            (line.origin().x_26_6() - line.block_extent_26_6()),
            -EM.max(members.saturating_mul(EM)),
            "{members} member(s): the line moved"
        );

        let cells: Vec<(i32, i32)> = line
            .glyphs()
            .iter()
            .filter(|glyph| glyph.transform() == jlreq::GlyphTransform::TateChuYoko)
            .map(|glyph| {
                let (x, _, width, _) = glyph.cell_bounds().as_26_6();
                (x, x.saturating_add(width))
            })
            .collect();
        assert_eq!(cells.len(), count, "{members} member(s)");
        assert_eq!(
            (cells[0].0, cells[count - 1].1),
            group,
            "{members} member(s): the run moved"
        );

        // What §3.2.5 asks for, stated beside what happens, so the correction
        // has a number to reach rather than only a direction.
        let extent = EM.max(members.saturating_mul(EM));
        let centred = (
            -extent / 2 - members * EM / 2,
            -extent / 2 + members * EM / 2,
        );
        assert_eq!(
            centred == group,
            members == 2,
            "{members} member(s): centred would be {centred:?}, the run is at {group:?}"
        );
        assert_eq!(
            jlreq::verify::inspect(&layout).is_sound(),
            members == 2,
            "{members} member(s): {}",
            jlreq::verify::inspect(&layout)
        );
    }
    Ok(())
}
