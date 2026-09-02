// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end acceptance tests for the draw-ready high-level API.

use std::error::Error;
use std::ops::Range;
use std::sync::Arc;

use jlreq::{
    Affinity, Alignment, BaseDirection, DiagnosticSeverity, DocumentBuilder, FontLibrary,
    FontSlant, FontStyle, FontVariation, GlyphTransform, LayoutEngine, LayoutError, LayoutOptions,
    OpenTypeFeature, OpenTypeTag, Point, Resource, ResourceLimits, RubyKind, RubyRun,
    ScriptPosition, SpanStyle, TextRole, WritingMode,
};

fn bytes(data: &'static [u8]) -> Arc<[u8]> {
    Arc::from(data)
}

fn fixture_fonts() -> Result<(FontLibrary, jlreq::FontId, jlreq::FontId), LayoutError> {
    let mut fonts = FontLibrary::new();
    fonts.register_face(
        bytes(font_test_data::NOTO_SANS_JP_CFF),
        0,
        "Noto Sans JP",
        FontStyle::default(),
    )?;
    let arabic = fonts.register_face(
        bytes(rwml_fonts::noto_sans_arabic_subset()),
        0,
        "Vazirmatn",
        FontStyle::default(),
    )?;
    let emoji = fonts.register_face(
        bytes(font_test_data::NOTO_COLOR_EMOJI_FLAGS),
        0,
        "Noto Color Emoji",
        FontStyle::default(),
    )?;
    fonts.register_face(
        bytes(font_test_data::TINOS_SUBSET),
        0,
        "Tinos",
        FontStyle::default(),
    )?;
    Ok((fonts, arabic, emoji))
}

fn range_of(text: &str, needle: &str) -> Result<Range<usize>, Box<dyn Error>> {
    let start = text
        .find(needle)
        .ok_or_else(|| std::io::Error::other(format!("{needle:?} is absent")))?;
    Ok(start..start.saturating_add(needle.len()))
}

fn expected_layout_error<T>(result: Result<T, LayoutError>) -> Result<LayoutError, Box<dyn Error>> {
    match result {
        Err(error) => Ok(error),
        Ok(_) => Err(Box::new(std::io::Error::other(
            "operation unexpectedly succeeded",
        ))),
    }
}

#[test]
fn plain_horizontal_and_vertical_text_are_draw_ready() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    let text = "日本語A12";
    let horizontal = jlreq::layout(text, &fonts, LayoutOptions::try_new(240.0, 16.0)?)?;
    assert!(!horizontal.lines().is_empty());
    assert!(!horizontal.fonts().is_empty());
    assert!(horizontal.glyphs().all(|glyph| {
        let range = glyph.source_range();
        range.start <= range.end && range.end <= text.len()
    }));

    let vertical = jlreq::layout(
        text,
        &fonts,
        LayoutOptions::try_new(240.0, 16.0)?.with_writing_mode(WritingMode::VerticalRl),
    )?;
    assert!(
        vertical
            .glyphs()
            .any(|glyph| glyph.transform() == GlyphTransform::RotateClockwise),
        "Latin text is rotated in vertical composition"
    );
    assert!(vertical.lines().iter().all(|line| {
        line.writing_mode() == WritingMode::VerticalRl && line.origin().x_26_6() <= 0
    }));

    assert_eq!(horizontal.writing_mode(), WritingMode::HorizontalTb);
    assert_eq!(vertical.writing_mode(), WritingMode::VerticalRl);
    assert_eq!(vertical.options().writing_mode(), WritingMode::VerticalRl);
    assert!((vertical.options().line_extent() - 240.0).abs() < f32::EPSILON);
    let relayout = jlreq::layout(text, &fonts, vertical.options().clone())?;
    assert_eq!(relayout, vertical);
    Ok(())
}

#[test]
fn bidi_fallback_and_missing_glyphs_preserve_source_ranges() -> Result<(), Box<dyn Error>> {
    let (fonts, arabic, emoji) = fixture_fonts()?;
    let text = "日本語 abc مرحبا 🇪🇨";
    let layout = jlreq::layout(
        text,
        &fonts,
        LayoutOptions::try_new(640.0, 18.0)?.with_base_direction(BaseDirection::Auto),
    )?;
    assert!(layout.glyphs().any(|glyph| glyph.bidi_level() % 2 == 1));
    assert!(layout.glyphs().any(|glyph| glyph.font_id() == arabic));
    assert!(layout.glyphs().any(|glyph| glyph.font_id() == emoji));
    assert!(
        layout
            .glyphs()
            .all(|glyph| glyph.source_range().end <= text.len())
    );

    let missing_text = "\u{10ffff}";
    let missing = jlreq::layout(missing_text, &fonts, LayoutOptions::try_new(80.0, 16.0)?)?;
    assert!(
        missing
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "font.missing-glyph")
    );
    assert!(missing.glyphs().all(|glyph| glyph.source_range() == (0..4)));
    Ok(())
}

#[test]
fn sparse_retained_font_ids_are_resolved_through_layout_lookup() -> Result<(), Box<dyn Error>> {
    let (fonts, arabic, emoji) = fixture_fonts()?;
    let layout = jlreq::layout("مرحبا", &fonts, LayoutOptions::try_new(240.0, 18.0)?)?;
    assert_eq!(layout.fonts().len(), 1);
    assert_eq!(layout.fonts()[0].id(), arabic);
    assert!(layout.font(emoji).is_none());
    for glyph in layout.glyphs() {
        let resource = layout
            .font(glyph.font_id())
            .ok_or_else(|| std::io::Error::other("glyph font was not retained"))?;
        assert_eq!(resource.id(), arabic);
        assert_eq!(resource.bytes(), rwml_fonts::noto_sans_arabic_subset());
        assert_eq!(resource.face_index(), 0);
    }
    Ok(())
}

#[test]
fn paragraph_breaks_tabs_and_empty_input_are_defined() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    let text = "A\r\nB\u{2028}C\tD\n";
    let layout = jlreq::layout(text, &fonts, LayoutOptions::try_new(160.0, 16.0)?)?;
    assert!(layout.lines().len() >= 4);
    assert!(layout.glyphs().all(|glyph| {
        let piece = &text[glyph.source_range()];
        piece != "\r" && piece != "\n" && piece != "\u{2028}"
    }));

    let empty = jlreq::layout(
        "",
        &fonts,
        LayoutOptions::try_new(160.0, 16.0)?.with_writing_mode(WritingMode::VerticalRl),
    )?;
    assert!(empty.lines().is_empty());
    assert_eq!(empty.glyphs().count(), 0);
    assert_eq!(empty.writing_mode(), WritingMode::VerticalRl);
    assert_eq!(empty.options().writing_mode(), WritingMode::VerticalRl);
    Ok(())
}

#[test]
fn ttc_variations_and_features_reach_the_shaper() -> Result<(), Box<dyn Error>> {
    let mut collection = FontLibrary::new();
    collection.register_face(
        bytes(font_test_data::ttc::TTC),
        0,
        "Fixture TTC",
        FontStyle::default(),
    )?;
    assert!(
        collection
            .register_face(
                bytes(font_test_data::ttc::TTC),
                u32::MAX,
                "bad face",
                FontStyle::default(),
            )
            .is_err()
    );

    let mut variable = FontLibrary::new();
    variable.register_face(
        bytes(font_test_data::VAZIRMATN_VAR),
        0,
        "Vazirmatn",
        FontStyle::default(),
    )?;
    let weight = FontVariation::try_new(OpenTypeTag::try_new("wght")?, 700.0)?;
    let liga = OpenTypeFeature::new(OpenTypeTag::try_new("liga")?, 1);
    let options = LayoutOptions::try_new(320.0, 18.0)?
        .with_variation(weight)
        .with_feature(liga);
    let layout = jlreq::layout("A", &variable, options)?;
    assert!(layout.glyphs().all(|glyph| glyph.glyph_id() != 0));
    Ok(())
}

#[test]
fn resolved_size_variations_and_draw_cells_match_shaping_runs() -> Result<(), Box<dyn Error>> {
    let mut fonts = FontLibrary::new();
    let id = fonts.register_face(
        bytes(font_test_data::VAZIRMATN_VAR),
        0,
        "Vazirmatn",
        FontStyle::default(),
    )?;
    let global = FontVariation::try_new(OpenTypeTag::try_new("wght")?, 450.0)?;
    let span_value = FontVariation::try_new(OpenTypeTag::try_new("wght")?, 725.0)?;
    let mut builder = DocumentBuilder::new("abc");
    builder.span(
        0..3,
        SpanStyle::new()
            .with_font_size(20.0)?
            .with_variation(span_value),
    )?;
    let layout = jlreq::layout_document(
        &builder.build()?,
        &fonts,
        LayoutOptions::try_new(240.0, 16.0)?.with_variation(global),
    )?;
    let glyphs = layout.glyphs().collect::<Vec<_>>();
    assert!(glyphs.len() >= 2);
    let shared_axes = glyphs[0].variations().as_ptr();
    for glyph in glyphs {
        assert_eq!(glyph.font_id(), id);
        assert_eq!(glyph.font_size().to_bits(), 20.0_f32.to_bits());
        assert_eq!(glyph.font_size_26_6(), 20 * 64);
        assert_eq!(glyph.variations(), [span_value]);
        assert_eq!(glyph.variations().as_ptr(), shared_axes);
        assert_eq!(
            glyph.draw_origin().x_26_6(),
            glyph
                .origin()
                .x_26_6()
                .saturating_add(glyph.geometry_26_6().4)
        );
        assert_eq!(
            glyph.draw_origin().y_26_6(),
            glyph
                .origin()
                .y_26_6()
                .saturating_add(glyph.geometry_26_6().5)
        );
        let (_, _, width, height) = glyph.cell_bounds().as_26_6();
        assert!(width > 0 && height > 0);
    }
    let resource = layout
        .font(id)
        .ok_or_else(|| std::io::Error::other("variable font was not retained"))?;
    assert!(resource.default_variations().is_empty());
    assert!(resource.synthesis().is_empty());
    assert!(layout.bounds().is_some());
    Ok(())
}

