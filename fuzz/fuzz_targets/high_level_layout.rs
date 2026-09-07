// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]

use std::hint::black_box;
use std::sync::Arc;

use jlreq::{
    Affinity, BaseDirection, DocumentBuilder, FontLibrary, LayoutEngine, LayoutOptions,
    ParagraphStyle, ResourceLimits, SpanStyle, TabAlignment, TabStop, Widow, WritingMode,
};
use libfuzzer_sys::fuzz_target;

const MAX_TEXT_BYTES: usize = 16 * 1024;

/// The most lines this target will check the geometry of.
///
/// `jlreq::verify::inspect` asks `hit_test` and `caret_rect` once per glyph and
/// per line edge, and each of those scans the layout, so a very tall layout is
/// quadratic to check. Every layout is still composed; only the check is capped,
/// and the cap is well above the shapes a wrapped paragraph actually produces.
const MAX_VERIFIED_LINES: usize = 64;

/// Every layout the facade returns must be geometrically self-consistent: the
/// cells a renderer draws into are the cells hit testing measures against and
/// the cells carets are cut from. A fuzzer is the only thing here that tries
/// inputs nobody thought to write a test for.
fn check_geometry(layout: &jlreq::TextLayout) {
    if layout.lines().len() > MAX_VERIFIED_LINES {
        return;
    }
    let report = jlreq::verify::inspect(layout);
    let unexplained: Vec<_> = report
        .faults()
        .iter()
        .filter(|fault| {
            // The exemption is for one fault on one shape, not for every fault
            // a line of that shape can produce. Widening it to the line would
            // hide a cell, an annotation or a hit-testing defect that happened
            // to land on a reordered construct.
            if !matches!(fault, jlreq::verify::Fault::CellEscapesTheMeasureSilently { .. }) {
                return true;
            }
            let line = fault.line().and_then(|line| layout.lines().get(line));
            !line.is_some_and(deferred_by_adr_0031)
        })
        .collect();
    assert!(
        unexplained.is_empty(),
        "{:?}\n{unexplained:?}",
        layout.source()
    );
}

/// A line that bidi reordering shuffled *and* that holds a construct.
///
/// The facade lays a line out by walking its cells in visual order with a
/// cumulative cursor. A warichu or furawake lane restarts behind its
/// predecessor, and the cursor can only take that step back when the two cells
/// stay next to each other in the walk — which reordering is free to undo. The
/// lanes then sit end to end and run past the measure, which is what this
/// construct did *everywhere* before `docs/adr/0030` and now does only here.
///
/// `docs/adr/0031` records it; `a_furawake_in_a_reordered_line_is_still_wrong`
/// in `crates/jlreq/tests/construct_geometry.rs` pins it deterministically, so
/// this exemption cannot outlive the defect unnoticed. The condition is the
/// real one — a reordered line holding a construct — rather than
/// `BaseDirection::RightToLeft`, which `Auto` would walk straight around.
fn deferred_by_adr_0031(line: &jlreq::TextLine) -> bool {
    let reordered = line.glyphs().iter().any(|glyph| glyph.bidi_level() % 2 == 1);
    reordered && line.glyphs().iter().any(|glyph| glyph.construct().is_some())
}

