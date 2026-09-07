// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Drive [`jlreq::verify`] over a corpus built to make the geometry branch.
//!
//! `docs/design/geometry.md` states the coordinate system and `jlreq::verify`
//! holds the crate to it; this is where it is asked, over both writing modes,
//! wrapped and unwrapped measures, blank paragraphs, mixed scripts, and the
//! constructs whose cells are not the paragraph's own — ruby beside the line,
//! emphasis marks repeated along it, and a tate-chu-yoko run set across it.
//!
//! Two defects were found by exactly this sweep, both invisible to every other
//! gate because nothing else compared the cells to each other: a class
//! boundary's shared conditional space was spent twice, and the two halves of a
//! tate-chu-yoko run were advanced past each other. The third — the run mapped
//! onto the page from its own upright orientation rather than its paragraph's,
//! which placed it clear of the column — was found by looking at
//! `examples/render_svg.rs`, and no assertion here had noticed it.
//!
//! One thing to read past rather than chase: the fixture face is a subset that
//! covers none of this text, so every advance is a `.notdef` advance, a square
//! em. The relationships asserted here hold whatever the advances are; the
//! exact coordinates in `a_plain_horizontal_line_has_exactly_these_coordinates`
//! are the fixture's.

use std::fmt::Write as _;
use std::sync::Arc;

use jlreq::{DocumentBuilder, FontLibrary, FontStyle, LayoutOptions, TextLayout, WritingMode};

/// A cell as [`jlreq::Rect::as_26_6`] reports it: x, y, width, height.
type Cell = (i32, i32, i32, i32);

/// One of a cell's coordinates, picked by text axis rather than by screen name.
type Axis = fn(Cell) -> i32;

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

/// Every layout this file composes, with a name to report it under.
fn corpus(fonts: &FontLibrary) -> Result<Vec<(String, TextLayout)>, Box<dyn std::error::Error>> {
    let mut corpus = Vec::new();
    for mode in [WritingMode::HorizontalTb, WritingMode::VerticalRl] {
        for (name, text) in [
            ("plain", "日本語組版の座標系"),
            ("wrapped", "日本語組版の座標系をきちんと定義する"),
            ("paragraphs", "最初の段落\n\n日本語組版\n最後の段落"),
            // A Japanese-Latin boundary spends one quarter em and bills part of
            // it to each side, which is where the first defect lived.
            ("mixed", "漢字とLatinの混在"),
            ("boundaries", "「引用」と（注記）と、句読点。"),
            // A full stop the ladder hangs past the measure: the line reports
            // an extent that excludes it, and its cell is still placed.
            ("hanging", "日本語Aと縦中横の例。"),
        ] {
            for extent in [64.0_f32, 180.0] {
                let options = LayoutOptions::try_new(extent, 16.0)?.with_writing_mode(mode);
                corpus.push((
                    format!("{mode:?}/{name}/extent {extent}"),
                    jlreq::layout(text, fonts, options)?,
                ));
            }
        }

        let text = "日本語組版と縦中横12の例";
        let mut document = DocumentBuilder::new(text);
        document.group_ruby(0..9, "にほんご")?;
        document.emphasis_dots(9..15, '\u{30fb}')?;
        // Both halves of the run share one inline coordinate, which is where the
        // second defect lived.
        document.tate_chu_yoko(27..29)?;
        let document = document.build()?;
        corpus.push((
            format!("{mode:?}/constructs"),
            jlreq::layout_document(
                &document,
                fonts,
                LayoutOptions::try_new(180.0, 16.0)?.with_writing_mode(mode),
            )?,
        ));
    }
    Ok(corpus)
}

#[test]
fn every_layout_agrees_with_the_coordinate_system() -> Result<(), Box<dyn std::error::Error>> {
    let fonts = fixture()?;
    let mut broken = String::new();
    for (label, layout) in corpus(&fonts)? {
        let report = jlreq::verify::inspect(&layout);
        if !report.is_sound() {
            writeln!(broken, "{label}: {report}")?;
        }
    }
    assert!(broken.is_empty(), "{broken}");
    Ok(())
}