#[test]
fn paragraph_progress_uses_actual_line_cells_and_applies_gap_once() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    let gap = 3 * 64;
    for mode in [WritingMode::HorizontalTb, WritingMode::VerticalRl] {
        let plain = jlreq::layout(
            "A\nB",
            &fonts,
            LayoutOptions::try_new(240.0, 16.0)?
                .with_writing_mode(mode)
                .with_line_gap(3.0)?,
        )?;
        assert_eq!(plain.lines().len(), 2);
        if mode == WritingMode::HorizontalTb {
            assert_eq!(
                plain.lines()[1]
                    .origin()
                    .y_26_6()
                    .saturating_sub(plain.lines()[0].origin().y_26_6()),
                19 * 64
            );
        } else {
            assert_eq!(
                plain.lines()[0]
                    .origin()
                    .x_26_6()
                    .saturating_sub(plain.lines()[1].origin().x_26_6()),
                19 * 64
            );
        }

        let mut builder = DocumentBuilder::new("漢注\n字");
        builder.span(0..6, SpanStyle::new().with_font_size(40.0)?)?;
        builder.group_ruby(0..3, "かん")?;
        builder.warichu(3..6)?;
        let layout = jlreq::layout_document(
            &builder.build()?,
            &fonts,
            LayoutOptions::try_new(240.0, 16.0)?
                .with_writing_mode(mode)
                .with_line_gap(3.0)?,
        )?;
        assert_eq!(layout.lines().len(), 2);
        let first = layout.lines()[0].bounds().as_26_6();
        let second = layout.lines()[1].bounds().as_26_6();
        if mode == WritingMode::HorizontalTb {
            assert!(second.1 >= first.1.saturating_add(first.3).saturating_add(gap));
        } else {
            assert!(second.0.saturating_add(second.2) <= first.0.saturating_sub(gap));
        }
    }
    Ok(())
}

#[test]
fn all_typed_constructs_and_ruby_modes_are_automatically_shaped() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    let text = "漢12強注割振字合上式";
    let mut builder = DocumentBuilder::new(text);
    builder.group_ruby(range_of(text, "漢")?, "かん")?;
    builder.tate_chu_yoko(range_of(text, "12")?)?;
    builder.emphasis_dots(range_of(text, "強")?, '・')?;
    builder.warichu(range_of(text, "注")?)?;
    let furawake = range_of(text, "割振")?;
    let furawake_split = furawake.start.saturating_add("割".len());
    builder.furawake(furawake, 2, 0.0)?;
    builder.mandatory_break(furawake_split)?;
    builder.jidori(range_of(text, "字")?, 2)?;
    builder.reference_mark(range_of(text, "合")?, "*")?;
    builder.script(range_of(text, "上")?, "2", ScriptPosition::Superscript)?;
    builder.formula(range_of(text, "式")?)?;
    let document = builder.build()?;
    assert_eq!(document.construct_count(), 9);
    let layout = jlreq::layout_document(
        &document,
        &fonts,
        LayoutOptions::try_new(800.0, 20.0)?.with_writing_mode(WritingMode::VerticalRl),
    )?;
    assert!(layout.glyphs().any(|glyph| glyph.annotation().is_some()));
    assert!(
        layout
            .glyphs()
            .any(|glyph| glyph.transform() == GlyphTransform::TateChuYoko)
    );

    let mut mono = DocumentBuilder::new("漢");
    mono.mono_ruby(0..3, "字")?;
    let mono = mono.build()?;
    let mono_layout = jlreq::layout_document(&mono, &fonts, LayoutOptions::try_new(120.0, 16.0)?)?;
    assert!(
        mono_layout
            .glyphs()
            .any(|glyph| glyph.annotation().is_some())
    );

    let mut jukugo = DocumentBuilder::new("漢");
    jukugo.jukugo_ruby(0..3, "字")?;
    let jukugo = jukugo.build()?;
    let jukugo_layout =
        jlreq::layout_document(&jukugo, &fonts, LayoutOptions::try_new(120.0, 16.0)?)?;
    assert!(
        jukugo_layout
            .glyphs()
            .any(|glyph| glyph.annotation().is_some())
    );
    Ok(())
}

#[test]
fn limits_fail_atomically_and_engine_remains_reusable() -> Result<(), Box<dyn Error>> {
    assert!(FontLibrary::new().register_font([0_u8; 8]).is_err());
    assert!(LayoutOptions::try_new(f32::NAN, 16.0).is_err());
    assert!(LayoutOptions::try_new(100.0, f32::INFINITY).is_err());

    let (fonts, _, _) = fixture_fonts()?;
    let limited = LayoutOptions::try_new(200.0, 16.0)?.with_limits(
        ResourceLimits::default()
            .with_max_glyphs(1)
            .with_max_core_operations(64),
    );
    let mut engine = LayoutEngine::new();
    let failure = engine.layout("ABC", &fonts, limited);
    assert!(matches!(
        failure,
        Err(LayoutError::ResourceLimit {
            resource: Resource::Glyphs,
            ..
        })
    ));

    let recovered = engine.layout("A", &fonts, LayoutOptions::try_new(200.0, 16.0)?)?;
    assert!(!recovered.lines().is_empty());
    let one_shot = jlreq::layout("A", &fonts, LayoutOptions::try_new(200.0, 16.0)?)?;
    assert_eq!(recovered, one_shot);
    Ok(())
}

#[test]
fn hit_caret_and_selection_round_trip_in_both_writing_modes() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    for mode in [WritingMode::HorizontalTb, WritingMode::VerticalRl] {
        let layout = jlreq::layout(
            "A日本語مرحبا",
            &fonts,
            LayoutOptions::try_new(420.0, 18.0)?.with_writing_mode(mode),
        )?;
        let first = layout
            .glyphs()
            .next()
            .ok_or_else(|| std::io::Error::other("fixture produced no glyph"))?;
        let hit = layout.hit_test(first.origin());
        assert!(hit.byte_offset() <= layout.source().len());
        assert!(
            layout
                .caret_rect(hit.byte_offset(), hit.affinity())
                .is_some()
        );
        assert!(!layout.selection_rects(0..layout.source().len()).is_empty());
    }
    Ok(())
}

#[test]
fn explicit_font_bytes_are_bit_identical_across_libraries() -> Result<(), Box<dyn Error>> {
    let mut first_fonts = FontLibrary::new();
    first_fonts.register_font(bytes(font_test_data::NOTO_SANS_JP_CFF))?;
    let mut second_fonts = FontLibrary::new();
    second_fonts.register_font(bytes(font_test_data::NOTO_SANS_JP_CFF))?;
    let first = jlreq::layout(
        "同一フォント",
        &first_fonts,
        LayoutOptions::try_new(300.0, 16.0)?,
    )?;
    let second = jlreq::layout(
        "同一フォント",
        &second_fonts,
        LayoutOptions::try_new(300.0, 16.0)?,
    )?;
    assert_eq!(first, second);
    Ok(())
}

#[test]
fn public_configuration_and_font_metadata_are_observable() -> Result<(), Box<dyn Error>> {
    let mut fonts = FontLibrary::new();
    assert!(fonts.is_empty());
    assert_eq!(fonts.len(), 0);
    assert!(fonts.primary().is_none());
    let style = FontStyle::new(550, 90, FontSlant::Italic);
    assert_eq!(
        (style.weight(), style.width(), style.slant()),
        (550, 90, FontSlant::Italic)
    );
    let primary =
        fonts.register_face(bytes(font_test_data::NOTO_SANS_JP_CFF), 0, "Primary", style)?;
    let secondary = fonts.register_face(
        bytes(font_test_data::TINOS_SUBSET),
        0,
        "Secondary",
        FontStyle::new(400, 100, FontSlant::Oblique),
    )?;
    assert!(!fonts.is_empty());
    assert_eq!(fonts.len(), 2);
    assert_eq!(fonts.primary(), Some(primary));
    fonts.set_primary(secondary)?;
    fonts.set_fallback_order([secondary, secondary, primary])?;
    let resource = fonts
        .get(primary)
        .ok_or_else(|| std::io::Error::other("registered font disappeared"))?;
    assert_eq!(resource.id(), primary);
    assert_eq!(resource.face_index(), 0);
    assert_eq!(resource.family(), "Primary");
    assert_eq!(resource.style(), style);
    assert_eq!(resource.bytes(), font_test_data::NOTO_SANS_JP_CFF);
    assert!(!format!("{resource:?} {fonts:?}").is_empty());

    let mut foreign = FontLibrary::new();
    let in_bounds = foreign.register_font(bytes(font_test_data::TINOS_SUBSET))?;
    foreign.register_font(bytes(font_test_data::TINOS_SUBSET))?;
    let unknown = foreign.register_font(bytes(font_test_data::TINOS_SUBSET))?;
    // Both the out-of-bounds and the in-bounds foreign identifier must fail:
    // provenance, not slot arithmetic, decides ownership.
    assert!(fonts.get(unknown).is_none());
    assert!(fonts.get(in_bounds).is_none());
    for id in [unknown, in_bounds] {
        assert_eq!(
            expected_layout_error(fonts.set_primary(id))?.code(),
            "font.unknown-id"
        );
        assert_eq!(
            expected_layout_error(fonts.set_fallback_order([id]))?.code(),
            "font.unknown-id"
        );
    }
    let layout = jlreq::layout("A", &fonts, LayoutOptions::try_new(160.0, 16.0)?)?;
    assert!(layout.font(in_bounds).is_none());
    let retained = layout.fonts()[0].id();
    assert!(layout.font(retained).is_some());

    #[cfg(feature = "system-fonts")]
    assert_eq!(
        expected_layout_error(fonts.register_system_family(
            "jlreq-family-that-cannot-exist-7f721b83",
            FontStyle::default(),
        ))?
        .code(),
        "font.system-family-not-found"
    );
    Ok(())
}

