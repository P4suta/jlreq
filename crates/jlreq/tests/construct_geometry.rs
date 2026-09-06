// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where the constructs that are not plain body text are set, and at what size.
//!
//! Three defects lived here until `docs/adr/0030` and were each invisible for
//! the same reason: every fixture in the workspace used the one parameter at
//! which the defect cancels. `construct_matrix.rs` sweeps the parameter; this
//! file states what the sweep is sweeping *for*, in exact 26.6 units.
//!
//! - **A tate-chu-yoko run is centred in its line.** JLReq §3.2.5 sets the
//!   string solid from left to right and then aligns it to the centre of the
//!   vertical line. The run was centred on the line's block *origin* instead,
//!   which is its edge, so it was displaced by `(members − 2) × advance / 2` —
//!   zero at two members, and enough at three to put a member on the line
//!   beside it.
//! - **A furawake is centred in its line.** The line reserves one em per
//!   column; the segment was centred inside a single em, so every lane landed
//!   half the surplus early and the first sat on the line above.
//! - **A warichu is set at half the paragraph's size.** JLReq §3.4 sets a 割注
//!   in characters smaller than the text around it, two lanes inside the space
//!   one line takes, and the composer reserves exactly one em for the pair. The
//!   facade handed it full-em clusters, so the pair was two em in that one.
//!
//! A run *widening* its line is not a defect and never was: §3.2.5 asks for
//! solid setting and centring and nothing narrower, and
//! `docs/conformance-deferrals.toml` records the widening as owned behaviour.
//! Getting two digits into one em is a matter of their half-width forms, which
//! is shaping, which ADR 0001 and ADR 0002 place with the caller.
//!
//! # About the fixture
//!
//! One face, registered alone, and documents that are nothing but the construct
//! and the text around it. The subset covers none of this text, so every
//! advance is a `.notdef` advance — a square em for Noto Sans JP — which is
//! what makes the numbers round. The relationships hold at any advance; the
//! exact coordinates belong to this fixture.

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

/// Two lanes at half the paragraph's size, filling the one em the line reserved
/// for them, in the order the writing mode reads.
#[test]
fn a_warichu_is_half_size_and_fills_the_em_its_line_reserved()
-> Result<(), Box<dyn std::error::Error>> {
    let fonts = fixture()?;
    for (mode, lanes) in [
        // (each lane's cell, first lane first: x, y, width, height)
        (
            WritingMode::HorizontalTb,
            [(3072, 0, 512, 512), (3072, 512, 512, 512)],
        ),
        (
            WritingMode::VerticalRl,
            [(-512, 3072, 512, 512), (-1024, 3072, 512, 512)],
        ),
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
        assert_eq!(
            line.block_extent_26_6(),
            EM,
            "{mode:?}: a warichu takes the space of one line"
        );

        let placed: Vec<(i32, i32, i32, i32)> = line
            .glyphs()
            .iter()
            .filter(|glyph| {
                let range = glyph.source_range();
                range.start >= 9 && range.end <= 15
            })
            .map(|glyph| glyph.cell_bounds().as_26_6())
            .collect();
        assert_eq!(placed, lanes, "{mode:?}");

        // §3.4's own statement, and the one the facade had no way to make: the
        // lanes are set smaller than the text around them.
        for glyph in line.glyphs() {
            let inside = glyph.source_range().start >= 9 && glyph.source_range().end <= 15;
            assert_eq!(
                glyph.font_size_26_6(),
                if inside { EM / 2 } else { EM },
                "{mode:?}: {:?}",
                glyph.source_range()
            );
        }
        assert!(jlreq::verify::inspect(&layout).is_sound());
    }
    Ok(())
}