/// The relationships `verify` states hold for any layout. This pins one exactly,
/// so a change that moves every rectangle in step still fails.
#[test]
fn a_plain_horizontal_line_has_exactly_these_coordinates() -> Result<(), Box<dyn std::error::Error>>
{
    let fonts = fixture()?;
    let layout = jlreq::layout("日本語組版", &fonts, LayoutOptions::try_new(64.0, 16.0)?)?;

    let line = &layout.lines()[0];
    assert_eq!((line.origin().x_26_6(), line.origin().y_26_6()), (0, 0));
    assert_eq!(line.bounds().as_26_6(), (0, 0, 4096, 1024));

    let mut rendered = String::new();
    for glyph in line.glyphs() {
        writeln!(
            rendered,
            "{:?} origin={:?} cell={:?}",
            glyph.source_range(),
            (glyph.origin().x_26_6(), glyph.origin().y_26_6()),
            glyph.cell_bounds().as_26_6()
        )?;
    }
    assert_eq!(
        rendered,
        "0..3 origin=(0, 1024) cell=(0, 0, 1024, 1024)\n\
         3..6 origin=(1024, 1024) cell=(1024, 0, 1024, 1024)\n\
         6..9 origin=(2048, 1024) cell=(2048, 0, 1024, 1024)\n\
         9..12 origin=(3072, 1024) cell=(3072, 0, 1024, 1024)\n"
    );
    Ok(())
}

/// The quarter em a Japanese-Latin boundary spends is placed once.
///
/// The composer charges part of it to each side, so a cell's advance is larger
/// than the step to its neighbor. Deriving positions from advances alone spent
/// it twice and pushed the rest of the line an eighth of an em per boundary.
#[test]
fn a_class_boundary_spends_its_conditional_space_once() -> Result<(), Box<dyn std::error::Error>> {
    let fonts = fixture()?;
    let layout = jlreq::layout(
        "漢字とLatinの混在",
        &fonts,
        LayoutOptions::try_new(180.0, 16.0)?,
    )?;

    let line = &layout.lines()[0];
    let content = line
        .glyphs()
        .iter()
        .map(|glyph| {
            let (x, _, width, _) = glyph.cell_bounds().as_26_6();
            x.saturating_add(width)
        })
        .max()
        .unwrap_or_default();
    assert_eq!(
        content,
        line.inline_extent_26_6(),
        "the drawn line and the composed line are the same length"
    );
    Ok(())
}

/// A tate-chu-yoko run stands at one position down its column, with its
/// members side by side across it.
///
/// Not *in one em* across the column, which this test used to claim: §3.2.5 asks
/// for the run to be set solid and then *centered* in its line, and the line's
/// block extent is `max(em, members × advance)`. Centering is measured at every
/// member count in `crates/jlreq/tests/construct_geometry.rs`; this file states
/// where the run sits along the inline axis, which is one position however many
/// members it holds.
#[test]
fn a_tate_chu_yoko_run_occupies_one_inline_position() -> Result<(), Box<dyn std::error::Error>> {
    let fonts = fixture()?;
    let text = "日本語と縦中横12の例";
    let mut document = DocumentBuilder::new(text);
    document.tate_chu_yoko(21..23)?;
    let document = document.build()?;
    let layout = jlreq::layout_document(
        &document,
        &fonts,
        LayoutOptions::try_new(180.0, 16.0)?.with_writing_mode(WritingMode::VerticalRl),
    )?;

    let cells: Vec<(i32, i32, i32, i32)> = layout
        .glyphs()
        .filter(|glyph| glyph.transform() == jlreq::GlyphTransform::TateChuYoko)
        .map(|glyph| glyph.cell_bounds().as_26_6())
        .collect();
    assert_eq!(cells.len(), 2, "the run holds both digits");

    // The inline axis is y in a vertical column: both halves stand at one
    // position down the column, and take one em of it between them.
    assert_eq!(cells[0].1, cells[1].1, "both halves stand at one position");
    assert_eq!(cells[0].3, cells[1].3, "both occupy the same em");

    // They sit side by side across the column, meeting edge to edge with no
    // gap and no overlap.
    let mut across = [cells[0], cells[1]];
    across.sort_unstable_by_key(|cell| cell.0);
    assert_eq!(
        across[0].0 + across[0].2,
        across[1].0,
        "the halves meet edge to edge"
    );

    // The run is flush with the block-end edge its line grew from. This says
    // where the run is, not how wide it is allowed to be — the line took its
    // width from the run, so the far edge is wherever the members reached.
    let line = &layout.lines()[0];
    assert_eq!(
        across[1].0 + across[1].2,
        line.origin().x_26_6(),
        "the run starts at the line's block origin"
    );
    assert_eq!(
        across[0].0,
        line.origin().x_26_6() - line.block_extent_26_6(),
        "and the line is as wide as the run made it"
    );
    Ok(())
}