#[test]
fn corrupted_font_bytes_never_panic_and_never_yield_partial_results() -> Result<(), Box<dyn Error>>
{
    let pristine = font_test_data::NOTO_SANS_JP_CFF;
    let options = LayoutOptions::try_new(120.0, 12.0)?;

    // Every truncation prefix of the fixture font must register-or-refuse
    // cleanly; a sparse subset is laid out end to end.
    for length in 0..=pristine.len() {
        let mut fonts = FontLibrary::new();
        let Ok(_) = fonts.register_font(bytes_of(&pristine[..length])) else {
            continue;
        };
        if length % 137 == 0
            && let Ok(layout) = jlreq::layout("AB", &fonts, options.clone())
        {
            assert!(layout.glyphs().count() < 1_000);
        }
    }

    // Every single-byte corruption of the metadata tables (name, head, hhea,
    // OS/2, post live in the leading section of this fixture) must keep
    // registration and the derived family/metrics readers total.
    for index in 0..pristine.len().min(2_200) {
        let mut corrupted = pristine.to_vec();
        corrupted[index] ^= 0xff;
        let mut fonts = FontLibrary::new();
        if let Ok(id) = fonts.register_font(bytes_of(&corrupted)) {
            let resource = fonts.get(id).ok_or("registered font is readable")?;
            let _ = (resource.family().len(), resource.metrics());
        }
    }
    Ok(())
}

fn bytes_of(data: &[u8]) -> Arc<[u8]> {
    Arc::from(data.to_vec())
}

#[test]
fn every_resource_limit_fails_exactly_below_its_observed_demand() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    let text = "AB CD\nMN PZ";
    let base = LayoutOptions::try_new(160.0, 16.0)?;
    let mut engine = LayoutEngine::new();

    // Directly countable demands: exactly-at succeeds, one-below fails with
    // the documented code, and the engine stays reusable after each failure.
    let font_bytes: usize = fonts.fonts().iter().map(|font| font.bytes().len()).sum();
    let countable = [
        ("limit.input-bytes", text.len(), {
            let make: fn(ResourceLimits, usize) -> ResourceLimits =
                ResourceLimits::with_max_input_bytes;
            make
        }),
        ("limit.fonts", fonts.len(), ResourceLimits::with_max_fonts),
        (
            "limit.font-bytes",
            font_bytes,
            ResourceLimits::with_max_font_bytes,
        ),
        ("limit.paragraphs", 2, ResourceLimits::with_max_paragraphs),
    ];
    for (code, demand, apply) in countable {
        let exact = base
            .clone()
            .with_limits(apply(ResourceLimits::default(), demand));
        engine.layout(text, &fonts, exact)?;
        let starved = base
            .clone()
            .with_limits(apply(ResourceLimits::default(), demand.saturating_sub(1)));
        let error = expected_layout_error(engine.layout(text, &fonts, starved))?;
        assert_eq!(error.code(), code, "one below the {code} demand");
    }

    // Search-derived demands: find the minimal passing budget, then pin the
    // boundary from both sides.
    let searchable = [
        ("limit.runs", {
            let make: fn(ResourceLimits, usize) -> ResourceLimits = ResourceLimits::with_max_runs;
            make
        }),
        ("limit.glyphs", ResourceLimits::with_max_glyphs),
        (
            "limit.core-operations",
            ResourceLimits::with_max_core_operations,
        ),
    ];
    for (code, apply) in searchable {
        let mut low = 0_usize;
        let mut high = 1_000_000_usize;
        let passes = |budget: usize, engine: &mut LayoutEngine| {
            engine
                .layout(
                    text,
                    &fonts,
                    base.clone()
                        .with_limits(apply(ResourceLimits::default(), budget)),
                )
                .is_ok()
        };
        assert!(
            passes(high, &mut engine),
            "the ceiling must pass for {code}"
        );
        while low + 1 < high {
            let middle = low + (high - low) / 2;
            if passes(middle, &mut engine) {
                high = middle;
            } else {
                low = middle;
            }
        }
        let error = expected_layout_error(
            engine.layout(
                text,
                &fonts,
                base.clone()
                    .with_limits(apply(ResourceLimits::default(), low)),
            ),
        )?;
        assert_eq!(error.code(), code, "one below the minimal {code} budget");
        engine.layout(
            text,
            &fonts,
            base.clone()
                .with_limits(apply(ResourceLimits::default(), high)),
        )?;
    }

    // The engine is fully reusable after the whole battery.
    engine.layout(text, &fonts, base)?;
    Ok(())
}

#[test]
fn layouts_are_deterministic_and_partition_their_source() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    let text = "AB CD EF\n\nMN PZ";
    let options = LayoutOptions::try_new(60.0, 16.0)?
        .with_first_line_indent(4.0)?
        .with_widow(jlreq::Widow::MinimumClusters(2));

    // Identical inputs are bit-identical, one-shot equals engine reuse, and
    // a layout relaid from its own retained options equals itself.
    let first = jlreq::layout(text, &fonts, options.clone())?;
    let second = jlreq::layout(text, &fonts, options.clone())?;
    assert_eq!(first, second);
    let mut engine = LayoutEngine::new();
    let reused_a = engine.layout(text, &fonts, options.clone())?;
    let reused_b = engine.layout(text, &fonts, options)?;
    assert_eq!(first, reused_a);
    assert_eq!(reused_a, reused_b);
    assert_eq!(jlreq::layout(text, &fonts, first.options().clone())?, first);

    // Lines partition the non-separator source in order, and every base
    // glyph stays inside its line's range.
    let mut cursor = 0;
    for line in first.lines() {
        let range = line.range();
        assert!(range.start >= cursor, "line ranges are ordered");
        assert!(
            text[cursor..range.start]
                .chars()
                .all(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}')),
            "only paragraph separators fall between lines"
        );
        for glyph in line.glyphs() {
            if glyph.annotation().is_none() {
                let source = glyph.source_range();
                assert!(source.start >= range.start && source.end <= range.end);
            }
        }
        cursor = range.end;
    }
    assert!(
        text[cursor..]
            .chars()
            .all(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    );
    Ok(())
}

#[test]
fn lines_carry_indices_paragraph_membership_and_offset_lookup() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    // First paragraph wraps into two lines, then a blank paragraph, then one more.
    let text = "AB CD EF\n\nMN";
    let layout = jlreq::layout(text, &fonts, LayoutOptions::try_new(60.0, 16.0)?)?;
    let lines = layout.lines();
    assert_eq!(lines.len(), 4);
    for (expected_index, line) in lines.iter().enumerate() {
        assert_eq!(line.index(), expected_index);
    }
    assert_eq!(lines[0].paragraph_index(), 0);
    assert_eq!(lines[1].paragraph_index(), 0);
    assert_eq!(lines[2].paragraph_index(), 1);
    assert_eq!(lines[3].paragraph_index(), 2);
    assert!(lines[0].is_first_in_paragraph());
    assert!(!lines[0].is_last_in_paragraph());
    assert!(!lines[1].is_first_in_paragraph());
    assert!(lines[1].is_last_in_paragraph());
    assert!(lines[2].is_first_in_paragraph() && lines[2].is_last_in_paragraph());
    assert!(lines[3].is_first_in_paragraph() && lines[3].is_last_in_paragraph());

    // Every caret position is addressable, including the ones the editing
    // primitives themselves produce.
    assert_eq!(layout.line_index_at(0), Some(0));
    assert_eq!(layout.line_index_at(lines[1].range().start), Some(1));
    assert_eq!(layout.line_index_at(7), Some(1));
    // A wrap boundary belongs to the following line, which starts there.
    assert_eq!(lines[0].range().end, lines[1].range().start);
    assert_eq!(layout.line_index_at(lines[0].range().end), Some(1));
    // A line ending before a paragraph separator keeps its own end.
    assert_eq!(layout.line_index_at(lines[1].range().end), Some(1));
    // The blank paragraph's empty line holds its own start offset.
    assert_eq!(lines[2].range().len(), 0);
    assert_eq!(layout.line_index_at(lines[2].range().start), Some(2));
    assert_eq!(layout.line_index_at(10), Some(3));
    // The end of the text belongs to the final line; past it, nothing does.
    assert_eq!(layout.line_index_at(text.len()), Some(3));
    assert_eq!(layout.line_index_at(text.len().saturating_add(1)), None);

    // Every offset a caret walk visits resolves to a line.
    let mut offset = 0;
    let mut affinity = Affinity::Upstream;
    while let Some(next) = layout.next_visual_caret(offset, affinity) {
        offset = next.byte_offset();
        affinity = next.affinity();
        assert!(
            layout.line_index_at(offset).is_some(),
            "caret offset {offset} must belong to a line"
        );
    }
    Ok(())
}

