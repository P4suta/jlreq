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
            let _ = black_box(engine.layout_document(&document, &valid_fonts, options.clone()));
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
