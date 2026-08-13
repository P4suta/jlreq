// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Black-box acceptance tests for the intentionally small public API.

use kumihan::{
    Alignment, Break, Cluster, Construct, Frame, Paragraph, Ruby, RubyKind, RubyRun, ShapedText,
    Size, Style, TabAlignment, TabStop, Widow, WritingMode,
    style::{
        AdjustmentPreference, AmbiguousContext, ExpansionOrder, GroupRubyDistribution,
        GroupedNumeralBeforeWestern, GroupedNumeralQualification, HangingPunctuation,
        IterationMarkAtLineHead, JapaneseLatinExpansionCeiling, JukugoRubyLayout, KinsokuLevel,
        LineEndFullStopComma, LineEndPunctuation, LineHeadOpeningBracket, ReductionTable,
        RelaxationMechanism, Remainder, RubyAlignment, RubyOverhangIndent, RubyOverhangKana,
        SentenceMedialDividingMark, UnlistedCodePoint,
    },
};

fn shaped(source: &str, frame: Frame, advance: i32) -> Result<ShapedText, kumihan::InputError> {
    let clusters = source.char_indices().map(|(start, character)| {
        Cluster::new(start..start.saturating_add(character.len_utf8()), advance)
    });
    ShapedText::new(source, Size::square(1_000)?, frame, clusters)
}

#[test]
fn every_one_of_the_twenty_two_settings_accepts_a_non_default_choice() {
    macro_rules! builds {
        ($builder:expr) => {
            assert!($builder.build().is_ok());
        };
    }

    builds!(
        Style::builder()
            .kinsoku_level(KinsokuLevel::VeryStrict)
            .grouped_numeral_before_western(GroupedNumeralBeforeWestern::Unbreakable)
            .relaxation_mechanism(RelaxationMechanism::Matrix)
    );
    builds!(Style::builder().reduction_table(ReductionTable::Table4));
    builds!(Style::builder().line_end_punctuation(LineEndPunctuation::Solid));
    builds!(Style::builder().line_end_full_stop_comma(LineEndFullStopComma::Jis));
    builds!(Style::builder().line_head_opening_bracket(LineHeadOpeningBracket::Pattern2));
    builds!(Style::builder().ruby_overhang_kana(RubyOverhangKana::None));
    builds!(Style::builder().ruby_overhang_indent(RubyOverhangIndent::Prohibited));
    builds!(Style::builder().ruby_alignment(RubyAlignment::Katatsuki));
    builds!(Style::builder().group_ruby_distribution(GroupRubyDistribution::Flush));
    builds!(Style::builder().jukugo_ruby_layout(JukugoRubyLayout::Phonetic));
    builds!(Style::builder().iteration_mark_at_line_head(IterationMarkAtLineHead::Replaced));
    builds!(Style::builder().hanging_punctuation(HangingPunctuation::Hanging));
    builds!(
        Style::builder().grouped_numeral_before_western(GroupedNumeralBeforeWestern::Unbreakable)
    );
    builds!(Style::builder().sentence_medial_dividing_mark(SentenceMedialDividingMark::QuarterEm));
    builds!(
        Style::builder().japanese_latin_expansion_ceiling(JapaneseLatinExpansionCeiling::Rigid)
    );
    builds!(Style::builder().expansion_order(ExpansionOrder::Implementation));
    builds!(Style::builder().adjustment_preference(AdjustmentPreference::EvenTexture));
    builds!(Style::builder().remainder(Remainder::Trailing));
    builds!(Style::builder().unlisted_code_point(UnlistedCodePoint::Ideographic));
    builds!(Style::builder().ambiguous_context(AmbiguousContext::HighestClass));
    builds!(Style::builder().grouped_numeral_qualification(GroupedNumeralQualification::ByRole));
    builds!(Style::builder().relaxation_mechanism(RelaxationMechanism::Matrix));
}

#[test]
fn contradictory_style_combinations_have_stable_codes() {
    let grouped = Style::builder()
        .kinsoku_level(KinsokuLevel::VeryStrict)
        .relaxation_mechanism(RelaxationMechanism::Matrix)
        .build()
        .expect_err("default breakable grouped numerals conflict");
    assert_eq!(grouped.code(), "style.very-strict-grouped-numeral");

    let relaxation = Style::builder()
        .kinsoku_level(KinsokuLevel::VeryStrict)
        .grouped_numeral_before_western(GroupedNumeralBeforeWestern::Unbreakable)
        .build()
        .expect_err("default reclassification conflicts");
    assert_eq!(relaxation.code(), "style.very-strict-relaxation");
}

#[test]
fn default_is_permanently_the_dated_jlreq_profile() {
    assert_eq!(Style::default(), Style::jlreq_2020());
    assert_ne!(Style::book_2020(), Style::jlreq_2020());
    assert_ne!(Style::magazine_2020(), Style::newspaper_2020());
    assert_ne!(Style::jis_reading_2020(), Style::jlreq_2020());
}