#[test]
fn glyphs_report_their_construct_and_words_and_graphemes_segment() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    let text = "AB CD";
    let mut builder = DocumentBuilder::new(text);
    builder.group_ruby(0..2, "NN")?;
    let document = builder.build()?;
    let layout = jlreq::layout_document(&document, &fonts, LayoutOptions::try_new(240.0, 16.0)?)?;

    // Base glyphs inside the ruby range and its annotation glyphs report the
    // construct ordinal; everything else reports none.
    for glyph in layout.glyphs() {
        let expected = if glyph.source_range().start < 2 {
            Some(0)
        } else {
            None
        };
        assert_eq!(glyph.construct(), expected);
    }
    assert!(
        layout
            .glyphs()
            .any(|glyph| glyph.annotation().is_some() && glyph.construct() == Some(0))
    );
    let ruby_range = document.construct(0).ok_or("construct read-back")?.range();
    assert_eq!(ruby_range, 0..2);

    // Grapheme boundaries respect multi-scalar clusters.
    let family = "a👨\u{200d}👧b";
    let family_layout = jlreq::layout(family, &fonts, LayoutOptions::try_new(240.0, 16.0)?)?;
    assert_eq!(family_layout.next_grapheme_boundary(0), Some(1));
    assert_eq!(
        family_layout.next_grapheme_boundary(1),
        Some(family.len() - 1)
    );
    assert_eq!(
        family_layout.next_grapheme_boundary(family.len() - 1),
        Some(family.len())
    );
    assert_eq!(family_layout.next_grapheme_boundary(family.len()), None);
    assert_eq!(
        family_layout.prev_grapheme_boundary(family.len()),
        Some(family.len() - 1)
    );
    assert_eq!(
        family_layout.prev_grapheme_boundary(family.len() - 1),
        Some(1)
    );
    assert_eq!(family_layout.prev_grapheme_boundary(1), Some(0));
    assert_eq!(family_layout.prev_grapheme_boundary(0), None);

    // Word and sentence segments enclose their offsets on char boundaries.
    assert_eq!(layout.word_range_at(0), Some(0..2));
    assert_eq!(layout.word_range_at(1), Some(0..2));
    assert_eq!(layout.word_range_at(2), Some(2..3));
    assert_eq!(layout.word_range_at(3), Some(3..5));
    assert_eq!(layout.word_range_at(text.len()), None);
    let japanese = "これは日本語です。次の文。";
    let japanese_layout = jlreq::layout(japanese, &fonts, LayoutOptions::try_new(640.0, 16.0)?)?;
    let kanji_offset = japanese.find("日本語").ok_or("substring")?;
    let word = japanese_layout
        .word_range_at(kanji_offset)
        .ok_or("word segment")?;
    assert!(word.start <= kanji_offset && word.end > kanji_offset);
    assert!(japanese.is_char_boundary(word.start) && japanese.is_char_boundary(word.end));
    let first_sentence = japanese_layout.sentence_range_at(0).ok_or("sentence")?;
    let terminator_end = japanese.find('。').ok_or("terminator")? + "。".len();
    assert_eq!(first_sentence, 0..terminator_end);
    let second_sentence = japanese_layout
        .sentence_range_at(terminator_end)
        .ok_or("sentence")?;
    assert_eq!(second_sentence.start, terminator_end);
    Ok(())
}

#[test]
fn visual_caret_motion_walks_lines_and_crosses_them() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    let text = "AB CD EF";
    let layout = jlreq::layout(text, &fonts, LayoutOptions::try_new(60.0, 16.0)?)?;
    assert!(layout.lines().len() > 1);
    let caret_x = |offset: usize, affinity: Affinity| {
        layout
            .caret_rect(offset, affinity)
            .map(|rect| rect.as_26_6().0)
    };

    // Walking forward from the line start strictly increases the caret's
    // inline position until the line is exhausted, then continues on the
    // following line.
    let mut offset = 0;
    let mut affinity = Affinity::Upstream;
    let mut previous_x = caret_x(offset, affinity).ok_or("start caret")?;
    let mut hops = 0;
    let mut crossed_line = false;
    while let Some(next) = layout.next_visual_caret(offset, affinity) {
        hops += 1;
        assert!(hops < 32, "visual walk must terminate");
        let next_x = caret_x(next.byte_offset(), next.affinity()).ok_or("caret")?;
        if layout.line_index_at(next.byte_offset()) == layout.line_index_at(offset) && !crossed_line
        {
            assert!(next_x > previous_x, "same-line motion moves inline-forward");
        } else {
            crossed_line = true;
        }
        offset = next.byte_offset();
        affinity = next.affinity();
        previous_x = next_x;
    }
    assert!(crossed_line, "the walk reaches the second line");
    assert!(hops >= 4);

    // Backward motion from the walk's end returns to the layout start.
    let mut back_offset = offset;
    let mut back_affinity = affinity;
    let mut back_hops = 0;
    while let Some(previous) = layout.prev_visual_caret(back_offset, back_affinity) {
        back_hops += 1;
        assert!(back_hops < 32, "backward walk must terminate");
        back_offset = previous.byte_offset();
        back_affinity = previous.affinity();
    }
    assert_eq!(back_offset, 0);

    // Line-to-line motion keeps the inline position and is reversible.
    let second_line_start = layout.lines()[1].range().start;
    let up = layout
        .caret_previous_line(second_line_start, Affinity::Upstream)
        .ok_or("previous line caret")?;
    assert_eq!(layout.line_index_at(up.byte_offset()), Some(0));
    let down = layout
        .caret_next_line(up.byte_offset(), up.affinity())
        .ok_or("next line caret")?;
    assert_eq!(layout.line_index_at(down.byte_offset()), Some(1));
    assert_eq!(
        layout.caret_previous_line(0, Affinity::Upstream),
        None,
        "the first line has no previous line"
    );

    // Filled selection rectangles extend the continuing line to its edge.
    let exact = layout.selection_rects(1..text.len());
    let filled = layout.selection_rects_filled(1..text.len());
    assert_eq!(filled.len(), 2);
    let exact_first_width: i32 = exact
        .iter()
        .filter(|rect| rect.as_26_6().1 == filled[0].as_26_6().1)
        .map(|rect| rect.as_26_6().2)
        .sum();
    assert!(filled[0].as_26_6().2 >= exact_first_width);
    let line_trailing = {
        let bounds = layout.lines()[0].bounds().as_26_6();
        bounds.0 + bounds.2
    };
    assert_eq!(
        filled[0].as_26_6().0 + filled[0].as_26_6().2,
        line_trailing,
        "the first selected line fills to its trailing edge"
    );
    assert!(layout.selection_rects_filled(0..0).is_empty());
    Ok(())
}

#[test]
fn discretionary_breaks_roles_and_frames_are_authorable() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;

    // A discretionary candidate flows through to the core as a penalized
    // break opportunity (the lowering itself is pinned by the engine unit
    // tests); the document reads the suggestion back, and layout stays
    // complete with the candidate in play.
    let text = "AB CD EF";
    let narrow = LayoutOptions::try_new(60.0, 16.0)?;
    let natural = jlreq::layout(text, &fonts, narrow.clone())?;
    assert!(natural.lines().len() > 1);
    let mut suggested = DocumentBuilder::new(text);
    suggested.discretionary_break(4)?;
    let document = suggested.build()?;
    assert_eq!(document.discretionary_breaks(), [4]);
    let laid_out = jlreq::layout_document(&document, &fonts, narrow)?;
    assert!(laid_out.lines().len() > 1);
    assert_eq!(laid_out.source(), text);

    // Suggesting and prohibiting the same offset is a contradiction.
    let mut conflicted = DocumentBuilder::new(text);
    conflicted.discretionary_break(3)?;
    conflicted.prohibit_break(3)?;
    assert_eq!(
        expected_layout_error(conflicted.build())?.code(),
        "document.conflicting-break"
    );

    // Asserted roles and frames flow through spans into a complete layout.
    let text = "3.4 A!";
    let mut builder = DocumentBuilder::new(text);
    builder.span(
        1..2,
        SpanStyle::new().with_role(jlreq::TextRole::DecimalPoint),
    )?;
    builder.span(5..6, SpanStyle::new().with_role(jlreq::TextRole::Plain))?;
    builder.span(
        0..1,
        SpanStyle::new().with_frame(jlreq::MetricsFrame::FullEm),
    )?;
    let document = builder.build()?;
    let spans: Vec<_> = document.spans().collect();
    assert_eq!(spans[1].1.role(), jlreq::TextRole::DecimalPoint);
    assert_eq!(spans[0].1.frame(), jlreq::MetricsFrame::FullEm);
    assert_eq!(
        spans[2].1.frame(),
        jlreq::MetricsFrame::Auto,
        "unset frames stay on the heuristic"
    );
    let layout = jlreq::layout_document(&document, &fonts, LayoutOptions::try_new(240.0, 16.0)?)?;
    assert!(layout.glyphs().count() > 0);
    Ok(())
}