/// However many members it holds, the run stands centred across its column, and
/// the line is as wide as the run needs.
#[test]
fn a_tate_chu_yoko_run_is_centred_in_its_line_at_every_member_count()
-> Result<(), Box<dyn std::error::Error>> {
    let fonts = fixture()?;
    for members in 1..=5_i32 {
        let count = usize::try_from(members)?;
        let digits: String = "12345".chars().take(count).collect();
        let mut builder = DocumentBuilder::new(format!("あ{digits}あ"));
        builder.tate_chu_yoko(3..3 + count)?;
        let document = builder.build()?;
        let layout = jlreq::layout_document(
            &document,
            &fonts,
            LayoutOptions::try_new(300.0, 16.0)?.with_writing_mode(WritingMode::VerticalRl),
        )?;

        let line = &layout.lines()[0];
        let run = members.saturating_mul(EM);
        let extent = EM.max(run);
        assert_eq!(
            line.block_extent_26_6(),
            extent,
            "{members} member(s): the line is as wide as the run needs"
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

        // Centred: the surplus over the run is split evenly between the two
        // sides of the column. The line runs from its origin backwards along
        // −x, so the run's far edge is the origin less half the surplus.
        let surplus = extent.saturating_sub(run);
        let expected = (
            line.origin().x_26_6() - extent + surplus / 2,
            line.origin().x_26_6() - surplus / 2,
        );
        assert_eq!(
            (cells[0].0, cells[count - 1].1),
            expected,
            "{members} member(s): the run is not centred in its column"
        );
        assert!(jlreq::verify::inspect(&layout).is_sound());
    }
    Ok(())
}

/// The lanes fill the columns the line reserved, in order, with no gap.
#[test]
fn a_furawake_fills_the_columns_its_line_reserved() -> Result<(), Box<dyn std::error::Error>> {
    let fonts = fixture()?;
    for (columns, lanes) in [
        // (columns, each lane's block-axis start, in the order the lanes read)
        (2_u16, vec![0, EM]),
        (3, vec![0, EM, 2 * EM]),
    ] {
        let reserved = EM * i32::from(columns);
        // 前(0..3) と(3..6) then four kanji, so every column holds content.
        let mut builder = DocumentBuilder::new("前と一二三四後。");
        builder.furawake(6..18, columns, 0.0)?;
        let document = builder.build()?;
        let layout =
            jlreq::layout_document(&document, &fonts, LayoutOptions::try_new(300.0, 16.0)?)?;

        let line = &layout.lines()[0];
        assert_eq!(
            (line.origin().y_26_6(), line.block_extent_26_6()),
            (0, reserved),
            "{columns} columns: the line reserves an em per column"
        );

        let mut starts: Vec<i32> = line
            .glyphs()
            .iter()
            .filter(|glyph| {
                let range = glyph.source_range();
                range.start >= 6 && range.end <= 18
            })
            .map(|glyph| glyph.cell_bounds().as_26_6().1)
            .collect();
        starts.sort_unstable();
        starts.dedup();
        assert_eq!(starts, lanes, "{columns} columns: the lanes are not flush");

        // Every lane begins at the construct's own inline origin: they stand
        // side by side, not end to end.
        let inline_starts: Vec<i32> = lanes
            .iter()
            .map(|lane| {
                line.glyphs()
                    .iter()
                    .filter(|glyph| {
                        let range = glyph.source_range();
                        range.start >= 6
                            && range.end <= 18
                            && glyph.cell_bounds().as_26_6().1 == *lane
                    })
                    .map(|glyph| glyph.cell_bounds().as_26_6().0)
                    .min()
                    .unwrap_or_default()
            })
            .collect();
        assert!(
            inline_starts.windows(2).all(|pair| pair[0] == pair[1]),
            "{columns} columns: the lanes start at {inline_starts:?}"
        );
        assert!(jlreq::verify::inspect(&layout).is_sound());
    }
    Ok(())
}

/// A note too long for its measure straddles two main lines when — and only
/// when — the document says where it may split.
///
/// JLReq §3.4.3 allows the straddle and `jlreq-core` composes it; the facade
/// suppresses automatic break opportunities inside every construct, so from
/// here it takes a `discretionary_break`. Both halves of that statement are
/// asserted, because the interesting one is the default: a long note that
/// silently overflows its measure is what a caller gets if they do not know
/// about the break, and `layout.overfull` is how they find out.
///
/// `construct_matrix.rs` sweeps single-line constructs and cannot reach this;
/// nothing else in the facade had a straddling note at all.
#[test]
fn a_long_warichu_straddles_two_lines_only_when_a_break_is_declared()
-> Result<(), Box<dyn std::error::Error>> {
    let fonts = fixture()?;
    // Twenty clusters of note between `前と` and `後。`: ten per lane at half
    // size is 80 pt of the 56 pt measure, so it cannot be set on one line.
    let note = "一二三四五六七八九十甲乙丙丁戊己庚辛壬癸";
    let text = format!("前と{note}後。");
    let note_range = 6..66;

    for declared in [None, Some(36)] {
        let mut builder = DocumentBuilder::new(text.clone());
        builder.warichu(note_range.clone())?;
        if let Some(offset) = declared {
            builder.discretionary_break(offset)?;
        }
        let document = builder.build()?;
        let layout =
            jlreq::layout_document(&document, &fonts, LayoutOptions::try_new(56.0, 16.0)?)?;

        let lines = layout
            .lines()
            .iter()
            .filter(|line| {
                line.glyphs().iter().any(|glyph| {
                    let range = glyph.source_range();
                    range.start >= note_range.start && range.end <= note_range.end
                })
            })
            .count();
        let overfull = layout
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "layout.overfull");

        if declared.is_some() {
            assert_eq!(lines, 2, "a declared split must be taken");
            assert!(!overfull, "and must leave both lines within the measure");
        } else {
            assert_eq!(lines, 1, "an undeclared note must stay on one line");
            assert!(overfull, "and must say that it did not fit");
        }
        let report = jlreq::verify::inspect(&layout);
        assert!(report.is_sound(), "{declared:?}: {report}");
    }
    Ok(())
}