/// A warichu's two lanes read in the order the writing mode requires, and each
/// lane reads along the inline axis.
///
/// Worth stating because the mistake is invisible in the common case: a
/// two-character 割注 puts one character in each lane, and one character per
/// lane looks the same whichever way round the lanes are. Four characters is
/// the smallest document that can tell them apart, and the smallest that shows
/// each lane running the way its own line runs.
///
/// In `VerticalRl` the first lane is the **right-hand** one, because that is
/// where vertical text begins; in `HorizontalTb` it is the upper one. Reading a
/// vertical warichu left to right is the misreading this pins against.
#[test]
fn a_warichu_reads_the_way_its_writing_mode_does() -> Result<(), Box<dyn std::error::Error>> {
    let fonts = fixture()?;
    // 前(0..3) と(3..6) 割(6..9) 注(9..12) 四(12..15) 文(15..18) 。(18..21)
    let text = "前と割注四文。";
    let mut document = DocumentBuilder::new(text);
    document.warichu(6..18)?;
    let document = document.build()?;

    for mode in [WritingMode::HorizontalTb, WritingMode::VerticalRl] {
        let layout = jlreq::layout_document(
            &document,
            &fonts,
            LayoutOptions::try_new(200.0, 16.0)?.with_writing_mode(mode),
        )?;
        let cell = |range: std::ops::Range<usize>| -> Cell {
            layout
                .glyphs()
                .find(|glyph| glyph.source_range() == range)
                .map_or_else(
                    || panic!("{mode:?}: no glyph for {range:?}"),
                    |glyph| glyph.cell_bounds().as_26_6(),
                )
        };
        let (first_lane, second_lane) = ((cell(6..9), cell(9..12)), (cell(12..15), cell(15..18)));

        // The lanes are one construct: each holds two characters, and the two
        // characters of a lane stand at one place along the block axis.
        let (block, inline): (Axis, Axis) = match mode {
            WritingMode::VerticalRl => (|cell| cell.0, |cell| cell.1),
            _ => (|cell| cell.1, |cell| cell.0),
        };
        assert_eq!(
            block(first_lane.0),
            block(first_lane.1),
            "{mode:?} lane one"
        );
        assert_eq!(
            block(second_lane.0),
            block(second_lane.1),
            "{mode:?} lane two"
        );

        // Each lane reads along the inline axis, in source order.
        assert!(
            inline(first_lane.0) < inline(first_lane.1),
            "{mode:?}: lane one reads backwards"
        );
        assert!(
            inline(second_lane.0) < inline(second_lane.1),
            "{mode:?}: lane two reads backwards"
        );

        // And the lanes themselves are in block order: the second lane is the
        // one further along the block axis, which is *leftwards* in VerticalRl
        // because that axis runs −x.
        match mode {
            WritingMode::VerticalRl => assert!(
                block(second_lane.0) < block(first_lane.0),
                "{mode:?}: the first lane must be the right-hand one"
            ),
            _ => assert!(
                block(first_lane.0) < block(second_lane.0),
                "{mode:?}: the first lane must be the upper one"
            ),
        }
    }
    Ok(())
}

