// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Draw a layout, so the coordinate contract can be checked by looking at it.
//!
//! Usage: `cargo run -p jlreq --example render_svg -- <font.otf> [out.svg]`
//!
//! Every rectangle is a cell the layout reported and every glyph is placed at the
//! point [`baseline_origin`] derives, so a wrong reading of the contract is visible
//! rather than merely arguable: the text would sit off its own cells.

use std::error::Error;
use std::fmt::Write as _;

use jlreq::{
    DocumentBuilder, FontLibrary, FontMetrics, FontResource, GlyphPlacement, GlyphTransform,
    LayoutOptions, TextLayout, WritingMode,
};

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
    document.group_ruby(0..9, "にほんご")?;
    document.tate_chu_yoko(37..39)?;
    document.warichu(42..48)?;
    let document = document.build()?;

    let options = LayoutOptions::try_new(180.0, 24.0)?.with_line_gap(8.0)?;
    let horizontal = jlreq::layout_document(&document, &fonts, options.clone())?;
    let vertical = jlreq::layout_document(
        &document,
        &fonts,
        options.with_writing_mode(WritingMode::VerticalRl),
    )?;

    let mut svg = String::new();
    writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"760\" height=\"360\" \
         viewBox=\"0 0 760 360\">"
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

    writeln!(svg, "</svg>")?;
    std::fs::write(&out, svg)?;
    println!("wrote {out}");
    Ok(())
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
        let bounds = line.bounds();
        writeln!(
            svg,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" \
             stroke=\"#7fa8d0\" stroke-width=\"1\"/>",
            bounds.x(),
            bounds.y(),
            bounds.width(),
            bounds.height()
        )?;
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
            let rotate = if glyph.transform() == GlyphTransform::RotateClockwise {
                format!(" transform=\"rotate(90 {x} {y})\"")
            } else {
                String::new()
            };
            writeln!(
                svg,
                "<text x=\"{x}\" y=\"{y}\" font-size=\"{}\" fill=\"#1a1a1a\"{rotate}>{}</text>",
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
    if glyph.annotation().is_some() {
        return None;
    }
    layout.source().get(glyph.source_range())
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
