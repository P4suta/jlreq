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
    foreign.register_font(bytes(font_test_data::TINOS_SUBSET))?;
    foreign.register_font(bytes(font_test_data::TINOS_SUBSET))?;
    let unknown = foreign.register_font(bytes(font_test_data::TINOS_SUBSET))?;
    assert!(fonts.get(unknown).is_none());
    assert_eq!(
        expected_layout_error(fonts.set_primary(unknown))?.code(),
        "font.unknown-id"
    );
    assert_eq!(
        expected_layout_error(fonts.set_fallback_order([unknown]))?.code(),
        "font.unknown-id"
    );

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