#[test]
fn paragraph_styles_override_measure_alignment_indent_widow_and_tabs() -> Result<(), Box<dyn Error>>
{
    let (fonts, _, _) = fixture_fonts()?;
    let base = LayoutOptions::try_new(240.0, 16.0)?;
    let text = "ABC ABC\nABC ABC";
    let first_paragraph = 0..7;
    let second_paragraph = 8..15;

    // A narrower measure wraps only the styled paragraph.
    let mut narrow = DocumentBuilder::new(text);
    narrow.paragraph_style(
        second_paragraph.clone(),
        jlreq::ParagraphStyle::new().with_line_extent(40.0)?,
    )?;
    let narrow = jlreq::layout_document(&narrow.build()?, &fonts, base.clone())?;
    let lines_in = |layout: &jlreq::TextLayout, range: &Range<usize>| {
        layout
            .lines()
            .iter()
            .filter(|line| line.range().start >= range.start && line.range().end <= range.end)
            .count()
    };
    assert_eq!(lines_in(&narrow, &first_paragraph), 1);
    assert!(lines_in(&narrow, &second_paragraph) > 1);

    // A first-line indent shifts exactly the styled paragraph's first glyph.
    let plain = jlreq::layout_document(&DocumentBuilder::new(text).build()?, &fonts, base.clone())?;
    let mut indented = DocumentBuilder::new(text);
    indented.paragraph_style(
        second_paragraph.clone(),
        jlreq::ParagraphStyle::new().with_first_line_indent(16.0)?,
    )?;
    let indented = jlreq::layout_document(&indented.build()?, &fonts, base.clone())?;
    let first_glyph_x = |layout: &jlreq::TextLayout, offset: usize| {
        layout
            .glyphs()
            .find(|glyph| glyph.source_range().start == offset)
            .map(|glyph| glyph.geometry_26_6().0)
    };
    assert_eq!(first_glyph_x(&indented, 0), first_glyph_x(&plain, 0));
    let plain_second = first_glyph_x(&plain, 8).ok_or("second paragraph glyph")?;
    let indented_second = first_glyph_x(&indented, 8).ok_or("second paragraph glyph")?;
    assert_eq!(indented_second, plain_second.saturating_add(16 * 64));

    // The document-wide indent from LayoutOptions applies to every paragraph.
    let global_indent = jlreq::layout_document(
        &DocumentBuilder::new(text).build()?,
        &fonts,
        base.clone().with_first_line_indent(16.0)?,
    )?;
    assert_eq!(
        first_glyph_x(&global_indent, 0),
        first_glyph_x(&plain, 0).map(|x| x.saturating_add(16 * 64))
    );

    // Alignment and policy overrides change only the styled paragraph.
    let mut centered = DocumentBuilder::new(text);
    centered.paragraph_style(
        second_paragraph.clone(),
        jlreq::ParagraphStyle::new()
            .with_alignment(Alignment::Center)
            .with_style(jlreq::Style::book_2020()),
    )?;
    let centered = jlreq::layout_document(&centered.build()?, &fonts, base.clone())?;
    assert_eq!(first_glyph_x(&centered, 0), first_glyph_x(&plain, 0));
    assert_ne!(centered, plain);

    // Widow control is reachable both document-wide and per paragraph.
    let widow_text = "ABCABCA";
    let tight = LayoutOptions::try_new(48.0, 16.0)?;
    let allow = jlreq::layout(widow_text, &fonts, tight.clone())?;
    let kept = jlreq::layout(
        widow_text,
        &fonts,
        tight.clone().with_widow(jlreq::Widow::MinimumClusters(3)),
    )?;
    let last_line_clusters = |layout: &jlreq::TextLayout| {
        layout
            .lines()
            .last()
            .map(|line| line.range().len())
            .unwrap_or_default()
    };
    assert!(last_line_clusters(&kept) >= 3);
    assert!(last_line_clusters(&allow) < 3 || allow != kept);
    let mut styled_widow = DocumentBuilder::new(widow_text);
    styled_widow.paragraph_style(
        0..widow_text.len(),
        jlreq::ParagraphStyle::new().with_widow(jlreq::Widow::MinimumClusters(3)),
    )?;
    let styled_widow = jlreq::layout_document(&styled_widow.build()?, &fonts, tight)?;
    assert_eq!(styled_widow.lines().len(), kept.lines().len());
    assert!(last_line_clusters(&styled_widow) >= 3);

    // Explicit tab stops replace the ladder: a Start stop pins the cluster
    // after the tab to its exact position.
    let tab_text = "A\tB";
    let stop = jlreq::TabStop::try_new(32.0, jlreq::TabAlignment::Start)?;
    let tabbed = jlreq::layout(tab_text, &fonts, base.clone().with_tab_stops([stop]))?;
    let after_tab = tabbed
        .glyphs()
        .find(|glyph| glyph.source_range().start == 2)
        .ok_or("glyph after tab")?;
    assert_eq!(after_tab.geometry_26_6().0, 32 * 64);
    assert_eq!(stop.position().to_bits(), 32.0_f32.to_bits());
    assert_eq!(stop.alignment(), jlreq::TabAlignment::Start);

    // Validation: overlap at build, bad range at build, and a style that
    // cuts a paragraph at layout.
    let mut overlapping = DocumentBuilder::new(text);
    overlapping.paragraph_style(0..7, jlreq::ParagraphStyle::new())?;
    assert_eq!(
        expected_layout_error(
            overlapping
                .paragraph_style(3..15, jlreq::ParagraphStyle::new())
                .map(|_| ())
        )?
        .code(),
        "document.overlapping-paragraph-styles"
    );
    assert_eq!(
        expected_layout_error(
            DocumentBuilder::new(text)
                .paragraph_style(7..7, jlreq::ParagraphStyle::new())
                .map(|_| ())
        )?
        .code(),
        "document.invalid-paragraph-style-range"
    );
    let mut splitting = DocumentBuilder::new(text);
    splitting.paragraph_style(0..10, jlreq::ParagraphStyle::new())?;
    let error = expected_layout_error(jlreq::layout_document(
        &splitting.build()?,
        &fonts,
        base.clone(),
    ))?;
    assert_eq!(error.code(), "document.paragraph-style-splits-paragraph");
    assert_eq!(error.range(), Some(0..10));

    // Read-back mirrors what the builder accepted.
    let style = jlreq::ParagraphStyle::new()
        .with_line_extent(120.0)?
        .with_alignment(Alignment::End)
        .with_style(jlreq::Style::book_2020())
        .with_first_line_indent(8.0)?
        .with_widow(jlreq::Widow::MinimumClusters(2))
        .with_tab_stops([stop]);
    assert_eq!(style.line_extent(), Some(120.0));
    assert_eq!(style.alignment(), Some(Alignment::End));
    assert_eq!(style.style(), Some(&jlreq::Style::book_2020()));
    assert_eq!(style.first_line_indent(), Some(8.0));
    assert_eq!(style.widow(), Some(jlreq::Widow::MinimumClusters(2)));
    assert_eq!(style.tab_stops(), Some([stop].as_slice()));
    let empty = jlreq::ParagraphStyle::new();
    assert_eq!(empty.line_extent(), None);
    assert_eq!(empty.alignment(), None);
    assert!(empty.style().is_none());
    assert_eq!(empty.first_line_indent(), None);
    assert_eq!(empty.widow(), None);
    assert!(empty.tab_stops().is_none());
    let mut documented = DocumentBuilder::new(text);
    documented.paragraph_style(first_paragraph.clone(), style.clone())?;
    let document = documented.build()?;
    let styles: Vec<_> = document.paragraph_styles().collect();
    assert_eq!(styles, [(first_paragraph, &style)]);

    // A blank paragraph obeys the alignment and indent that govern it, so its
    // caret does not jump to the margin between two styled paragraphs.
    let blank = "ああ\n\nいい";
    let plain_blank =
        jlreq::layout_document(&DocumentBuilder::new(blank).build()?, &fonts, base.clone())?;
    assert_eq!(plain_blank.lines()[1].range().len(), 0);
    assert_eq!(plain_blank.lines()[1].origin().x_26_6(), 0);
    for (alignment, expected_26_6) in [
        (Alignment::End, 240 * 64),
        (Alignment::Center, 120 * 64),
        (Alignment::Start, 0),
    ] {
        let mut aligned = DocumentBuilder::new(blank);
        aligned.paragraph_style(
            0..blank.len(),
            jlreq::ParagraphStyle::new().with_alignment(alignment),
        )?;
        let layout = jlreq::layout_document(&aligned.build()?, &fonts, base.clone())?;
        let empty = &layout.lines()[1];
        assert_eq!(empty.range().len(), 0);
        assert_eq!(
            empty.origin().x_26_6(),
            expected_26_6,
            "blank paragraph under {alignment:?}"
        );
        let caret = layout
            .caret_rect(empty.range().start, Affinity::Downstream)
            .ok_or("the blank paragraph holds a caret")?;
        assert_eq!(
            caret.as_26_6().0,
            empty.origin().x_26_6(),
            "the caret follows the empty line's own origin"
        );
    }
    let mut indented_blank = DocumentBuilder::new(blank);
    indented_blank.paragraph_style(
        0..blank.len(),
        jlreq::ParagraphStyle::new().with_first_line_indent(16.0)?,
    )?;
    let layout = jlreq::layout_document(&indented_blank.build()?, &fonts, base.clone())?;
    assert_eq!(layout.lines()[1].origin().x_26_6(), 16 * 64);

    // Options-level read-back for the three new document-wide controls.
    let configured = base
        .with_widow(jlreq::Widow::MinimumClusters(2))
        .with_first_line_indent(4.0)?
        .with_tab_stops([stop]);
    assert_eq!(configured.widow(), jlreq::Widow::MinimumClusters(2));
    assert_eq!(configured.first_line_indent().to_bits(), 4.0_f32.to_bits());
    assert_eq!(configured.tab_stops(), [stop]);
    Ok(())
}