fuzz_target!(|data: &[u8]| {
    let controls = data.get(..8).unwrap_or(data);
    let body = data.get(controls.len()..).unwrap_or_default();
    let text_bytes = body.get(..body.len().min(MAX_TEXT_BYTES)).unwrap_or_default();
    let text = String::from_utf8_lossy(text_bytes);
    let byte = |index: usize| controls.get(index).copied().unwrap_or_default();
    let line_extent = match byte(0) % 5 {
        0 => f32::NAN,
        1 => f32::INFINITY,
        2 => 0.0,
        3 => 1.0,
        _ => f32::from(byte(1)).mul_add(8.0, 16.0),
    };
    let font_size = match byte(2) % 4 {
        0 => f32::NEG_INFINITY,
        1 => 0.0,
        2 => 1.0,
        _ => f32::from(byte(3)).mul_add(0.25, 4.0),
    };
    let Ok(options) = LayoutOptions::try_new(line_extent, font_size) else {
        return;
    };
    let options = options
        .with_writing_mode(if byte(4) & 1 == 0 {
            WritingMode::HorizontalTb
        } else {
            WritingMode::VerticalRl
        })
        .with_base_direction(match byte(5) % 3 {
            0 => BaseDirection::Auto,
            1 => BaseDirection::LeftToRight,
            _ => BaseDirection::RightToLeft,
        })
        .with_limits(
            ResourceLimits::default()
                .with_max_input_bytes(text.len().max(MAX_TEXT_BYTES))
                .with_max_fonts(4)
                .with_max_font_bytes(2 * 1024 * 1024)
                .with_max_paragraphs(512)
                .with_max_runs(16 * 1024)
                .with_max_glyphs(64 * 1024)
                .with_max_constructs(512)
                .with_max_core_operations(250_000),
        );

    let options = options
        .with_widow(if byte(6) & 1 == 0 {
            Widow::Allow
        } else {
            Widow::MinimumClusters(u16::from(byte(6)))
        })
        .with_first_line_indent(f32::from(byte(7) % 32))
        .unwrap_or_else(|_| LayoutOptions::try_new(64.0, 8.0).unwrap());
    let options = match TabStop::try_new(f32::from(byte(5)).mul_add(4.0, 1.0), TabAlignment::Character('.')) {
        Ok(stop) => options.with_tab_stops([stop]),
        Err(_) => options,
    };

    let mut valid_fonts = FontLibrary::new();
    if valid_fonts
        .register_font(Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF))
        .is_ok()
    {
        let mut engine = LayoutEngine::new();
        if let Ok(layout) = black_box(engine.layout(&text, &valid_fonts, options.clone())) {
            check_geometry(&layout);
            // Exercise the editing surface with arbitrary offsets: every call
            // must be total over any (offset, affinity) pair.
            let offset = usize::from(byte(1)).saturating_mul(usize::from(byte(3)));
            let affinity = if byte(2) & 1 == 0 {
                Affinity::Upstream
            } else {
                Affinity::Downstream
            };
            let _ = black_box(layout.line_index_at(offset));
            let _ = black_box(layout.next_grapheme_boundary(offset));
            let _ = black_box(layout.prev_grapheme_boundary(offset));
            let _ = black_box(layout.word_range_at(offset));
            let _ = black_box(layout.sentence_range_at(offset));
            let _ = black_box(layout.next_visual_caret(offset, affinity));
            let _ = black_box(layout.prev_visual_caret(offset, affinity));
            let _ = black_box(layout.caret_previous_line(offset, affinity));
            let _ = black_box(layout.caret_next_line(offset, affinity));
            let end = offset.min(layout.source().len());
            let _ = black_box(layout.selection_rects_filled(0..end));
        }
        let _ = black_box(engine.layout("再利用", &valid_fonts, options.clone()));

        // Typed documents from arbitrary text: spans, paragraph styles, an
        // automatic furawake, and a discretionary break, all validated.
        let mut builder = DocumentBuilder::new(text.as_ref());
        let half = text.len() / 2;
        let _ = builder.span(0..half.max(1), SpanStyle::new().with_family("Fuzz"));
        let _ = builder.paragraph_style(
            0..text.len().max(1),
            ParagraphStyle::new().with_alignment(jlreq::Alignment::Center),
        );
        let _ = builder.furawake(0..half.max(1), 2 + u16::from(byte(0) % 3), 0.5);
        let _ = builder.discretionary_break(usize::from(byte(4)));
        if let Ok(document) = builder.build() {
            if let Ok(layout) =
                black_box(engine.layout_document(&document, &valid_fonts, options.clone()))
            {
                check_geometry(&layout);
            }
        }
    }

    let mut arbitrary_fonts = FontLibrary::new();
    if arbitrary_fonts
        .register_font(Arc::<[u8]>::from(data))
        .is_ok()
    {
        if let Ok(foreign) = valid_fonts.register_font(Arc::<[u8]>::from(
            font_test_data::NOTO_SANS_JP_CFF,
        )) {
            // Cross-library identifiers must resolve to None, never a wrong font.
            let _ = black_box(arbitrary_fonts.get(foreign));
        }
        let _ = black_box(jlreq::layout(&text, &arbitrary_fonts, options));
    }
});
