// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Draw a layout, so the coordinate contract can be checked by looking at it.
//!
//! Usage: `cargo run -p jlreq --example render_svg -- <font.otf> [out.svg]`
//!
//! Every rectangle is a cell the layout reported and every glyph is placed at the
//! point [`baseline_origin`] derives, so a wrong reading of the contract is visible
//! rather than merely arguable: the text would sit off its own cells.
//!
//! Each line is drawn twice. The solid box is the line's **composed box** — its
//! origin plus the extents it reports — which is the claim, and the dashed one,
//! drawn only when it differs, is [`jlreq::TextLine::bounds`], the union of every
//! cell in the line. Drawing only the union would hide the very thing this file
//! exists to show: a union contains every cell by construction, so a cell that
//! left its line would still look enclosed. Ruby stands outside the solid box on
//! purpose; a body cell outside it is a defect, and `jlreq::verify` names it.

use std::error::Error;
use std::fmt::Write as _;

use jlreq::{
    DocumentBuilder, FontLibrary, FontMetrics, FontResource, GlyphPlacement, GlyphTransform,
    LayoutOptions, TextLayout, WritingMode,
};

/// The annotation stream of each construct this example adds, by ordinal.
///
/// An annotation glyph's `source_range` indexes its construct's own stream
/// rather than the paragraph, so nothing but the writer of the document can
/// turn one back into characters. Only the ruby has a stream; a tate-chu-yoko
/// run and a warichu are body text set differently.
const ANNOTATIONS: [&str; 3] = ["にほんご", "", ""];

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("pass a TTF, OTF, or TTC path")?;
    let out = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "layout.svg".to_owned());

    let face = std::fs::read(&path)?;
    let mut fonts = FontLibrary::new();
    fonts.register_font(face.clone())?;

    // 27 bytes of kanji, a separator, then three kanji, two digits set as a
    // tate-chu-yoko run, and a warichu. Every construct whose cells are not the
    // paragraph's own is on the page, because those are the ones a wrong
    // reading of the contract puts somewhere else entirely.
    let text = "日本語組版の座標系\n漢字と12と割注。";
    let mut document = DocumentBuilder::new(text);
    document.group_ruby(0..9, ANNOTATIONS[0])?;
    document.tate_chu_yoko(37..39)?;
    document.warichu(42..48)?;
    let document = document.build()?;

    let options = LayoutOptions::try_new(180.0, 24.0)?.with_line_gap(8.0)?;
    let horizontal = jlreq::layout_document(&document, &fonts, options.clone())?;
    let vertical = jlreq::layout_document(
        &document,
        &fonts,
        options.clone().with_writing_mode(WritingMode::VerticalRl),
    )?;
    // The same document with §3.3.3's 三分ルビ. Its reading is set at the same
    // size down the block axis and narrowed to a third across the inline one,
    // so the two horizontal panels differ in exactly one thing and the
    // condensation is the thing you are looking at.
    let condensed = jlreq::layout_document(
        &document,
        &fonts,
        options.with_ruby_scale(jlreq::RubyScale::THIRD),
    )?;

    let mut svg = String::new();
    writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"760\" height=\"560\" \
         viewBox=\"0 0 760 560\">"
    )?;
    writeln!(
        svg,
        "<rect width=\"100%\" height=\"100%\" fill=\"#fbfbf8\"/>"
    )?;
    // Embedding the very bytes the layout was measured from is what makes this a
    // witness and not a picture: the shapes on the page came from those advances.
    writeln!(
        svg,
        "<style>@font-face{{font-family:jlreq-fixture;src:url(data:font/otf;base64,{});}}\
         text{{font-family:jlreq-fixture,sans-serif}}</style>",
        base64(&face)
    )?;

    draw(&mut svg, &horizontal, 60.0, 70.0, "horizontal-tb")?;
    draw(&mut svg, &vertical, 700.0, 70.0, "vertical-rl")?;
    draw(
        &mut svg,
        &condensed,
        60.0,
        340.0,
        "horizontal-tb, RubyScale::THIRD (三分ルビ)",
    )?;

    writeln!(svg, "</svg>")?;
    std::fs::write(&out, svg)?;
    println!("wrote {out}");
    Ok(())
}

/// The box a line claims: its origin plus the extents it reports, in 26.6.
///
/// Not [`jlreq::TextLine::bounds`], which unions in every cell it is asked
/// about and so can never show one leaving.
fn composed_box(line: &jlreq::TextLine) -> (i32, i32, i32, i32) {
    let (x, y) = (line.origin().x_26_6(), line.origin().y_26_6());
    let (inline, block) = (line.inline_extent_26_6(), line.block_extent_26_6());
    match line.writing_mode() {
        WritingMode::VerticalRl => (x.saturating_sub(block), y, block, inline),
        _ => (x, y, inline, block),
    }
}

/// 26.6 fixed point as the number an SVG attribute takes.
fn fixed(value: i32) -> f64 {
    f64::from(value) / 64.0
}

