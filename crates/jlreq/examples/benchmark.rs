// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reproducible median-time smoke benchmarks for the complete layout stack.
//!
//! Run with `cargo run -p jlreq --release --example benchmark`. Set
//! `JLREQ_BENCH_SAMPLES` to an odd positive sample count when collecting a
//! comparison; the default is nine measured samples after two warmups.

use std::{
    env,
    error::Error,
    hint::black_box,
    sync::Arc,
    time::{Duration, Instant},
};

use jlreq::{
    BaseDirection, Document, DocumentBuilder, FontLibrary, FontStyle, LayoutEngine, LayoutOptions,
    ScriptPosition, SpanStyle, WritingMode,
    core::{
        Break, Cluster, Composer, CompositionLimits, Construct, Frame, Paragraph, ShapedText, Size,
        Style,
    },
};

type BenchResult<T> = Result<T, Box<dyn Error>>;

fn main() -> BenchResult<()> {
    let samples = sample_count();
    println!("jlreq median benchmark ({samples} measured samples, release recommended)");

    let fonts = fixture_fonts()?;
    let plain = "日本語組版では、行長と約物の間隔を整える。".repeat(64);
    let plain_options = LayoutOptions::try_new(640.0, 16.0)?;

    measure("layout.one-shot.japanese", samples, || {
        let layout = jlreq::layout(black_box(&plain), &fonts, plain_options.clone())?;
        Ok(layout.glyphs().count())
    })?;

    let mut engine = LayoutEngine::new();
    measure("layout.reused-engine.japanese", samples, || {
        let layout = engine.layout(black_box(&plain), &fonts, plain_options.clone())?;
        Ok(layout.glyphs().count())
    })?;

    let bidi = "日本語 abc مرحبا 🇪🇨 ".repeat(48);
    let bidi_options = LayoutOptions::try_new(640.0, 18.0)?.base_direction(BaseDirection::Auto);
    measure("layout.reused-engine.bidi-fallback", samples, || {
        let layout = engine.layout(black_box(&bidi), &fonts, bidi_options.clone())?;
        Ok(layout.glyphs().count())
    })?;

    let span_text = "日本語ABC".repeat(96);
    measure("document.build.many-spans", samples, || {
        Ok(many_span_document(black_box(&span_text))?.text().len())
    })?;
    let span_document = many_span_document(&span_text)?;
    measure("layout.reused-engine.many-spans", samples, || {
        let layout =
            engine.layout_document(black_box(&span_document), &fonts, plain_options.clone())?;
        Ok(layout.glyphs().count())
    })?;

    let complex_document = complex_document(32)?;
    let vertical_options = LayoutOptions::try_new(640.0, 18.0)?
        .writing_mode(WritingMode::VerticalRl)
        .tab_width(4)?;
    measure("layout.reused-engine.vertical-constructs", samples, || {
        let layout = engine.layout_document(
            black_box(&complex_document),
            &fonts,
            vertical_options.clone(),
        )?;
        Ok(layout.glyphs().count())
    })?;

    let query_layout = engine.layout(&plain, &fonts, plain_options)?;
    let points: Vec<_> = query_layout
        .lines()
        .iter()
        .map(jlreq::TextLine::origin)
        .collect();
    let boundaries: Vec<_> = plain
        .char_indices()
        .map(|(offset, _)| offset)
        .chain([plain.len()])
        .collect();
    measure("result.hit-caret-selection", samples, || {
        let mut observed = 0_usize;
        for (ordinal, point) in points.iter().enumerate() {
            let hit = query_layout.hit_test(black_box(*point));
            observed = observed.saturating_add(hit.byte_offset());
            let boundary = cyclic_value(&boundaries, ordinal)?;
            observed = observed.saturating_add(usize::from(
                query_layout.caret_rect(black_box(boundary)).is_some(),
            ));
            let next = cyclic_value(&boundaries, ordinal.saturating_add(1))?;
            let range = boundary.min(next)..boundary.max(next);
            observed =
                observed.saturating_add(query_layout.selection_rects(black_box(range)).len());
        }
        Ok(observed)
    })?;

    let regular = core_paragraph(10_000, false)?;
    let core_style = Style::default();
    measure("core.compose-fresh.10k", samples, || {
        Ok(jlreq::core::compose(black_box(&regular), &core_style)?
            .lines()
            .len())
    })?;

    let mut composer = Composer::new();
    measure("core.composer-reused.10k", samples, || {
        Ok(composer
            .compose(black_box(&regular), &core_style)?
            .lines()
            .len())
    })?;

    let static_constructs = core_paragraph(1_000, true)?;
    composer.set_limits(CompositionLimits::DEFAULT.with_max_search_transitions(1_000_000_000));
    measure("core.static-construct-index.1k", samples, || {
        Ok(composer
            .compose(black_box(&static_constructs), &core_style)?
            .lines()
            .len())
    })?;
    Ok(())
}

