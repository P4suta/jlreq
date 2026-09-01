// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use harfrust::{
    Direction, Feature, Language, ShapeOptions, ShaperData, ShaperInstance, UnicodeBuffer,
    Variation,
};
use icu_segmenter::{GraphemeClusterSegmenter, LineSegmenter, options::LineBreakOptions};
use unicode_bidi::{BidiInfo, Level, ParagraphBidiInfo};

use crate::document::{DocumentConstruct, TextRole};
use crate::{
    Alignment, AnnotationSource, BaseDirection, Diagnostic, DiagnosticSeverity, Document,
    DocumentBuilder, FontId, FontLibrary, FontResource, FontSlant, FontStyle, FontVariation,
    GlyphPlacement, GlyphTransform, LayoutError, LayoutOptions, OpenTypeFeature, Point, Resource,
    SpanStyle, TextLayout, TextLine, WritingMode,
};
// Keep the engine's private data flow in one module while maintaining each phase separately.
include!("engine/layout.rs");
include!("engine/preparation.rs");
include!("engine/shaping.rs");
include!("engine/lowering.rs");
include!("engine/result.rs");
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn fixture_fonts() -> (FontLibrary, FontId, FontId) {
        let mut fonts = FontLibrary::new();
        let first = fonts
            .register_face(
                Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF),
                0,
                "Primary",
                FontStyle::default(),
            )
            .unwrap();
        let second = fonts
            .register_face(
                Arc::<[u8]>::from(font_test_data::TINOS_SUBSET),
                0,
                "Secondary",
                FontStyle::default(),
            )
            .unwrap();
        (fonts, first, second)
    }

    fn raw(font_id: FontId) -> RawGlyph {
        RawGlyph {
            font_id,
            glyph_id: 37,
            cluster: 0,
            x_advance: -256,
            y_advance: -384,
            x_offset: 64,
            y_offset: -128,
        }
    }

    fn cluster(range: Range<usize>, font_id: FontId, level: u8) -> PreparedCluster {
        PreparedCluster {
            range,
            advance: 256,
            size: 192,
            frame: jlreq_core::Frame::Proportional,
            role: None,
            bidi_level: level,
            variations: Arc::from([]),
            glyphs: vec![raw(font_id)],
        }
    }

    fn effective(size: i32) -> EffectiveStyle {
        EffectiveStyle {
            families: vec!["Primary".into()],
            font_style: FontStyle::default(),
            size,
            language: "und".into(),
            features: Vec::new(),
            global_variations: Vec::new(),
            span_variations: Vec::new(),
            role: TextRole::Text,
            frame: crate::MetricsFrame::Auto,
        }
    }

    fn grapheme(font_id: FontId) -> GraphemeItem {
        GraphemeItem {
            range: 0..1,
            level: Level::ltr(),
            script: ScriptClass::Latin,
            direction: Direction::LeftToRight,
            font_id,
            effective: Arc::new(effective(1024)),
            is_tab: false,
        }
    }

    #[test]
    fn engine_debug_and_font_cache_observe_face_and_byte_identity() {
        let (fonts, id, _) = fixture_fonts();
        let resource = fonts.get(id).unwrap().clone();
        let mut engine = LayoutEngine::new();
        assert!(format!("{engine:?}").contains("cached_fonts: 0"));
        engine.ensure_cache(&resource).unwrap();
        assert!(format!("{engine:?}").contains("cached_fonts: 1"));

        let replacement = FontResource {
            bytes: Arc::from(resource.bytes().to_vec()),
            ..resource.clone()
        };
        assert!(!Arc::ptr_eq(&resource.bytes, &replacement.bytes));
        engine.ensure_cache(&replacement).unwrap();
        assert!(Arc::ptr_eq(
            &engine.fonts.get(&id).unwrap().bytes,
            &replacement.bytes
        ));

        let invalid_face = FontResource {
            face_index: u32::MAX,
            ..replacement
        };
        assert!(engine.ensure_cache(&invalid_face).is_err());
    }

    #[test]
    fn preparation_groups_only_identical_non_tab_runs() {
        let (fonts, _, _) = fixture_fonts();
        let options = LayoutOptions::try_new(200.0, 16.0).unwrap();
        let mut engine = LayoutEngine::new();
        let mut call = CallState::new(&options);
        let prepared = engine
            .prepare_text(
                PrepareRequest {
                    source: "AB",
                    global_offset: 0,
                    spans: &[],
                    fonts: &fonts,
                    options: &options,
                    diagnostic_range: None,
                },
                &mut call,
            )
            .unwrap();
        assert_eq!(call.runs, 1);
        assert_eq!(prepared.clusters.len(), 2);

        let mut call = CallState::new(&options);
        let prepared = engine
            .prepare_text(
                PrepareRequest {
                    source: "A\tB",
                    global_offset: 0,
                    spans: &[],
                    fonts: &fonts,
                    options: &options,
                    diagnostic_range: None,
                },
                &mut call,
            )
            .unwrap();
        assert_eq!(call.runs, 2);
        assert_eq!(prepared.clusters.len(), 3);
        assert!(prepared.clusters[1].glyphs.is_empty());
    }

    #[test]
    fn repeated_graphemes_reuse_exact_fallback_shaping_within_each_call() {
        let mut fonts = FontLibrary::new();
        fonts
            .register_face(
                Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF),
                0,
                "Japanese",
                FontStyle::default(),
            )
            .unwrap();
        let emoji = fonts
            .register_face(
                Arc::<[u8]>::from(font_test_data::NOTO_COLOR_EMOJI_FLAGS),
                0,
                "Emoji",
                FontStyle::default(),
            )
            .unwrap();
        let options = LayoutOptions::try_new(300.0, 16.0).unwrap();
        let mut engine = LayoutEngine::new();

        for _ in 0..2 {
            let mut call = CallState::new(&options);
            let prepared = engine
                .prepare_text(
                    PrepareRequest {
                        source: "🇪🇨🇪🇨🇪🇨🇪🇨",
                        global_offset: 0,
                        spans: &[],
                        fonts: &fonts,
                        options: &options,
                        diagnostic_range: None,
                    },
                    &mut call,
                )
                .unwrap();
            assert_eq!(prepared.clusters.len(), 4);
            assert_eq!(call.font_candidates.len(), 1);
            assert_eq!(call.font_selections.len(), 1);
            assert_eq!(call.shape_calls, 3, "two fallbacks plus one shaping run");
            assert!(
                call.shape_calls < 4_usize.saturating_mul(2).saturating_add(1),
                "uncached fallback would shape both faces for every grapheme"
            );
            assert!(
                prepared
                    .clusters
                    .iter()
                    .flat_map(|cluster| &cluster.glyphs)
                    .all(|glyph| glyph.font_id == emoji)
            );
        }
    }

    #[test]
    fn style_resolver_scans_forward_and_shares_one_effective_style_per_span() {
        let options = LayoutOptions::try_new(300.0, 16.0).unwrap();
        let spans = vec![
            (1..3, SpanStyle::new().with_family("First")),
            (4..6, SpanStyle::new().with_family("Second")),
        ];
        let mut resolver = StyleResolver::new(&spans, &options, 0);

        let base = resolver.resolve(&(0..1)).unwrap();
        let first_left = resolver.resolve(&(1..2)).unwrap();
        let first_right = resolver.resolve(&(2..3)).unwrap();
        let between = resolver.resolve(&(3..4)).unwrap();
        let second = resolver.resolve(&(4..5)).unwrap();

        assert!(Arc::ptr_eq(&first_left, &first_right));
        assert!(Arc::ptr_eq(&base, &between));
        assert_eq!(first_left.families, ["First"]);
        assert_eq!(second.families, ["Second"]);
        assert_eq!(resolver.next, 1);
    }

    #[test]
    fn call_charges_are_inclusive_and_accumulate() {
        let mut options = LayoutOptions::try_new(100.0, 16.0).unwrap();
        options.limits.runs = 1;
        options.limits.glyphs = 3;
        let mut call = CallState::new(&options);
        call.charge_run().unwrap();
        assert_eq!(call.runs, 1);
        assert_eq!(call.charge_run().unwrap_err().code(), "limit.runs");
        call.charge_glyphs(2).unwrap();
        assert_eq!(call.glyphs, 2);
        assert_eq!(call.charge_glyphs(2).unwrap_err().code(), "limit.glyphs");
        assert_eq!(call.glyphs, 4);
    }

    #[test]
    fn run_identity_checks_every_shaping_dimension() {
        let (_, first, second) = fixture_fonts();
        let base = grapheme(first);
        assert!(base.same_run(&grapheme(first)));

        let mut changed = grapheme(second);
        assert!(!base.same_run(&changed));
        changed = grapheme(first);
        changed.level = Level::rtl();
        assert!(!base.same_run(&changed));
        changed = grapheme(first);
        changed.level = Level::new(2).expect("valid nested LTR level");
        assert!(!base.same_run(&changed));
        changed = grapheme(first);
        changed.script = ScriptClass::Japanese;
        assert!(!base.same_run(&changed));
        changed = grapheme(first);
        changed.direction = Direction::RightToLeft;
        assert!(!base.same_run(&changed));
        changed = grapheme(first);
        Arc::make_mut(&mut changed.effective).size = 2048;
        assert!(!base.same_run(&changed));
    }

    #[test]
    fn raw_advances_and_prepared_boundaries_are_directional_and_half_open() {
        let (_, id, _) = fixture_fonts();
        let glyph = raw(id);
        assert_eq!(glyph.inline_advance(Direction::LeftToRight), 256);
        assert_eq!(glyph.inline_advance(Direction::RightToLeft), 256);
        assert_eq!(glyph.inline_advance(Direction::TopToBottom), 384);
        assert_eq!(glyph.inline_advance(Direction::BottomToTop), 384);

        let prepared = PreparedText {
            clusters: vec![cluster(2..4, id, 0), cluster(4..6, id, 0)],
        };
        assert!(prepared.is_boundary(0, 8));
        assert!(prepared.is_boundary(8, 8));
        assert!(prepared.is_boundary(2, 8));
        assert!(prepared.is_boundary(4, 8));
        assert!(!prepared.is_boundary(1, 8));
        assert!(!prepared.is_boundary(6, 8));
    }

    #[test]
    fn limits_core_resources_and_severities_map_without_collapsing() {
        assert!(check_limit(Resource::Runs, 2, 2).is_ok());
        let error = check_limit(Resource::Runs, 2, 3).unwrap_err();
        assert_eq!(error.code(), "limit.runs");
        assert_eq!(
            high_level_resource(jlreq_core::CompositionResource::Clusters),
            Some(Resource::Glyphs)
        );
        assert_eq!(
            high_level_resource(jlreq_core::CompositionResource::BreakCandidates),
            Some(Resource::Runs)
        );
        assert_eq!(
            high_level_resource(jlreq_core::CompositionResource::Constructs),
            Some(Resource::Constructs)
        );
        assert_eq!(
            high_level_resource(jlreq_core::CompositionResource::TabStops),
            Some(Resource::Constructs)
        );
        assert_eq!(
            high_level_resource(jlreq_core::CompositionResource::SearchTransitions),
            Some(Resource::CoreOperations)
        );
        assert_eq!(
            diagnostic_severity(jlreq_core::Severity::Info),
            DiagnosticSeverity::Info
        );
        assert_eq!(
            diagnostic_severity(jlreq_core::Severity::Warning),
            DiagnosticSeverity::Warning
        );
        assert_eq!(
            diagnostic_severity(jlreq_core::Severity::Error),
            DiagnosticSeverity::Error
        );
    }

    #[test]
    fn effective_style_rejects_both_sides_of_a_split_grapheme_and_merges_values() {
        let feature = OpenTypeFeature::new(crate::OpenTypeTag::try_new("liga").unwrap(), 0);
        let variation =
            FontVariation::try_new(crate::OpenTypeTag::try_new("wght").unwrap(), 515.0).unwrap();
        let options = LayoutOptions::try_new(100.0, 16.0)
            .unwrap()
            .with_language("en")
            .unwrap()
            .with_feature(feature)
            .with_variation(variation);

        let left = vec![(2..4, SpanStyle::new())];
        assert_eq!(
            effective_style(&(1..3), &left, &options)
                .unwrap_err()
                .code(),
            "document.span-splits-grapheme"
        );
        let right = vec![(0..2, SpanStyle::new())];
        assert_eq!(
            effective_style(&(1..3), &right, &options)
                .unwrap_err()
                .code(),
            "document.span-splits-grapheme"
        );

        let span_feature = OpenTypeFeature::new(crate::OpenTypeTag::try_new("kern").unwrap(), 0);
        let span_variation =
            FontVariation::try_new(crate::OpenTypeTag::try_new("wdth").unwrap(), 90.0).unwrap();
        let span = SpanStyle::new()
            .with_family("Secondary")
            .with_font_style(FontStyle::new(600, 90, crate::FontSlant::Italic))
            .with_font_size(18.0)
            .unwrap()
            .with_language("ja")
            .unwrap()
            .with_feature(span_feature)
            .with_variation(span_variation)
            .with_role(TextRole::Formula);
        let merged = effective_style(&(1..3), &[(0..4, span)], &options).unwrap();
        assert_eq!(merged.families, ["Secondary"]);
        assert_eq!(merged.size, 18 * 64);
        assert_eq!(merged.language, "ja");
        assert_eq!(merged.features, [feature, span_feature]);
        assert_eq!(merged.global_variations, [variation]);
        assert_eq!(merged.span_variations, [span_variation]);
        assert_eq!(merged.role, TextRole::Formula);
    }

    #[test]
    fn variation_layers_merge_by_tag_in_global_system_span_order() {
        let variation = |tag, value| {
            FontVariation::try_new(crate::OpenTypeTag::try_new(tag).unwrap(), value).unwrap()
        };
        let options = LayoutOptions::try_new(100.0, 16.0)
            .unwrap()
            .with_variation(variation("wght", 400.0))
            .with_variation(variation("wdth", 90.0))
            .with_variation(variation("wght", 500.0));
        let span = SpanStyle::new()
            .with_variation(variation("wdth", 80.0))
            .with_variation(variation("wght", 700.0));
        let style = span_effective_style(&base_effective_style(&options), &span);
        let (fonts, first, _) = fixture_fonts();
        let mut resource = fonts.get(first).unwrap().clone();
        resource.default_variations = vec![variation("wght", 600.0), variation("opsz", 12.0)];

        let resolved = resolved_variations(&style, &resource);
        assert_eq!(
            resolved
                .iter()
                .map(|value| (value.tag().bytes(), value.value_26_6()))
                .collect::<Vec<_>>(),
            [
                (*b"opsz", 12 * 64),
                (*b"wdth", 80 * 64),
                (*b"wght", 700 * 64),
            ]
        );
    }

    #[test]
    fn frames_roles_scripts_and_shape_directions_cover_the_closed_tables() {
        assert_eq!(frame_for("日"), jlreq_core::Frame::FullEm);
        assert_eq!(frame_for("😀"), jlreq_core::Frame::FullEm);
        assert_eq!(frame_for("！"), jlreq_core::Frame::FullEm);
        assert_eq!(frame_for("｠"), jlreq_core::Frame::FullEm);
        assert_eq!(frame_for("｡"), jlreq_core::Frame::Proportional);
        assert_eq!(frame_for("A"), jlreq_core::Frame::Proportional);
        assert_eq!(frame_for("日本"), jlreq_core::Frame::Proportional);

        assert_eq!(
            classify_role("A", 0..1, TextRole::Formula),
            Some(jlreq_core::ClusterRole::Formula)
        );
        for text in ["1.2", "1．2", "1・2"] {
            let start = text.chars().next().unwrap().len_utf8();
            let end = start + text[start..].chars().next().unwrap().len_utf8();
            assert_eq!(
                classify_role(text, start..end, TextRole::Text),
                Some(jlreq_core::ClusterRole::DecimalPoint)
            );
        }
        for text in ["1,2", "1，2", "1、2"] {
            let start = text.chars().next().unwrap().len_utf8();
            let end = start + text[start..].chars().next().unwrap().len_utf8();
            assert_eq!(
                classify_role(text, start..end, TextRole::Text),
                Some(jlreq_core::ClusterRole::DigitGroupSeparator)
            );
        }
        assert_eq!(
            classify_role("!", 0..1, TextRole::Text),
            Some(jlreq_core::ClusterRole::SentenceTerminator)
        );
        assert_eq!(
            classify_role("! A", 0..1, TextRole::Text),
            Some(jlreq_core::ClusterRole::SentenceMedial)
        );
        assert_eq!(classify_role(".2", 0..1, TextRole::Text), None);
        assert_eq!(classify_role("1.", 1..2, TextRole::Text), None);
        assert_eq!(classify_role("1A2", 1..2, TextRole::Text), None);
        assert_eq!(classify_role("A,2", 1..2, TextRole::Text), None);
        assert_eq!(classify_role("12", 0..2, TextRole::Text), None);
        assert_eq!(classify_role("", 0..0, TextRole::Text), None);

        assert_eq!(script_class("日"), ScriptClass::Japanese);
        assert_eq!(script_class("😀"), ScriptClass::Emoji);
        assert_eq!(script_class("ع"), ScriptClass::Rtl);
        assert_eq!(script_class("A"), ScriptClass::Latin);
        assert_eq!(script_class("é"), ScriptClass::Latin);
        assert_eq!(script_class("\u{0301}"), ScriptClass::Other);
        assert!(is_japanese('日'));
        assert!(is_japanese('！'));
        assert!(is_japanese('｠'));
        assert!(!is_japanese('｡'));
        assert!(!is_japanese('A'));
        assert!(is_emoji('😀'));
        assert!(!is_emoji('A'));

        assert_eq!(
            shape_direction(WritingMode::VerticalRl, Level::ltr(), ScriptClass::Japanese),
            Direction::TopToBottom
        );
        assert_eq!(
            shape_direction(WritingMode::VerticalRl, Level::ltr(), ScriptClass::Emoji),
            Direction::TopToBottom
        );
        assert_eq!(
            shape_direction(
                WritingMode::HorizontalTb,
                Level::rtl(),
                ScriptClass::Japanese
            ),
            Direction::RightToLeft
        );
        assert_eq!(
            shape_direction(WritingMode::VerticalRl, Level::rtl(), ScriptClass::Latin),
            Direction::RightToLeft
        );
        assert_eq!(
            shape_direction(WritingMode::HorizontalTb, Level::ltr(), ScriptClass::Latin),
            Direction::LeftToRight
        );
    }

    #[test]
    fn paragraph_separators_keep_original_utf8_ranges() {
        let text = "a\r\nb\rc\nd\u{2028}e\u{2029}f";
        let ranges: Vec<_> = paragraph_segments(text)
            .into_iter()
            .map(|segment| segment.content)
            .collect();
        assert_eq!(ranges, [0..1, 3..4, 5..6, 7..8, 11..12, 15..16]);
        assert_eq!(
            paragraph_segments("\r\n")
                .into_iter()
                .map(|segment| segment.content)
                .collect::<Vec<_>>(),
            [0..0, 2..2]
        );
        let offsets = [0, 1, 3, 4, 6];
        assert_eq!(sorted_offsets_in_range(&offsets, &(1..4)), [1, 3]);
        assert!(sorted_offsets_in_range(&offsets, &(2..3)).is_empty());
    }

    #[test]
    fn automatic_and_explicit_break_filters_are_independent() {
        assert!(automatic_break_allowed(3, 6, true, false, false));
        assert!(!automatic_break_allowed(0, 6, true, false, false));
        assert!(!automatic_break_allowed(6, 6, true, false, false));
        assert!(!automatic_break_allowed(3, 6, false, false, false));
        assert!(!automatic_break_allowed(3, 6, true, true, false));
        assert!(!automatic_break_allowed(3, 6, true, false, true));

        let (_, id, _) = fixture_fonts();
        let source = "日本";
        let prepared = PreparedText {
            clusters: vec![cluster(0..3, id, 0), cluster(3..6, id, 0)],
        };
        let plain = DocumentBuilder::new(source).build().unwrap();
        let breaks = collect_breaks(&plain, &(0..6), source, &prepared, &[]);
        assert_eq!(
            breaks.iter().map(|item| item.offset()).collect::<Vec<_>>(),
            [3]
        );
        assert!(!breaks[0].is_mandatory());

        let mut prohibited = DocumentBuilder::new(source);
        prohibited.prohibit_break(3).unwrap();
        assert!(
            collect_breaks(
                &prohibited.build().unwrap(),
                &(0..6),
                source,
                &prepared,
                &[]
            )
            .is_empty()
        );

        let mut mandatory = DocumentBuilder::new(source);
        mandatory.mandatory_break(3).unwrap();
        let breaks = collect_breaks(&mandatory.build().unwrap(), &(0..6), source, &prepared, &[]);
        assert_eq!(breaks.len(), 1);
        assert!(breaks[0].is_mandatory());

        assert!(
            collect_breaks(
                &plain,
                &(0..6),
                source,
                &prepared,
                &[jlreq_core::Construct::tate_chu_yoko(0..6)]
            )
            .is_empty()
        );
        assert_eq!(
            collect_breaks(
                &plain,
                &(0..6),
                source,
                &prepared,
                &[jlreq_core::Construct::tate_chu_yoko(0..3)]
            )
            .len(),
            1
        );
        assert_eq!(
            collect_breaks(
                &plain,
                &(0..6),
                source,
                &prepared,
                &[jlreq_core::Construct::tate_chu_yoko(3..6)]
            )
            .len(),
            1
        );
    }

    #[test]
    fn discretionary_breaks_are_lowered_between_allowed_and_mandatory() {
        let source = "廃線"; // no automatic opportunity strictly inside a 2-cluster run
        let (_, font, _) = fixture_fonts();
        let prepared = PreparedText {
            clusters: vec![cluster(0..3, font, 0), cluster(3..6, font, 0)],
        };

        let mut discretionary = DocumentBuilder::new(source);
        discretionary.discretionary_break(3).unwrap();
        let breaks = collect_breaks(
            &discretionary.build().unwrap(),
            &(0..6),
            source,
            &prepared,
            &[],
        );
        let inserted = breaks
            .iter()
            .find(|candidate| candidate.offset() == 3)
            .unwrap();
        assert!(inserted.is_discretionary());
        assert!(!inserted.is_mandatory());

        // A mandatory break at the same offset wins over the suggestion.
        let mut both = DocumentBuilder::new(source);
        both.discretionary_break(3).unwrap();
        both.mandatory_break(3).unwrap();
        let breaks = collect_breaks(&both.build().unwrap(), &(0..6), source, &prepared, &[]);
        let inserted = breaks
            .iter()
            .find(|candidate| candidate.offset() == 3)
            .unwrap();
        assert!(inserted.is_mandatory());
    }

    #[test]
    fn frames_and_roles_resolve_assertions_before_heuristics() {
        assert_eq!(
            resolve_frame(crate::MetricsFrame::Auto, "日"),
            jlreq_core::Frame::FullEm
        );
        assert_eq!(
            resolve_frame(crate::MetricsFrame::Auto, "A"),
            jlreq_core::Frame::Proportional
        );
        assert_eq!(
            resolve_frame(crate::MetricsFrame::FullEm, "A"),
            jlreq_core::Frame::FullEm
        );
        assert_eq!(
            resolve_frame(crate::MetricsFrame::Proportional, "日"),
            jlreq_core::Frame::Proportional
        );
        assert_eq!(
            resolve_frame(crate::MetricsFrame::HalfEm, "한"),
            jlreq_core::Frame::HalfEm
        );

        // Default inference stays conservative and data-driven.
        assert_eq!(
            classify_role("3.4", 1..2, TextRole::Text),
            Some(jlreq_core::ClusterRole::DecimalPoint)
        );
        assert_eq!(
            classify_role("1,000", 1..2, TextRole::Text),
            Some(jlreq_core::ClusterRole::DigitGroupSeparator)
        );
        assert_eq!(
            classify_role("あ！", 3..6, TextRole::Text),
            Some(jlreq_core::ClusterRole::SentenceTerminator)
        );
        assert_eq!(
            classify_role("あ！い", 3..6, TextRole::Text),
            Some(jlreq_core::ClusterRole::SentenceMedial)
        );
        assert_eq!(classify_role("ab", 0..1, TextRole::Text), None);

        // Plain suppresses inference; explicit roles override it.
        assert_eq!(
            classify_role("3.4", 1..2, TextRole::Plain),
            Some(jlreq_core::ClusterRole::Text)
        );
        assert_eq!(
            classify_role("あ！い", 3..6, TextRole::Plain),
            Some(jlreq_core::ClusterRole::Text)
        );
        assert_eq!(
            classify_role("あ！い", 3..6, TextRole::SentenceTerminator),
            Some(jlreq_core::ClusterRole::SentenceTerminator)
        );
        assert_eq!(
            classify_role("x", 0..1, TextRole::DecimalPoint),
            Some(jlreq_core::ClusterRole::DecimalPoint)
        );
        assert_eq!(
            classify_role("x", 0..1, TextRole::DigitGroupSeparator),
            Some(jlreq_core::ClusterRole::DigitGroupSeparator)
        );
        assert_eq!(
            classify_role("x", 0..1, TextRole::SentenceMedial),
            Some(jlreq_core::ClusterRole::SentenceMedial)
        );
    }

    #[test]
    fn tab_stops_and_annotation_options_honor_exact_boundaries() {
        let exact = LayoutOptions::try_new(64.0, 16.0)
            .unwrap()
            .with_tab_width(2)
            .unwrap();
        assert_eq!(
            collect_tab_stops("\t", &exact, exact.line_extent, &[])
                .unwrap()
                .iter()
                .map(|stop| stop.position())
                .collect::<Vec<_>>(),
            [2048]
        );
        let bounded = LayoutOptions::try_new(100.0, 16.0)
            .unwrap()
            .with_tab_width(2)
            .unwrap()
            .with_limits(crate::ResourceLimits::default().with_max_constructs(1));
        assert_eq!(
            collect_tab_stops("\t", &bounded, bounded.line_extent, &[])
                .unwrap()
                .len(),
            1
        );
        assert!(
            collect_tab_stops("no tab", &bounded, bounded.line_extent, &[])
                .unwrap()
                .is_empty()
        );

        // Explicit stops replace the generated ladder and carry their
        // alignment through to the core, quantized like every other length.
        let explicit = [
            crate::TabStop::try_new(24.0, crate::TabAlignment::Character('.')).unwrap(),
            crate::TabStop::try_new(48.0, crate::TabAlignment::End).unwrap(),
        ];
        let stops = collect_tab_stops("\t", &exact, exact.line_extent, &explicit).unwrap();
        assert_eq!(
            stops
                .iter()
                .map(|stop| (stop.position(), stop.alignment()))
                .collect::<Vec<_>>(),
            [
                (24 * 64, jlreq_core::TabAlignment::Character('.')),
                (48 * 64, jlreq_core::TabAlignment::End),
            ]
        );
        assert!(
            collect_tab_stops("no tab", &exact, exact.line_extent, &explicit)
                .unwrap()
                .is_empty()
        );

        let source = LayoutOptions::try_new(101.0, 17.0)
            .unwrap()
            .with_alignment(Alignment::End);
        let annotation = annotation_options(&source);
        assert_eq!(annotation.font_size, 544);
        assert_eq!(annotation.line_extent, source.line_extent);
        assert_eq!(annotation.alignment, Alignment::Start);
    }

    #[test]
    fn ruby_run_lowering_handles_declared_group_and_filtered_mono_runs() {
        let (_, id, _) = fixture_fonts();
        let base = PreparedText {
            clusters: vec![
                cluster(0..1, id, 0),
                cluster(1..2, id, 0),
                cluster(2..3, id, 0),
                cluster(3..4, id, 0),
                cluster(4..5, id, 0),
            ],
        };
        let annotation = PreparedText {
            clusters: vec![cluster(0..1, id, 0), cluster(1..2, id, 0)],
        };
        let declared = [crate::RubyRun::new(12..13, 0..1)];
        let runs = ruby_runs(
            crate::RubyKind::Mono,
            &(2..3),
            10,
            &declared,
            &base,
            &annotation,
            2,
        )
        .unwrap();
        assert_eq!(runs[0].base(), 2..3);
        assert_eq!(runs[0].annotation(), 0..1);

        let group = ruby_runs(
            crate::RubyKind::Group,
            &(2..4),
            0,
            &[],
            &base,
            &annotation,
            2,
        )
        .unwrap();
        assert_eq!(group.len(), 1);
        assert_eq!(group[0].base(), 2..4);
        assert_eq!(group[0].annotation(), 0..2);

        let mono = ruby_runs(
            crate::RubyKind::Mono,
            &(2..4),
            0,
            &[],
            &base,
            &annotation,
            2,
        )
        .unwrap();
        assert_eq!(mono.len(), 2);
        assert_eq!(mono[0].base(), 2..3);
        assert_eq!(mono[1].base(), 3..4);
    }

    #[test]
    fn placement_origins_bidi_order_and_physical_geometry_are_exact() {
        let (_, id, _) = fixture_fonts();
        let prepared = PreparedText {
            clusters: vec![
                cluster(0..2, id, 0),
                cluster(2..4, id, 1),
                cluster(4..6, id, 0),
            ],
        };
        assert_eq!(
            placement_cluster_indices(jlreq_core::PlacementOrigin::Cluster(2), &prepared, &(0..6)),
            2..3
        );
        assert_eq!(
            placement_cluster_indices(
                jlreq_core::PlacementOrigin::Construct(0),
                &prepared,
                &(1..5)
            ),
            0..3
        );
        assert_eq!(logical_cluster_order(&[0, 1, 2], 0), [0, 1, 2]);
        assert_eq!(logical_cluster_order(&[0, 1, 2], 1), [2, 1, 0]);

        let raw = raw(id);
        let horizontal = place_raw_glyph(
            &raw,
            &prepared.clusters[0],
            PlacementContext {
                source_range: 0..2,
                annotation: None,
                inline: 320,
                block: 640,
                transform: GlyphTransform::Identity,
                writing_mode: WritingMode::HorizontalTb,
            },
        );
        assert_eq!(horizontal.geometry_26_6(), (320, 832, 256, 0, 64, 128));
        let vertical = place_raw_glyph(
            &raw,
            &prepared.clusters[0],
            PlacementContext {
                source_range: 0..2,
                annotation: None,
                inline: 320,
                block: 640,
                transform: GlyphTransform::Identity,
                writing_mode: WritingMode::VerticalRl,
            },
        );
        assert_eq!(vertical.geometry_26_6(), (640, 320, 0, 384, 64, 128));
        let tate_chu_yoko = place_raw_glyph(
            &raw,
            &prepared.clusters[0],
            PlacementContext {
                source_range: 0..2,
                annotation: None,
                inline: 320,
                block: 640,
                transform: GlyphTransform::TateChuYoko,
                writing_mode: WritingMode::VerticalRl,
            },
        );
        assert_eq!(tate_chu_yoko.geometry_26_6(), (320, 832, 256, 0, 64, 128));
    }

    #[test]
    fn block_and_direction_helpers_distinguish_every_geometry_branch() {
        let horizontal = LayoutOptions::try_new(100.0, 16.0)
            .unwrap()
            .with_line_gap(2.0)
            .unwrap();
        assert_eq!(adjusted_block(10, 3, 20, &horizontal), 414);
        let vertical = horizontal
            .clone()
            .with_writing_mode(WritingMode::VerticalRl);
        assert_eq!(adjusted_block(10, 3, 20, &vertical), -354);

        let (_, id, _) = fixture_fonts();
        let with_vertical_advance = raw(id);
        assert_eq!(
            direction_from_geometry(
                &with_vertical_advance,
                WritingMode::VerticalRl,
                GlyphTransform::Identity
            ),
            Direction::TopToBottom
        );
        assert_eq!(
            direction_from_geometry(
                &with_vertical_advance,
                WritingMode::VerticalRl,
                GlyphTransform::TateChuYoko
            ),
            Direction::LeftToRight
        );
        assert_eq!(
            direction_from_geometry(
                &with_vertical_advance,
                WritingMode::HorizontalTb,
                GlyphTransform::Identity
            ),
            Direction::LeftToRight
        );
        let mut without_vertical_advance = with_vertical_advance.clone();
        without_vertical_advance.y_advance = 0;
        assert_eq!(
            direction_from_geometry(
                &without_vertical_advance,
                WritingMode::VerticalRl,
                GlyphTransform::RotateClockwise
            ),
            Direction::LeftToRight
        );

        assert_eq!(advance_block(100, 30, WritingMode::HorizontalTb), 130);
        assert_eq!(advance_block(100, 30, WritingMode::VerticalRl), 70);
        assert!(ranges_overlap(&(0..2), &(1..3)));
        assert!(!ranges_overlap(&(0..1), &(1..2)));
        assert!(!ranges_overlap(&(1..2), &(0..1)));
    }
}