#[test]
fn script_positions_place_annotations_on_opposite_block_sides() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    let build = |position: ScriptPosition, mode: WritingMode| -> Result<_, Box<dyn Error>> {
        let mut builder = DocumentBuilder::new("AB");
        builder.script(0..1, "N", position)?;
        Ok(jlreq::layout_document(
            &builder.build()?,
            &fonts,
            LayoutOptions::try_new(240.0, 16.0)?.with_writing_mode(mode),
        )?)
    };

    let annotation_extremes = |layout: &jlreq::TextLayout, vertical: bool| {
        let mut base: Option<i32> = None;
        let mut annotation: Option<i32> = None;
        for glyph in layout.glyphs() {
            let value = if vertical {
                glyph.geometry_26_6().0
            } else {
                glyph.geometry_26_6().1
            };
            if glyph.annotation().is_some() {
                annotation = Some(annotation.map_or(value, |kept| kept.max(value)));
            } else {
                base = Some(base.map_or(value, |kept| kept.max(value)));
            }
        }
        (base, annotation)
    };

    // Horizontal: the superscript annotation sits above the base glyphs
    // (smaller y), the subscript below (larger y).
    let superscript = build(ScriptPosition::Superscript, WritingMode::HorizontalTb)?;
    let subscript = build(ScriptPosition::Subscript, WritingMode::HorizontalTb)?;
    assert_ne!(superscript, subscript);
    let (base_y, above_y) = annotation_extremes(&superscript, false);
    let (_, below_y) = annotation_extremes(&subscript, false);
    let base_y = base_y.ok_or("no base glyph")?;
    assert!(above_y.ok_or("no superscript glyph")? < base_y);
    assert!(below_y.ok_or("no subscript glyph")? > base_y);

    // Vertical: superscript right of the base column (larger x), subscript
    // left of it (smaller x).
    let vertical_raised = build(ScriptPosition::Superscript, WritingMode::VerticalRl)?;
    let vertical_lowered = build(ScriptPosition::Subscript, WritingMode::VerticalRl)?;
    assert_ne!(vertical_raised, vertical_lowered);
    let (base_x, right_x) = annotation_extremes(&vertical_raised, true);
    let (_, left_x) = annotation_extremes(&vertical_lowered, true);
    let base_x = base_x.ok_or("no base glyph")?;
    assert!(right_x.ok_or("no superscript glyph")? > base_x);
    assert!(left_x.ok_or("no subscript glyph")? < base_x);
    Ok(())
}

#[test]
fn furawake_without_manual_breaks_balances_clusters_across_columns() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    let options = LayoutOptions::try_new(240.0, 16.0)?;

    // The quickstart shape: furawake plus nothing else must lay out.
    let mut quickstart = DocumentBuilder::new("AB MN PZ");
    quickstart.furawake(3..5, 2, 1.0)?;
    let auto = jlreq::layout_document(&quickstart.build()?, &fonts, options.clone())?;
    assert!(auto.glyphs().count() > 0);

    // Synthesis is exactly the balanced manual split: 2 clusters over 2
    // columns splits after the first cluster.
    let mut manual = DocumentBuilder::new("AB MN PZ");
    manual.furawake(3..5, 2, 1.0)?;
    manual.mandatory_break(4)?;
    let manual = jlreq::layout_document(&manual.build()?, &fonts, options.clone())?;
    assert_eq!(auto, manual);

    // Six clusters over three columns: sublines of 2/2/2, splits at 2 and 4.
    let mut auto_six = DocumentBuilder::new("ABCMNP");
    auto_six.furawake(0..6, 3, 0.5)?;
    let auto_six = jlreq::layout_document(&auto_six.build()?, &fonts, options.clone())?;
    let mut manual_six = DocumentBuilder::new("ABCMNP");
    manual_six.furawake(0..6, 3, 0.5)?;
    manual_six.mandatory_break(2)?;
    manual_six.mandatory_break(4)?;
    let manual_six = jlreq::layout_document(&manual_six.build()?, &fonts, options.clone())?;
    assert_eq!(auto_six, manual_six);

    // Five clusters over two columns: the remainder goes to the earlier
    // subline, so the split falls after three clusters, not two.
    let mut auto_five = DocumentBuilder::new("ABCMN");
    auto_five.furawake(0..5, 2, 0.0)?;
    let auto_five = jlreq::layout_document(&auto_five.build()?, &fonts, options.clone())?;
    let mut manual_five = DocumentBuilder::new("ABCMN");
    manual_five.furawake(0..5, 2, 0.0)?;
    manual_five.mandatory_break(3)?;
    let manual_five = jlreq::layout_document(&manual_five.build()?, &fonts, options.clone())?;
    assert_eq!(auto_five, manual_five);
    let mut unbalanced = DocumentBuilder::new("ABCMN");
    unbalanced.furawake(0..5, 2, 0.0)?;
    unbalanced.mandatory_break(2)?;
    let unbalanced = jlreq::layout_document(&unbalanced.build()?, &fonts, options.clone())?;
    assert_ne!(auto_five, unbalanced);

    // Fewer clusters than columns cannot balance; the core reports the
    // explicit-count contract with the construct's range.
    let mut tiny = DocumentBuilder::new("AB");
    tiny.furawake(0..1, 2, 0.0)?;
    let error = expected_layout_error(jlreq::layout_document(
        &tiny.build()?,
        &fonts,
        options.clone(),
    ))?;
    assert_eq!(error.code(), "input.furawake-split-count");

    // A caller-supplied split disables synthesis entirely: one split for
    // three columns stays an explicit-count error instead of being topped up.
    let mut partial = DocumentBuilder::new("ABCMNP");
    partial.furawake(0..6, 3, 0.0)?;
    partial.mandatory_break(2)?;
    let error = expected_layout_error(jlreq::layout_document(&partial.build()?, &fonts, options))?;
    assert_eq!(error.code(), "input.furawake-split-count");
    Ok(())
}

#[test]
fn derived_families_match_spans_and_unknown_families_are_diagnosed() -> Result<(), Box<dyn Error>> {
    // register_font derives "Noto Sans CJK JP" from the font's own name
    // table, so a span can request it without a manual register_face call.
    let mut fonts = FontLibrary::new();
    let noto = fonts.register_font(bytes(font_test_data::NOTO_SANS_JP_CFF))?;
    let tinos = fonts.register_face(
        bytes(font_test_data::TINOS_SUBSET),
        0,
        "Tinos",
        FontStyle::default(),
    )?;
    fonts.set_primary(tinos)?;
    assert_eq!(
        fonts.get(noto).map(jlreq::FontResource::family),
        Some("Noto Sans CJK JP")
    );

    let text = "ABC";
    let mut builder = DocumentBuilder::new(text);
    builder.span(0..3, SpanStyle::new().with_family("noto sans cjk jp"))?;
    let matched = jlreq::layout_document(
        &builder.build()?,
        &fonts,
        LayoutOptions::try_new(240.0, 16.0)?,
    )?;
    assert!(matched.glyphs().count() > 0);
    assert!(matched.glyphs().all(|glyph| glyph.font_id() == noto));
    assert!(
        matched
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code() != "font.unknown-family")
    );

    // A family nothing declares falls back silently in output but reports
    // one warning per family per call, carrying the first requesting range.
    let mut builder = DocumentBuilder::new(text);
    builder.span(0..1, SpanStyle::new().with_family("Absent Family"))?;
    builder.span(1..2, SpanStyle::new().with_family("Absent Family"))?;
    builder.span(
        2..3,
        SpanStyle::new()
            .with_family("Second Ghost")
            .with_family("Tinos"),
    )?;
    let diagnosed = jlreq::layout_document(
        &builder.build()?,
        &fonts,
        LayoutOptions::try_new(240.0, 16.0)?,
    )?;
    assert!(diagnosed.glyphs().count() > 0);
    let unknown_family: Vec<_> = diagnosed
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code() == "font.unknown-family")
        .collect();
    assert_eq!(unknown_family.len(), 2);
    assert_eq!(unknown_family[0].range(), Some(0..1));
    assert_eq!(unknown_family[0].severity(), DiagnosticSeverity::Warning);
    assert_eq!(unknown_family[1].range(), Some(2..3));

    // Font metrics reach the renderer through the retained resource.
    let resource = matched
        .font(noto)
        .ok_or_else(|| std::io::Error::other("the matched face was not retained"))?;
    let metrics = resource
        .metrics()
        .ok_or_else(|| std::io::Error::other("the fixture font carries metrics tables"))?;
    assert!(metrics.ascent() > 0.0);
    assert!(metrics.descent() < 0.0);
    assert!(metrics.underline_thickness().is_some());
    Ok(())
}