/// Translate one layout's own coordinates onto the page and draw every cell, then
/// the text itself at the point the drawing contract names.
fn draw(
    svg: &mut String,
    layout: &TextLayout,
    origin_x: f32,
    origin_y: f32,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    writeln!(svg, "<g transform=\"translate({origin_x},{origin_y})\">")?;
    writeln!(
        svg,
        "<text x=\"0\" y=\"-16\" font-size=\"13\" fill=\"#777\" \
         font-family=\"sans-serif\">{label}</text>"
    )?;

    for line in layout.lines() {
        let claimed = composed_box(line);
        writeln!(
            svg,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" \
             stroke=\"#7fa8d0\" stroke-width=\"1\"/>",
            fixed(claimed.0),
            fixed(claimed.1),
            fixed(claimed.2),
            fixed(claimed.3)
        )?;
        let bounds = line.bounds().as_26_6();
        if bounds != claimed {
            writeln!(
                svg,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" \
                 stroke=\"#b9b9c8\" stroke-width=\"0.7\" stroke-dasharray=\"3 2\"/>",
                fixed(bounds.0),
                fixed(bounds.1),
                fixed(bounds.2),
                fixed(bounds.3)
            )?;
        }
    }

    for glyph in layout.glyphs() {
        let cell = glyph.cell_bounds();
        writeln!(
            svg,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\" \
             stroke-width=\"0.6\"/>",
            cell.x(),
            cell.y(),
            cell.width(),
            cell.height(),
            if glyph.annotation().is_some() {
                "#d09090"
            } else {
                "#8fbf8f"
            }
        )?;
    }

    for line in layout.lines() {
        for glyph in line.glyphs() {
            let Some(text) = glyph_text(layout, glyph) else {
                continue;
            };
            let (x, y) = text_origin(layout, glyph);
            // One `transform`, built from both things that can move a glyph.
            // SVG takes the attribute once; emitting it twice silently drops
            // one, which is the sort of thing this example exists to not do.
            let mut operations = Vec::new();
            if glyph.transform() == GlyphTransform::RotateClockwise {
                operations.push(format!("rotate(90 {x} {y})"));
            }
            // One character size is two numbers: the face is set at
            // `font_size` and the inline axis is narrowed to `inline_size`,
            // which is a third of the base em for JLReq §3.3.3's 三分ルビ and
            // equal to it for everything else. Drawing without this puts a
            // half-em outline in a third of an em of advance — drawn text off
            // its own cells, which is what this file makes visible.
            if let Some(factor) = condensation(glyph) {
                let axis = match glyph.writing_mode() {
                    WritingMode::VerticalRl => format!("1 {factor}"),
                    _ => format!("{factor} 1"),
                };
                operations.push(format!(
                    "translate({x} {y}) scale({axis}) translate({} {})",
                    -x, -y
                ));
            }
            let transform = if operations.is_empty() {
                String::new()
            } else {
                format!(" transform=\"{}\"", operations.join(" "))
            };
            writeln!(
                svg,
                "<text x=\"{x}\" y=\"{y}\" font-size=\"{}\" fill=\"#1a1a1a\"{transform}>{}</text>",
                glyph.font_size(),
                escape(text)
            )?;
            writeln!(
                svg,
                "<circle cx=\"{x}\" cy=\"{y}\" r=\"1.3\" fill=\"#c04040\"/>"
            )?;
        }
    }

    writeln!(svg, "</g>")?;
    Ok(())
}

/// Where to put the glyph so that it lands on its own cell.
///
/// [`GlyphPlacement::origin`] is the cell's inline-start, block-end corner — not
/// the font's baseline. For horizontal text the baseline is one em-relative
/// descent away from that corner, and `FontMetrics::descent` is negative, which
/// is exactly the correction. For vertical text the cell is what an upright
/// glyph is centered in, so the example places it from the cell and says so
/// rather than pretending a horizontal baseline is meaningful there.
fn text_origin(layout: &TextLayout, glyph: &GlyphPlacement) -> (f32, f32) {
    let cell = glyph.cell_bounds();
    let descent = layout
        .font(glyph.font_id())
        .and_then(FontResource::metrics)
        .map_or(0.0, FontMetrics::descent);
    (
        cell.x(),
        cell.y() + cell.height() + descent * glyph.font_size(),
    )
}

fn glyph_text<'a>(layout: &'a TextLayout, glyph: &GlyphPlacement) -> Option<&'a str> {
    let Some(annotation) = glyph.annotation() else {
        return layout.source().get(glyph.source_range());
    };
    // An annotation glyph is attributed to its construct's own stream, not to
    // the paragraph, so the paragraph source cannot resolve it. This example
    // wrote the document, so it is the one thing here that knows the readings.
    ANNOTATIONS
        .get(annotation.construct())
        .and_then(|stream: &&str| stream.get(annotation.range()))
}

/// Minimal RFC 4648 encoder, so the example keeps the workspace's no-extra-dependency rule.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3).saturating_mul(4));
    for chunk in bytes.chunks(3) {
        let mut block = [0_u8; 3];
        block[..chunk.len()].copy_from_slice(chunk);
        let packed = (u32::from(block[0]) << 16) | (u32::from(block[1]) << 8) | u32::from(block[2]);
        for index in 0..4_u32 {
            if index as usize <= chunk.len() {
                let value = (packed >> (18_u32.saturating_sub(index.saturating_mul(6)))) & 0x3f;
                out.push(char::from(ALPHABET[value as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// How far the inline axis is narrowed, where it is narrowed at all.
///
/// `None` for a square size, which is every glyph unless the caller declared a
/// [`jlreq::RubyScale`] that is not `HALF`. `docs/design/geometry.md` states
/// the ratio; this is that sentence, executed.
fn condensation(glyph: &jlreq::GlyphPlacement) -> Option<f32> {
    let block = glyph.font_size_26_6();
    let inline = glyph.inline_size_26_6();
    if inline == block || block <= 0 {
        return None;
    }
    Some(glyph.inline_size() / glyph.font_size())
}
