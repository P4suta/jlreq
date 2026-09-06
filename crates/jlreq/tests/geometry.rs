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
//! Two defects were found by exactly this sweep, both of them invisible to
//! every other gate because nothing else compared the cells to each other:
//! a class boundary's shared conditional space was spent twice, and the two
//! halves of a tate-chu-yoko run were advanced past each other.

use std::fmt::Write as _;
use std::sync::Arc;

use jlreq::{DocumentBuilder, FontLibrary, FontStyle, LayoutOptions, TextLayout, WritingMode};

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

/// A tate-chu-yoko run stands in one em, however many characters it holds.
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

    let inline: Vec<i32> = layout
        .glyphs()
        .filter(|glyph| glyph.transform() == jlreq::GlyphTransform::TateChuYoko)
        .map(|glyph| glyph.cell_bounds().as_26_6().0)
        .collect();
    assert_eq!(inline.len(), 2, "the run holds both digits");
    assert_eq!(
        inline[0], inline[1],
        "both halves stand at the same inline coordinate"
    );
    Ok(())
}