#[test]
fn all_option_values_validate_quantize_and_reach_layout() -> Result<(), Box<dyn Error>> {
    let tag = OpenTypeTag::try_new("kern")?;
    assert_eq!(tag.bytes(), *b"kern");
    assert_eq!(OpenTypeTag::try_new("abc ")?.bytes(), *b"abc ");
    assert!(OpenTypeTag::try_new("bad").is_err());
    assert!(OpenTypeTag::try_new("a bc").is_err());
    assert!(OpenTypeTag::try_new(" abc").is_err());
    assert!(OpenTypeTag::try_new("a\ncd").is_err());
    let feature = OpenTypeFeature::new(tag, 7);
    assert_eq!((feature.tag(), feature.value()), (tag, 7));
    let variation = FontVariation::try_new(OpenTypeTag::try_new("wght")?, 650.125)?;
    assert_eq!(variation.tag().bytes(), *b"wght");
    assert!((variation.value() - 650.125).abs() < f32::EPSILON);
    assert!(FontVariation::try_new(tag, f32::NAN).is_err());

    let limits = ResourceLimits::default()
        .with_max_input_bytes(11)
        .with_max_fonts(12)
        .with_max_font_bytes(13)
        .with_max_paragraphs(14)
        .with_max_runs(15)
        .with_max_glyphs(16)
        .with_max_constructs(17)
        .with_max_core_operations(18);
    assert_eq!(
        (
            limits.max_input_bytes(),
            limits.max_fonts(),
            limits.max_font_bytes(),
            limits.max_paragraphs(),
            limits.max_runs(),
            limits.max_glyphs(),
            limits.max_constructs(),
            limits.max_core_operations(),
        ),
        (11, 12, 13, 14, 15, 16, 17, 18)
    );

    let options = LayoutOptions::try_new(100.125, 16.25)?
        .with_writing_mode(WritingMode::HorizontalTb)
        .with_alignment(Alignment::Center)
        .with_style(jlreq::Style::book_2020())
        .with_language("ja-JP")?
        .with_base_direction(BaseDirection::LeftToRight)
        .with_line_gap(2.5)?
        .with_tab_width(8)?
        .with_feature(feature)
        .with_variation(variation)
        .with_limits(limits);
    assert!((options.line_extent() - 100.125).abs() < f32::EPSILON);
    assert!((options.font_size() - 16.25).abs() < f32::EPSILON);
    assert_eq!(options.writing_mode(), WritingMode::HorizontalTb);
    assert_eq!(options.alignment(), Alignment::Center);
    assert_eq!(options.style(), &jlreq::Style::book_2020());
    assert_eq!(options.language(), "ja-JP");
    assert_eq!(options.base_direction(), BaseDirection::LeftToRight);
    assert!((options.line_gap() - 2.5).abs() < f32::EPSILON);
    assert_eq!(options.tab_width(), 8);
    assert_eq!(options.features(), [feature]);
    assert_eq!(options.variations(), [variation]);
    assert_eq!(options.limits(), limits);

    let replaced = options
        .clone()
        .with_features([feature, feature])
        .with_variations([variation]);
    assert_eq!(replaced.features(), [feature, feature]);
    assert_eq!(replaced.variations(), [variation]);
    let cleared = replaced.with_features([]).with_variations([]);
    assert!(cleared.features().is_empty());
    assert!(cleared.variations().is_empty());
    let options = options.with_limits(ResourceLimits::default());
    assert_eq!(options.limits(), ResourceLimits::default());
    assert!(
        LayoutOptions::try_new(100.0, 16.0)?
            .with_language("")
            .is_err()
    );
    assert!(
        LayoutOptions::try_new(100.0, 16.0)?
            .with_language("ja_JP")
            .is_err()
    );
    assert!(
        LayoutOptions::try_new(100.0, 16.0)?
            .with_line_gap(-1.0)
            .is_err()
    );
    assert!(
        LayoutOptions::try_new(100.0, 16.0)?
            .with_tab_width(0)
            .is_err()
    );

    let (fonts, _, _) = fixture_fonts()?;
    for alignment in [
        Alignment::Start,
        Alignment::Center,
        Alignment::End,
        Alignment::Justify,
    ] {
        let layout = jlreq::layout("ABC", &fonts, options.clone().with_alignment(alignment))?;
        assert!(!layout.lines().is_empty());
    }
    for direction in [BaseDirection::LeftToRight, BaseDirection::RightToLeft] {
        let layout = jlreq::layout(
            "A مرحبا",
            &fonts,
            options.clone().with_base_direction(direction),
        )?;
        assert!(!layout.lines().is_empty());
    }
    Ok(())
}

#[test]
fn span_roles_and_document_validation_are_typed() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    let text = "ABCDEF";
    let feature = OpenTypeFeature::new(OpenTypeTag::try_new("liga")?, 0);
    let variation = FontVariation::try_new(OpenTypeTag::try_new("wght")?, 500.0)?;
    let roles = [
        TextRole::Text,
        TextRole::GroupedNumeral,
        TextRole::UnitSymbol,
        TextRole::QuantitySymbol,
        TextRole::Formula,
        TextRole::WarichuBracket,
    ];
    let mut builder = DocumentBuilder::new(text);
    for (index, role) in roles.into_iter().enumerate() {
        let mut style = SpanStyle::new().with_role(role);
        if index == 0 {
            style = style
                .with_family("Noto Sans JP")
                .with_font_style(FontStyle::new(500, 100, FontSlant::Normal))
                .with_font_size(17.0)?
                .with_language("ja")?
                .with_feature(feature)
                .with_variation(variation);
        }
        builder.span(index..index.saturating_add(1), style)?;
    }
    builder.mandatory_break(2)?;
    builder.prohibit_break(3)?;
    let document = builder.build()?;
    assert_eq!(document.text(), text);
    let layout = jlreq::layout_document(&document, &fonts, LayoutOptions::try_new(200.0, 16.0)?)?;
    assert!(!layout.lines().is_empty());
    assert!(SpanStyle::new().with_font_size(0.0).is_err());
    assert!(SpanStyle::new().with_language("ja_JP").is_err());

    let mut invalid = DocumentBuilder::new("éA");
    assert_eq!(
        expected_layout_error(invalid.span(1..2, SpanStyle::default()))?.code(),
        "document.invalid-span-range"
    );
    invalid.span(0..2, SpanStyle::default())?;
    assert_eq!(
        expected_layout_error(invalid.span(0..3, SpanStyle::default()))?.code(),
        "document.overlapping-spans"
    );
    assert_eq!(
        expected_layout_error(invalid.mandatory_break(0))?.code(),
        "document.invalid-break"
    );
    assert_eq!(
        expected_layout_error(invalid.prohibit_break(1))?.code(),
        "document.invalid-break"
    );

    let mut conflicting = DocumentBuilder::new("AB");
    conflicting.mandatory_break(1)?;
    conflicting.prohibit_break(1)?;
    assert_eq!(
        expected_layout_error(conflicting.build())?.code(),
        "document.conflicting-break"
    );
    Ok(())
}

#[test]
fn ruby_and_construct_validation_preserve_precise_codes() -> Result<(), Box<dyn Error>> {
    let run = RubyRun::new(0..3, 0..3);
    assert_eq!(run.base(), 0..3);
    assert_eq!(run.annotation(), 0..3);
    let mut valid = DocumentBuilder::new("漢");
    valid.ruby(RubyKind::Jukugo, 0..3, "字", [run])?;
    assert_eq!(valid.build()?.construct_count(), 1);

    let mut group = DocumentBuilder::new("AB");
    group.ruby(
        RubyKind::Group,
        0..2,
        "xy",
        [RubyRun::new(0..1, 0..1), RubyRun::new(1..2, 1..2)],
    )?;
    assert_eq!(
        expected_layout_error(group.build())?.code(),
        "document.group-ruby-run-count"
    );
    let mut invalid_run = DocumentBuilder::new("AB");
    invalid_run.ruby(RubyKind::Jukugo, 0..2, "xy", [RubyRun::new(1..2, 0..1)])?;
    assert_eq!(
        expected_layout_error(invalid_run.build())?.code(),
        "document.invalid-ruby-run"
    );
    let mut incomplete = DocumentBuilder::new("AB");
    incomplete.ruby(RubyKind::Jukugo, 0..2, "xy", [RubyRun::new(0..1, 0..1)])?;
    assert_eq!(
        expected_layout_error(incomplete.build())?.code(),
        "document.incomplete-ruby-runs"
    );

    let mut invalid = DocumentBuilder::new("AB");
    assert_eq!(
        expected_layout_error(invalid.formula(0..0))?.code(),
        "document.invalid-construct-range"
    );
    assert_eq!(
        expected_layout_error(invalid.group_ruby(0..1, ""))?.code(),
        "document.empty-ruby-annotation"
    );
    assert_eq!(
        expected_layout_error(invalid.reference_mark(0..1, ""))?.code(),
        "document.empty-reference-mark"
    );
    assert_eq!(
        expected_layout_error(invalid.script(0..1, "", ScriptPosition::Subscript))?.code(),
        "document.empty-script-annotation"
    );
    assert_eq!(
        expected_layout_error(invalid.furawake(0..1, 1, 0.0))?.code(),
        "document.invalid-furawake-columns"
    );
    assert_eq!(
        expected_layout_error(invalid.jidori(0..1, 0))?.code(),
        "document.invalid-jidori-cells"
    );
    Ok(())
}