fn sample_count() -> usize {
    let parsed = env::var("JLREQ_BENCH_SAMPLES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(9);
    if parsed.is_multiple_of(2) {
        parsed.saturating_add(1)
    } else {
        parsed
    }
}

fn measure(
    name: &str,
    samples: usize,
    mut operation: impl FnMut() -> BenchResult<usize>,
) -> BenchResult<Duration> {
    for _ in 0..2 {
        black_box(operation()?);
    }
    let mut elapsed = Vec::with_capacity(samples);
    for _ in 0..samples {
        let started = Instant::now();
        black_box(operation()?);
        elapsed.push(started.elapsed());
    }
    elapsed.sort_unstable();
    let median = elapsed[elapsed.len() / 2];
    println!("{name:40} {:>12} ns", median.as_nanos());
    Ok(median)
}

fn cyclic_value(values: &[usize], index: usize) -> BenchResult<usize> {
    let wrapped = index
        .checked_rem(values.len())
        .ok_or_else(|| std::io::Error::other("a benchmark value cycle is empty"))?;
    values
        .get(wrapped)
        .copied()
        .ok_or_else(|| std::io::Error::other("a benchmark value index is outside its cycle").into())
}

fn fixture_fonts() -> BenchResult<FontLibrary> {
    let mut fonts = FontLibrary::new();
    let _ = fonts.register_face(
        Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF),
        0,
        "Noto Sans JP",
        FontStyle::default(),
    )?;
    let _ = fonts.register_face(
        Arc::<[u8]>::from(rwml_fonts::noto_sans_arabic_subset()),
        0,
        "Vazirmatn",
        FontStyle::default(),
    )?;
    let _ = fonts.register_face(
        Arc::<[u8]>::from(font_test_data::NOTO_COLOR_EMOJI_FLAGS),
        0,
        "Noto Color Emoji",
        FontStyle::default(),
    )?;
    let _ = fonts.register_face(
        Arc::<[u8]>::from(font_test_data::TINOS_SUBSET),
        0,
        "Tinos",
        FontStyle::default(),
    )?;
    Ok(fonts)
}

fn many_span_document(text: &str) -> BenchResult<Document> {
    let mut boundaries: Vec<_> = text.char_indices().map(|(offset, _)| offset).collect();
    boundaries.push(text.len());
    let mut builder = DocumentBuilder::new(text);
    for ordinal in (0..boundaries.len().saturating_sub(1)).step_by(2) {
        let style = if ordinal.is_multiple_of(4) {
            SpanStyle::new().family("Noto Sans JP")
        } else {
            SpanStyle::new().family("Tinos")
        };
        let next = ordinal.saturating_add(1);
        if let Some((&start, &end)) = boundaries.get(ordinal).zip(boundaries.get(next)) {
            let _ = builder.span(start..end, style)?;
        }
    }
    Ok(builder.build()?)
}

fn complex_document(repetitions: usize) -> BenchResult<Document> {
    const SEGMENT: &str = "漢12強注式\t\n";
    let mut builder = DocumentBuilder::new(SEGMENT.repeat(repetitions));
    for repetition in 0..repetitions {
        let base = repetition.saturating_mul(SEGMENT.len());
        let _ = builder.group_ruby(base..base.saturating_add(3), "かん")?;
        let _ = builder.tate_chu_yoko(base.saturating_add(3)..base.saturating_add(5))?;
        let _ = builder.emphasis_dots(base.saturating_add(5)..base.saturating_add(8), '・')?;
        let _ = builder.warichu(base.saturating_add(8)..base.saturating_add(11))?;
        let _ = builder.script(
            base.saturating_add(11)..base.saturating_add(14),
            "2",
            ScriptPosition::Superscript,
        )?;
    }
    Ok(builder.build()?)
}

fn core_paragraph(cluster_count: usize, with_constructs: bool) -> BenchResult<Paragraph> {
    let source = "日".repeat(cluster_count);
    let ranges: Vec<_> = source
        .char_indices()
        .map(|(start, character)| start..start.saturating_add(character.len_utf8()))
        .collect();
    let clusters = ranges
        .iter()
        .cloned()
        .map(|range| Cluster::new(range, 1_000));
    let shaped = ShapedText::new(source, Size::square(1_000)?, Frame::FullEm, clusters)?;
    let breaks = ranges
        .iter()
        .skip(1)
        .map(|range| Break::allowed(range.start));
    let constructs: Vec<_> = if with_constructs {
        ranges
            .iter()
            .step_by(20)
            .cloned()
            .map(|range| Construct::emphasis_dots(range, '・'))
            .collect()
    } else {
        Vec::new()
    };
    Ok(Paragraph::builder(shaped, 20_000)
        .breaks(breaks)
        .constructs(constructs)
        .build()?)
}
