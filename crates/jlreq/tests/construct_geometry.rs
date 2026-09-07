// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Where the constructs that are not plain body text are set, and at what size.
//!
//! Three defects lived here until `docs/adr/0030` and were each invisible for
//! the same reason: every fixture in the workspace used the one parameter at
//! which the defect cancels. `construct_matrix.rs` sweeps the parameter; this
//! file states what the sweep is sweeping *for*, in exact 26.6 units.
//!
//! - **A tate-chu-yoko run is centered in its line.** JLReq §3.2.5 sets the
//!   string solid from left to right and then aligns it to the center of the
//!   vertical line. The run was centered on the line's block *origin* instead,
//!   which is its edge, so it was displaced by `(members − 2) × advance / 2` —
//!   zero at two members, and enough at three to put a member on the line
//!   beside it.
//! - **A furawake is centered in its line.** The line reserves one em per
//!   column; the segment was centered inside a single em, so every lane landed
//!   half the surplus early and the first sat on the line above.
//! - **A warichu is set at half the paragraph's size.** JLReq §3.4 sets a 割注
//!   in characters smaller than the text around it, two lanes inside the space
//!   one line takes, and the composer reserves exactly one em for the pair. The
//!   facade handed it full-em clusters, so the pair was two em in that one.
//!
//! A run *widening* its line is not a defect and never was: §3.2.5 asks for
//! solid setting and centering and nothing narrower, and
//! `docs/conformance-deferrals.toml` records the widening as owned behavior.
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

use std::ops::Range;
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

/// However many members it holds, the run stands centered across its column, and
/// the line is as wide as the run needs.
#[test]
fn a_tate_chu_yoko_run_is_centered_in_its_line_at_every_member_count()
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

        // Centered: the surplus over the run is split evenly between the two
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
            "{members} member(s): the run is not centered in its column"
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

/// A furawake in a line that bidi reordering shuffled still runs past the
/// measure, and this is what says so.
///
/// The facade walks a line's cells in visual order with a cumulative cursor. A
/// lane restarts behind its predecessor, and the cursor can only take that step
/// back when the two cells are still adjacent in the walk. Reordering separates
/// them, the lanes go end to end, and the second one leaves the measure — which
/// is what every furawake did before `docs/adr/0030` and what only this case
/// still does.
///
/// Pinned rather than fixed: `docs/adr/0031` records both attempts that made it
/// worse — computing the gaps in visual order breaks every ordinary bidi line,
/// and giving a construct one bidi level throughout breaks more columns than it
/// fixes — and neither is a change to make at merge time. The facade fuzz target
/// carries the matching exemption, keyed to the same condition, and this test is
/// what keeps that exemption honest: fix the defect and this test fails.
#[test]
fn a_furawake_in_a_reordered_line_is_still_wrong() -> Result<(), Box<dyn std::error::Error>> {
    let fonts = fixture()?;
    // Latin letters between neutrals: with a right-to-left base direction the
    // runs interleave, so the construct's lanes do not stay together.
    let text = "\u{fffd}\u{fffd}Aba\u{fffd}\u{fffd}ir\u{fffd}\u{fffd}Ar";
    let mut builder = DocumentBuilder::new(text);
    builder.furawake(0..text.len() / 2, 2, 0.5)?;
    let document = builder.build()?;
    let layout = jlreq::layout_document(
        &document,
        &fonts,
        // Wide enough that nothing wraps: the lanes leave the measure because
        // they were laid end to end, not because the line was short.
        LayoutOptions::try_new(2056.0, 16.0)?
            .with_writing_mode(WritingMode::HorizontalTb)
            .with_base_direction(jlreq::BaseDirection::RightToLeft),
    )?;

    let reordered = layout
        .glyphs()
        .any(|glyph| glyph.bidi_level() % 2 == 1 && glyph.construct().is_some());
    assert!(reordered, "the fixture no longer reorders the construct");

    let kinds: Vec<&str> = jlreq::verify::inspect(&layout)
        .faults()
        .iter()
        .map(jlreq::verify::Fault::kind)
        .collect();
    assert_eq!(
        kinds,
        vec!["cell-escapes-the-measure-silently"; kinds.len()],
        "the only thing wrong here is the lane that went past the measure"
    );
    // The two lane cells by name, not just by kind. `docs/adr/0032` narrowed
    // this fault's exemption, and the first rule tried — excusing a run at the
    // line's start the way one at its end is excused — silenced exactly these
    // two and nothing else. Only an assertion that says *which* cells could
    // tell that apart from a correction.
    let ranges: Vec<Range<usize>> = jlreq::verify::inspect(&layout)
        .faults()
        .iter()
        .filter_map(|fault| match fault {
            jlreq::verify::Fault::CellEscapesTheMeasureSilently { range, .. } => {
                Some(range.clone())
            },
            _ => None,
        })
        .collect();
    assert_eq!(
        ranges,
        vec![7..8, 8..9],
        "docs/adr/0031's furawake case has moved or been fixed"
    );
    Ok(())
}