#[test]
fn cross_paragraph_grapheme_and_mono_ruby_fail_atomically() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    let mut crossing = DocumentBuilder::new("A\nB");
    crossing.group_ruby(0..3, "all")?;
    let crossing = crossing.build()?;
    assert_eq!(
        expected_layout_error(jlreq::layout_document(
            &crossing,
            &fonts,
            LayoutOptions::try_new(200.0, 16.0)?,
        ))?
        .code(),
        "document.construct-crosses-paragraph"
    );

    let mut nested_crossing = DocumentBuilder::new("A\r\nB");
    nested_crossing.group_ruby(1..4, "outer")?;
    nested_crossing.tate_chu_yoko(2..3)?;
    let nested_crossing = nested_crossing.build()?;
    let nested_error = expected_layout_error(jlreq::layout_document(
        &nested_crossing,
        &fonts,
        LayoutOptions::try_new(200.0, 16.0)?,
    ))?;
    assert_eq!(nested_error.code(), "document.construct-crosses-paragraph");
    assert_eq!(nested_error.range(), Some(1..4));

    let mut split = DocumentBuilder::new("e\u{301}");
    split.span(0..1, SpanStyle::default())?;
    let split = split.build()?;
    assert_eq!(
        expected_layout_error(jlreq::layout_document(
            &split,
            &fonts,
            LayoutOptions::try_new(200.0, 16.0)?,
        ))?
        .code(),
        "document.span-splits-grapheme"
    );

    let mut mono = DocumentBuilder::new("漢字");
    mono.mono_ruby(0..6, "か")?;
    let mono = mono.build()?;
    assert_eq!(
        expected_layout_error(jlreq::layout_document(
            &mono,
            &fonts,
            LayoutOptions::try_new(200.0, 16.0)?,
        ))?
        .code(),
        "document.mono-ruby-cluster-count"
    );
    Ok(())
}

#[test]
fn every_high_level_resource_limit_has_a_stable_error() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    let cases = [
        (
            ResourceLimits::default().with_max_input_bytes(0),
            Resource::InputBytes,
            "limit.input-bytes",
        ),
        (
            ResourceLimits::default().with_max_fonts(0),
            Resource::Fonts,
            "limit.fonts",
        ),
        (
            ResourceLimits::default().with_max_font_bytes(0),
            Resource::FontBytes,
            "limit.font-bytes",
        ),
        (
            ResourceLimits::default().with_max_paragraphs(0),
            Resource::Paragraphs,
            "limit.paragraphs",
        ),
        (
            ResourceLimits::default().with_max_runs(0),
            Resource::Runs,
            "limit.runs",
        ),
        (
            ResourceLimits::default().with_max_glyphs(0),
            Resource::Glyphs,
            "limit.glyphs",
        ),
        (
            ResourceLimits::default().with_max_core_operations(0),
            Resource::CoreOperations,
            "limit.core-operations",
        ),
    ];
    for (limits, expected_resource, expected_code) in cases {
        let error = expected_layout_error(jlreq::layout(
            "A",
            &fonts,
            LayoutOptions::try_new(100.0, 16.0)?.with_limits(limits),
        ))?;
        assert!(matches!(
            error,
            LayoutError::ResourceLimit { resource, .. } if resource == expected_resource
        ));
        assert_eq!(error.code(), expected_code);
        assert!(!error.to_string().is_empty());
    }

    let mut builder = DocumentBuilder::new("A");
    builder.formula(0..1)?;
    let document = builder.build()?;
    let error = expected_layout_error(jlreq::layout_document(
        &document,
        &fonts,
        LayoutOptions::try_new(100.0, 16.0)?
            .with_limits(ResourceLimits::default().with_max_constructs(0)),
    ))?;
    assert!(matches!(
        error,
        LayoutError::ResourceLimit {
            resource: Resource::Constructs,
            ..
        }
    ));
    assert_eq!(error.code(), "limit.constructs");
    Ok(())
}

#[test]
fn error_codes_ranges_displays_and_sources_are_stable() -> Result<(), Box<dyn Error>> {
    let invalid_font = expected_layout_error(FontLibrary::new().register_font([0_u8; 8]))?;
    let no_fonts = expected_layout_error(jlreq::layout(
        "A",
        &FontLibrary::new(),
        LayoutOptions::try_new(100.0, 16.0)?,
    ))?;
    let invalid_option = expected_layout_error(LayoutOptions::try_new(-1.0, 16.0))?;
    let mut invalid_document = DocumentBuilder::new("A");
    let invalid_document = expected_layout_error(invalid_document.group_ruby(0..1, ""))?;
    let core_input =
        expected_layout_error(jlreq::core::Size::square(0).map_err(LayoutError::from))?;

    let shaped = jlreq::core::ShapedText::new(
        "A",
        jlreq::core::Size::square(1_000)?,
        jlreq::core::Frame::FullEm,
        [jlreq::core::Cluster::new(0..1, 1_000)],
    )?;
    let paragraph = jlreq::core::Paragraph::builder(shaped, 1_000).build()?;
    let mut composer = jlreq::core::Composer::with_limits(
        jlreq::core::CompositionLimits::default().with_max_clusters(0),
    );
    let core_composition = LayoutError::from(
        composer
            .compose(&paragraph, &jlreq::core::Style::default())
            .err()
            .ok_or_else(|| std::io::Error::other("core limit unexpectedly succeeded"))?,
    );

    let errors = [
        invalid_font,
        no_fonts,
        invalid_option,
        invalid_document,
        core_input,
        core_composition,
    ];
    for error in errors {
        assert!(!error.code().is_empty());
        let _ = error.range();
        assert!(!error.to_string().is_empty());
        let _ = error.source();
    }
    Ok(())
}

#[test]
fn renderer_geometry_and_interaction_accessors_are_complete() -> Result<(), Box<dyn Error>> {
    let (fonts, _, _) = fixture_fonts()?;
    let point = Point::try_new(1.25, -2.5)?;
    assert_eq!((point.x(), point.y()), (1.25, -2.5));
    assert_eq!(point.x_26_6(), 80);
    assert_eq!(point.y_26_6(), -160);
    assert!(Point::try_new(9_000_000.0, 0.0).is_err());
    assert!(Point::try_new(0.0, f32::INFINITY).is_err());

    let mut builder = DocumentBuilder::new("漢 \u{10ffff}");
    builder.group_ruby(0..3, "かん")?;
    let document = builder.build()?;
    let layout = jlreq::layout_document(&document, &fonts, LayoutOptions::try_new(160.0, 16.0)?)?;
    for line in layout.lines() {
        let _ = (
            line.range(),
            line.origin(),
            line.inline_extent(),
            line.block_extent(),
            line.bounds(),
            line.writing_mode(),
            line.glyphs().len(),
        );
    }
    for glyph in layout.glyphs() {
        let _ = (
            glyph.glyph_id(),
            glyph.source_range(),
            glyph.x(),
            glyph.y(),
            glyph.advance_x(),
            glyph.advance_y(),
            glyph.offset_x(),
            glyph.offset_y(),
            glyph.draw_origin(),
            glyph.font_size(),
            glyph.font_size_26_6(),
            glyph.variations().len(),
            glyph.cell_bounds(),
            glyph.geometry_26_6(),
        );
        if let Some(annotation) = glyph.annotation() {
            let _ = (annotation.construct(), annotation.range());
        }
    }
    for diagnostic in layout.diagnostics() {
        assert_eq!(diagnostic.severity(), DiagnosticSeverity::Warning);
        let _ = (diagnostic.range(), diagnostic.message(), diagnostic.jlreq());
    }
    let _ = layout.bounds();

    for test_point in [
        Point::try_new(-10_000.0, -10_000.0)?,
        Point::try_new(10_000.0, 10_000.0)?,
        Point::default(),
    ] {
        let hit = layout.hit_test(test_point);
        assert!(hit.byte_offset() <= layout.source().len());
        assert!(matches!(
            hit.affinity(),
            Affinity::Upstream | Affinity::Downstream
        ));
        let _ = hit.is_inside();
    }
    let _ = layout.hit_test_xy(0.0, 0.0)?;
    assert!(layout.hit_test_xy(f32::NAN, 0.0).is_err());
    assert!(
        layout
            .caret_rect(
                layout.source().len().saturating_add(1),
                Affinity::Downstream,
            )
            .is_none()
    );
    assert!(layout.caret_rect(1, Affinity::Downstream).is_none());
    assert!(layout.selection_rects(0..0).is_empty());
    assert!(
        layout
            .selection_rects(0..layout.source().len().saturating_add(1))
            .is_empty()
    );
    for rect in layout.selection_rects(0..layout.source().len()) {
        let _ = (
            rect.x(),
            rect.y(),
            rect.width(),
            rect.height(),
            rect.as_26_6(),
        );
    }

    let blank_line = jlreq::layout("\n", &fonts, LayoutOptions::try_new(100.0, 16.0)?)?;
    assert!(blank_line.caret_rect(0, Affinity::Downstream).is_some());
    let empty = jlreq::layout("", &fonts, LayoutOptions::try_new(100.0, 16.0)?)?;
    let empty_hit = empty.hit_test(Point::default());
    assert_eq!(empty_hit.byte_offset(), 0);
    assert_eq!(empty_hit.affinity(), Affinity::Downstream);
    assert!(!empty_hit.is_inside());
    Ok(())
}
