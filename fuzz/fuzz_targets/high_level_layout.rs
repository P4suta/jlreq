// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]

use std::hint::black_box;
use std::sync::Arc;

use jlreq::{BaseDirection, FontLibrary, LayoutEngine, LayoutOptions, ResourceLimits, WritingMode};
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
        .writing_mode(if byte(4) & 1 == 0 {
            WritingMode::HorizontalTb
        } else {
            WritingMode::VerticalRl
        })
        .base_direction(match byte(5) % 3 {
            0 => BaseDirection::Auto,
            1 => BaseDirection::LeftToRight,
            _ => BaseDirection::RightToLeft,
        })
        .limits(
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

    let mut valid_fonts = FontLibrary::new();
    if valid_fonts
        .register_font(Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF))
        .is_ok()
    {
        let mut engine = LayoutEngine::new();
        let _ = black_box(engine.layout(&text, &valid_fonts, options.clone()));
        let _ = black_box(engine.layout("再利用", &valid_fonts, options.clone()));
    }

    let mut arbitrary_fonts = FontLibrary::new();
    if arbitrary_fonts
        .register_font(Arc::<[u8]>::from(data))
        .is_ok()
    {
        let _ = black_box(jlreq::layout(&text, &arbitrary_fonts, options));
    }
});