/// A reading declared at 三分ルビ is condensed, not merely small.
///
/// JLReq §3.3.3 gives it a block extent of half the base em and an inline
/// extent of a third. One scalar cannot say that, which is why
/// [`GlyphPlacement::inline_size`] exists and why `docs/adr/0033` adds it: the
/// face is still set at half — 512 of the paragraph's 1024 — and the outlines
/// are narrowed to 341, which is 1024 × 240/720 truncated, the third stated in
/// ADR-0007's 1/720 em.
///
/// Asked in both writing modes because the axis that narrows is the text's, not
/// the screen's: it is the cell's width in horizontal writing and its height in
/// vertical, and a condensation applied to a screen axis would be right in one
/// and wrong in the other.
#[test]
fn a_three_part_ruby_is_condensed_across_the_inline_axis() -> Result<(), Box<dyn std::error::Error>>
{
    let fonts = fixture()?;
    for (mode, half_cell, third_cell) in [
        // (mode, one ruby cell at RubyScale::HALF, the same at THIRD)
        (
            WritingMode::HorizontalTb,
            (86, -512, 512, 512),
            (171, -512, 341, 512),
        ),
        (
            WritingMode::VerticalRl,
            (0, 86, 512, 512),
            (0, 171, 512, 341),
        ),
    ] {
        for (scale, expected, inline_em) in [
            (jlreq::RubyScale::HALF, half_cell, 512),
            (jlreq::RubyScale::THIRD, third_cell, 341),
        ] {
            let mut builder = DocumentBuilder::new("漢字とルビ");
            builder.group_ruby(0..6, "かんじ")?;
            let document = builder.build()?;
            let layout = jlreq::layout_document(
                &document,
                &fonts,
                LayoutOptions::try_new(400.0, 16.0)?
                    .with_writing_mode(mode)
                    .with_ruby_scale(scale),
            )?;

            let ruby: Vec<&jlreq::GlyphPlacement> = layout
                .glyphs()
                .filter(|glyph| glyph.annotation().is_some())
                .collect();
            assert_eq!(ruby.len(), 3, "{mode:?}: the reading is three clusters");
            for glyph in &ruby {
                // The block em never moves: a renderer sets the face at this
                // size under either scale, and condenses from there.
                assert_eq!(glyph.font_size_26_6(), EM / 2, "{mode:?}");
                assert_eq!(glyph.inline_size_26_6(), inline_em, "{mode:?}");
                // The caller's unit says the same thing. Asked separately
                // because it is a separate accessor, and one that reported a
                // constant would agree with the fixed-point one nowhere.
                assert!(
                    (glyph.inline_size() - f32::from(i16::try_from(inline_em)?) / 64.0).abs()
                        < f32::EPSILON,
                    "{mode:?}: {} is not {inline_em} in 26.6",
                    glyph.inline_size()
                );
            }
            assert_eq!(ruby[0].cell_bounds().as_26_6(), expected, "{mode:?}");

            // The advance narrows with the em, so the reading still partitions
            // its base rather than overhanging what the composer reserved.
            let (_, _, width, height) = ruby[0].cell_bounds().as_26_6();
            let along = match mode {
                WritingMode::VerticalRl => height,
                _ => width,
            };
            assert_eq!(along, inline_em, "{mode:?}: the cell follows the advance");
            assert!(jlreq::verify::inspect(&layout).is_sound(), "{mode:?}");
        }
    }
    Ok(())
}