#[test]
fn all_nine_constructs_compose_in_both_writing_modes() {
    let annotation = shaped("に", Frame::FullEm, 500).expect("valid annotation");
    let ruby = Ruby::new(
        RubyKind::Group,
        0..3,
        annotation.clone(),
        [RubyRun::new(0..3, 0..3)],
    )
    .expect("valid group ruby");
    let constructs = [
        Construct::ruby(ruby),
        Construct::tate_chu_yoko(0..3),
        Construct::emphasis_dots(0..3, '・'),
        Construct::warichu(0..3),
        Construct::furawake(0..3, 2),
        Construct::jidori(0..3, 2),
        Construct::reference_mark(0..3, annotation.clone()),
        Construct::script(0..3, annotation),
        Construct::formula(0..3),
    ];

    for mode in [WritingMode::HorizontalTb, WritingMode::VerticalRl] {
        for construct in &constructs {
            let text = shaped("日", Frame::FullEm, 1_000).expect("valid base");
            let paragraph = Paragraph::builder(text, 2_000)
                .constructs([construct.clone()])
                .writing_mode(mode)
                .build()
                .expect("valid construct paragraph");
            let layout = kumihan::compose(&paragraph, &Style::default());
            assert_eq!(layout.lines().len(), 1);
            assert_eq!(layout.lines()[0].clusters().len(), 1);
        }
    }
}

#[test]
fn appendix_pair_is_normalized_across_shaping_clusters() {
    let source = "\u{31f7}\u{309a}";
    let first_end = '\u{31f7}'.len_utf8();
    let text = ShapedText::new(
        source,
        Size::square(1_000).expect("positive size"),
        Frame::FullEm,
        [
            Cluster::new(0..first_end, 500),
            Cluster::new(first_end..source.len(), 0),
        ],
    )
    .expect("the normalizer accepts split shaping clusters");
    let error = Paragraph::builder(text, 1_000)
        .breaks([Break::allowed(first_end)])
        .build()
        .expect_err("a break may not split one Appendix A key");
    assert_eq!(error.code(), "input.break-splits-cluster");
}

#[test]
fn non_latin_cluster_cannot_hide_two_appendix_keys() {
    let error = ShapedText::new(
        "日本",
        Size::square(1_000).expect("positive size"),
        Frame::FullEm,
        [Cluster::new(0..6, 2_000)],
    )
    .expect_err("two Japanese keys cannot be one shaped cluster");
    assert_eq!(error.code(), "input.cluster-covers-multiple-keys");

    assert!(
        ShapedText::new(
            "ffi",
            Size::square(1_000).expect("positive size"),
            Frame::Proportional,
            [Cluster::new(0..3, 1_500)],
        )
        .is_ok()
    );
}

#[test]
fn invalid_utf8_crossing_ranges_and_construct_internal_breaks_are_rejected() {
    let utf8 = ShapedText::new(
        "日",
        Size::square(1_000).expect("positive size"),
        Frame::FullEm,
        [Cluster::new(0..1, 1_000)],
    )
    .expect_err("byte one is inside a code point");
    assert_eq!(utf8.code(), "input.invalid-utf8-boundary");

    let text = shaped("日本語", Frame::FullEm, 1_000).expect("valid crossing fixture");
    let crossing = Paragraph::builder(text, 4_000)
        .constructs([
            Construct::emphasis_dots(0..6, '・'),
            Construct::formula(3..9),
        ])
        .build()
        .expect_err("constructs cross");
    assert_eq!(crossing.code(), "input.crossing-constructs");

    let text = shaped("日本", Frame::FullEm, 1_000).expect("valid formula fixture");
    let internal = Paragraph::builder(text, 4_000)
        .constructs([Construct::formula(0..6)])
        .breaks([Break::allowed(3)])
        .build()
        .expect_err("formula is indivisible");
    assert_eq!(internal.code(), "input.break-inside-construct");
}

#[test]
fn mandatory_discretionary_widow_and_tabs_share_the_paragraph_pipeline() {
    let source = "A\tB";
    let stop = TabStop::new(3_000, TabAlignment::Start).expect("valid tab stop");
    let text = shaped(source, Frame::Proportional, 500).expect("valid tab fixture");
    let tabbed = Paragraph::builder(text, 5_000)
        .tab_stops([stop])
        .alignment(Alignment::Start)
        .build()
        .expect("valid tab paragraph");
    let layout = kumihan::compose(&tabbed, &Style::default());
    assert_eq!(layout.lines()[0].clusters()[2].inline(), 3_000);

    let source = "日本語";
    let text = shaped(source, Frame::FullEm, 1_000).expect("valid break fixture");
    let paragraph = Paragraph::builder(text, 3_000)
        .breaks([Break::discretionary(3), Break::mandatory(6)])
        .widow(Widow::MinimumClusters(2))
        .build()
        .expect("valid mixed-break paragraph");
    let layout = kumihan::compose(&paragraph, &Style::default());
    assert_eq!(layout.lines().len(), 2);
    assert_eq!(layout.lines()[0].range(), 0..6);
}

#[test]
fn composer_reuses_scratch_without_borrowing_the_returned_layout() {
    let text = shaped("日本", Frame::FullEm, 1_000).expect("valid reuse fixture");
    let paragraph = Paragraph::builder(text, 1_000)
        .breaks([Break::allowed(3)])
        .build()
        .expect("valid paragraph");
    let mut composer = kumihan::Composer::new();
    let first = composer.compose(&paragraph, &Style::default());
    let second = composer.compose(&paragraph, &Style::book_2020());
    assert_eq!(first.lines().len(), 2);
    assert_eq!(second.lines().len(), 2);
}