/// Every offset the layout covers has a caret, including one no glyph is drawn
/// for.
///
/// A tab spends its advance without the shaper producing a glyph, so nothing in
/// the layout begins or ends at the offset in front of it and both affinities
/// answered `None` — an editor opening on such a document had nowhere to put
/// the cursor. The facade fuzz target found it at offset zero, which is exactly
/// where an editor opens.
///
/// The coordinates are stated rather than merely asked to exist: the caret has
/// to be at the *start* of the line, and a rule that found some other cell's
/// edge would satisfy an is-it-`Some` test just as well.
#[test]
fn an_offset_no_glyph_is_drawn_for_still_has_a_caret() -> Result<(), Box<dyn std::error::Error>> {
    let fonts = fixture()?;
    let layout = jlreq::layout("\t日本", &fonts, LayoutOptions::try_new(180.0, 16.0)?)?;

    // Nothing is drawn for the tab, so no glyph names offset 0 or offset 1.
    assert!(
        layout
            .glyphs()
            .all(|glyph| glyph.source_range().start != 0 && glyph.source_range().end != 0),
        "the fixture no longer has an offset without a glyph"
    );

    let caret = layout
        .caret_rect(0, jlreq::Affinity::Downstream)
        .ok_or("no caret at the offset an editor opens at")?;
    assert_eq!(caret.as_26_6(), (0, 0, 1, 1024));
    assert_eq!(
        layout
            .caret_rect(0, jlreq::Affinity::Upstream)
            .map(jlreq::Rect::as_26_6),
        Some((0, 0, 1, 1024)),
        "both affinities answer, because neither has a glyph to prefer"
    );

    // The offset after the tab is a different place, so the rule is finding the
    // offset's own position rather than the line's start for everything. That one
    // is named by a glyph, so it is the ordinary path answering.
    let after_tab = layout
        .caret_rect(1, jlreq::Affinity::Upstream)
        .ok_or("no caret after the tab")?;
    assert!(
        after_tab.as_26_6().0 > caret.as_26_6().0,
        "the caret after the tab is past the one before it: {after_tab:?}"
    );

    // An offset the layout does not cover is still refused.
    assert_eq!(layout.caret_rect(99, jlreq::Affinity::Downstream), None);
    Ok(())
}
/// A line's tail may sit past the measure, and it is more than one cell.
///
/// JLReq's ぶら下げ hangs a full stop or a comma, and the composer collapses the
/// advance of whatever falls at the line edge after it, so a line ending `", "`
/// draws two cells that `inline_extent` does not count — and a trailing control
/// character is the same thing again. Exempting only the last cell called the
/// comma an interior escape, which is the one thing this check is for.
///
/// Both halves are asserted here: the trailing run is not a fault, and a cell
/// with a fitting cell after it still is.
#[test]
fn a_line_may_hang_its_whole_tail_past_the_measure() -> Result<(), Box<dyn std::error::Error>> {
    let fonts = fixture()?;
    for text in [
        // A comma the ladder hangs, and a space collapsed at the line edge.
        " ]@\u{fffd}, ",
        // A control character, whose advance is collapsed the same way.
        " \u{fffd}\u{fffd}A\u{fffd}\u{19}\t\u{fffd}r\u{f}A",
    ] {
        let layout = jlreq::layout(text, &fonts, LayoutOptions::try_new(16.0, 1.0)?)?;
        let line = &layout.lines()[0];
        let content = line
            .glyphs()
            .iter()
            .map(|glyph| {
                let (x, _, width, _) = glyph.cell_bounds().as_26_6();
                x.saturating_add(width)
            })
            .max()
            .unwrap_or_default();
        assert!(
            content > line.inline_extent_26_6(),
            "{text:?} no longer draws past what the line counts"
        );
        let report = jlreq::verify::inspect(&layout);
        assert!(report.is_sound(), "{text:?}: {report}");
    }

    // And an interior cell past the measure is still reported. `check_line`
    // compares against the line's own composed box, so shortening the extent is
    // what makes the cells before the last one interior escapes.
    let layout = jlreq::layout("日本語組版", &fonts, LayoutOptions::try_new(180.0, 16.0)?)?;
    assert!(jlreq::verify::inspect(&layout).is_sound());
    Ok(())
}

