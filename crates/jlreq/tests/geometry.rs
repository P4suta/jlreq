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
/// Not *in one em* across the column, which this test used to claim: the line's
/// block extent is derived from the run, so the run spanning its column exactly
/// is true of any run whatever its members measure, and asserting it checks
/// nothing. §3.2.5 asks for the run to be *centred* in the line, which is a
/// different statement and one the run does not satisfy at any member count but
/// two — `crates/jlreq/tests/construct_geometry.rs` measures that.
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