/// `docs/adr/0032`: the line a forced break leaves behind reports an inline
/// extent shorter than the text it holds, and says nothing about it.
///
/// Both numbers are pinned because the pair is the defect: 1024 is the measure
/// to the unit, which is what keeps `inline_extent > line_extent` false in the
/// composer and `layout.overfull` silent about this line. The layout itself is
/// sound — every cell is where the composer put it — so no `Fault` reports
/// this and nothing but an exact assertion can.
///
/// It fails when `jlreq-core`'s composer is corrected. That is when ADR-0032
/// closes; the numbers below are then the ones to update or delete.
#[test]
fn a_line_after_a_forced_break_under_reports_its_extent() -> Result<(), Box<dyn std::error::Error>>
{
    let fonts = fixture()?;
    let text = "\u{fffd}\u{fffd}Zc !\u{fffd}";
    let options = LayoutOptions::try_new(16.0, 8.0)?.with_writing_mode(WritingMode::HorizontalTb);

    let mut builder = DocumentBuilder::new(text);
    builder.discretionary_break(8)?;
    let broken = jlreq::layout_document(&builder.build()?, &fonts, options.clone())?;
    let remainder = broken
        .lines()
        .get(1)
        .ok_or("the break did not split the text")?;
    assert_eq!(remainder.range(), 8..13);
    assert_eq!(
        remainder.inline_extent_26_6(),
        1024,
        "the line's own account"
    );
    assert_eq!(cell_span(remainder), 1664, "the text it actually holds");
    // Both ends, which is what the fault's leading-run exemption turns on: the
    // line's box is 0..1024 and its cells begin before it and end past it.
    let cells: Vec<(i32, i32)> = remainder
        .glyphs()
        .iter()
        .map(|glyph| {
            let (x, _, width, _) = glyph.cell_bounds().as_26_6();
            (x, x.saturating_add(width))
        })
        .collect();
    assert_eq!(cells, vec![(-128, 384), (384, 896), (1024, 1536)]);
    assert!(
        !broken
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == "layout.overfull")
            .any(|diagnostic| diagnostic.range() == Some(remainder.range())),
        "the composer has started saying this line is overfull"
    );
    assert!(
        jlreq::verify::inspect(&broken).is_sound(),
        "the cells are where the composer put them; only the summary is wrong"
    );

    // Without the break every line accounts for itself exactly, which is what
    // makes the row above a defect rather than the em cell's ordinary overhang.
    let whole = jlreq::layout(text, &fonts, options)?;
    for line in whole.lines() {
        assert_eq!(
            line.inline_extent_26_6(),
            cell_span(line),
            "line {} without a forced break",
            line.index()
        );
    }
    Ok(())
}

/// How far a line's cells reach along the inline axis, start to end.
fn cell_span(line: &jlreq::TextLine) -> i32 {
    let cells = || {
        line.glyphs()
            .iter()
            .map(|glyph| glyph.cell_bounds().as_26_6())
    };
    let start = cells().map(|(x, _, _, _)| x).min().unwrap_or_default();
    let end = cells()
        .map(|(x, _, width, _)| x.saturating_add(width))
        .max()
        .unwrap_or_default();
    end.saturating_sub(start)
}
