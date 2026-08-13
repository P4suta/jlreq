// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Black-box acceptance tests for the intentionally small public API.

use kumihan::{
    Alignment, Break, Cluster, ClusterRole, Construct, CoordinateTransform, Frame, Paragraph, Ruby,
    RubyKind, RubyRun, ShapedText, Size, Style, TabAlignment, TabStop, Widow, WritingMode,
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
        Construct::furawake(0..3, 1, 0),
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
fn vertical_western_text_exposes_upright_rotated_and_tate_chu_yoko_methods() {
    let upright = Paragraph::builder(
        shaped("Ａ", Frame::FullEm, 1_000).expect("valid upright fixture"),
        1_000,
    )
    .writing_mode(WritingMode::VerticalRl)
    .build()
    .expect("valid upright paragraph");
    let upright_layout = kumihan::compose(&upright, &Style::default());
    let upright_cluster = &upright_layout.lines()[0].clusters()[0];
    assert_eq!(upright_cluster.writing_mode(), WritingMode::VerticalRl);
    assert_eq!(upright_cluster.transform(), CoordinateTransform::Identity);

    let rotated = Paragraph::builder(
        shaped("AB", Frame::Proportional, 500).expect("valid rotated fixture"),
        1_000,
    )
    .writing_mode(WritingMode::VerticalRl)
    .build()
    .expect("valid rotated paragraph");
    let rotated_layout = kumihan::compose(&rotated, &Style::default());
    assert!(rotated_layout.lines()[0].clusters().iter().all(|cluster| {
        cluster.writing_mode() == WritingMode::VerticalRl
            && cluster.transform() == CoordinateTransform::RotateClockwise
    }));

    let tate_chu_yoko = Paragraph::builder(
        shaped("12", Frame::Proportional, 500).expect("valid tate-chu-yoko fixture"),
        1_000,
    )
    .constructs([Construct::tate_chu_yoko(0..2)])
    .writing_mode(WritingMode::VerticalRl)
    .build()
    .expect("valid tate-chu-yoko paragraph");
    let tate_chu_yoko_layout = kumihan::compose(&tate_chu_yoko, &Style::default());
    assert!(
        tate_chu_yoko_layout.lines()[0]
            .clusters()
            .iter()
            .all(|cluster| {
                cluster.writing_mode() == WritingMode::HorizontalTb
                    && cluster.transform() == CoordinateTransform::TateChuYoko
            })
    );
}

#[test]
fn tate_chu_yoko_is_one_centered_solid_item_in_a_vertical_line() {
    let text = ShapedText::new(
        "日123本語",
        Size::square(1_000).expect("positive base size"),
        Frame::FullEm,
        [
            Cluster::new(0..3, 1_000),
            Cluster::new(3..4, 400).with_frame(Frame::Proportional),
            Cluster::new(4..5, 400).with_frame(Frame::Proportional),
            Cluster::new(5..6, 400).with_frame(Frame::Proportional),
            Cluster::new(6..9, 1_000),
            Cluster::new(9..12, 1_000),
        ],
    )
    .expect("valid mixed vertical fixture");
    let paragraph = Paragraph::builder(text, 4_000)
        .breaks([Break::mandatory(9)])
        .constructs([Construct::tate_chu_yoko(3..6)])
        .alignment(Alignment::Justify)
        .writing_mode(WritingMode::VerticalRl)
        .build()
        .expect("valid tate-chu-yoko paragraph");

    let layout = kumihan::compose(&paragraph, &Style::default());
    let first = &layout.lines()[0];
    assert_eq!(first.inline_extent(), 4_000);
    assert_eq!(first.block_extent(), 1_200);
    assert_eq!(first.clusters()[0].inline(), 0);
    assert_eq!(first.clusters()[4].inline(), 3_000);

    let digits = &first.clusters()[1..4];
    assert_eq!(digits[0].inline(), 1_500);
    assert_eq!(digits[1].inline(), 1_500);
    assert_eq!(digits[2].inline(), 1_500);
    assert_eq!(digits[0].block(), -600);
    assert_eq!(digits[1].block(), -200);
    assert_eq!(digits[2].block(), 200);
    assert!(digits.iter().all(|cluster| {
        cluster.writing_mode() == WritingMode::HorizontalTb
            && cluster.transform() == CoordinateTransform::TateChuYoko
    }));

    assert_eq!(layout.lines()[1].block_origin(), -1_200);
}

#[test]
fn tate_chu_yoko_punctuation_boundaries_follow_the_directional_half_em_rules() {
    let cases = [
        ("）12", 1_500, None, 2_500),
        ("。12", 1_500, None, 2_500),
        ("、12", 1_500, None, 2_500),
        ("「12", 1_000, None, 2_000),
        ("12（", 0, Some(1_500), 2_500),
        ("12）", 0, Some(1_000), 2_000),
        ("12。", 0, Some(1_000), 2_000),
        ("12、", 0, Some(1_000), 2_000),
    ];

    for (source, expected_digits, expected_following, expected_extent) in cases {
        let digit_start = source.find('1').expect("fixture contains a digit run");
        let clusters = source.char_indices().map(|(start, character)| {
            let cluster = Cluster::new(
                start..start.saturating_add(character.len_utf8()),
                if character.is_ascii_digit() {
                    500
                } else {
                    1_000
                },
            );
            if character.is_ascii_digit() {
                cluster.with_frame(Frame::Proportional)
            } else {
                cluster
            }
        });
        let text = ShapedText::new(
            source,
            Size::square(1_000).expect("positive punctuation fixture size"),
            Frame::FullEm,
            clusters,
        )
        .expect("valid punctuation fixture");
        let paragraph = Paragraph::builder(text, 4_000)
            .constructs([Construct::tate_chu_yoko(
                digit_start..digit_start.saturating_add(2),
            )])
            .writing_mode(WritingMode::VerticalRl)
            .build()
            .expect("valid punctuation paragraph");
        let layout = kumihan::compose(&paragraph, &Style::default());
        let line = &layout.lines()[0];
        let first_digit = line
            .clusters()
            .iter()
            .find(|cluster| cluster.range().start == digit_start)
            .expect("placed first digit");
        assert_eq!(first_digit.inline(), expected_digits, "source {source}");
        if let Some(expected) = expected_following {
            assert_eq!(
                line.clusters()
                    .last()
                    .expect("following punctuation")
                    .inline(),
                expected,
                "source {source}"
            );
        }
        assert_eq!(line.inline_extent(), expected_extent, "source {source}");
    }

    let text = ShapedText::new(
        "。12",
        Size::square(1_000).expect("positive line-end fixture size"),
        Frame::FullEm,
        [
            Cluster::new(0..3, 1_000),
            Cluster::new(3..4, 500).with_frame(Frame::Proportional),
            Cluster::new(4..5, 500).with_frame(Frame::Proportional),
        ],
    )
    .expect("valid line-end fixture");
    let paragraph = Paragraph::builder(text, 2_000)
        .breaks([Break::mandatory(3)])
        .constructs([Construct::tate_chu_yoko(3..5)])
        .writing_mode(WritingMode::VerticalRl)
        .build()
        .expect("valid line-end paragraph");
    let layout = kumihan::compose(&paragraph, &Style::default());
    assert_eq!(layout.lines()[0].inline_extent(), 1_500);
    assert_eq!(layout.lines()[0].clusters()[0].advance(), 1_500);

    let text = ShapedText::new(
        "。x12",
        Size::square(1_000).expect("positive multi-key fixture size"),
        Frame::Proportional,
        [
            Cluster::new(0..4, 1_500),
            Cluster::new(4..5, 500),
            Cluster::new(5..6, 500),
        ],
    )
    .expect("valid indivisible proportional fixture");
    let paragraph = Paragraph::builder(text, 3_000)
        .constructs([Construct::tate_chu_yoko(4..6)])
        .writing_mode(WritingMode::VerticalRl)
        .build()
        .expect("valid multi-key paragraph");
    let layout = kumihan::compose(&paragraph, &Style::default());
    assert_eq!(
        layout.lines()[0].clusters()[1].inline(),
        1_500,
        "a multi-code-point shaping cluster is not classified by its first character alone"
    );
}

#[test]
fn appendix_a_opening_brackets_are_not_limited_to_a_handwritten_subset() {
    let text = shaped("日⦅日", Frame::FullEm, 1_000).expect("valid Appendix A fixture");
    let paragraph = Paragraph::builder(text, 4_000)
        .build()
        .expect("valid Appendix A paragraph");

    let layout = kumihan::compose(&paragraph, &Style::default());
    let line = &layout.lines()[0];
    assert_eq!(
        line.clusters()
            .iter()
            .map(kumihan::ClusterPlacement::inline)
            .collect::<Vec<_>>(),
        vec![0, 1_500, 2_500]
    );
    assert_eq!(line.inline_extent(), 3_500);
}

#[test]
fn table_one_spaces_japanese_and_western_text_by_the_referents_em() {
    fn positions(source: &str) -> (Vec<i32>, i32) {
        let clusters = source.char_indices().map(|(start, character)| {
            let cluster = Cluster::new(
                start..start.saturating_add(character.len_utf8()),
                if character.is_ascii() { 400 } else { 1_000 },
            );
            if character.is_ascii() {
                cluster
                    .with_frame(Frame::Proportional)
                    .with_size(Size::square(400).expect("positive Western size"))
            } else {
                cluster
            }
        });
        let text = ShapedText::new(
            source,
            Size::square(1_000).expect("positive Japanese size"),
            Frame::FullEm,
            clusters,
        )
        .expect("valid mixed-size text");
        let paragraph = Paragraph::builder(text, 3_000)
            .build()
            .expect("valid mixed-size paragraph");
        let layout = kumihan::compose(&paragraph, &Style::default());
        let line = &layout.lines()[0];
        (
            line.clusters()
                .iter()
                .map(kumihan::ClusterPlacement::inline)
                .collect(),
            line.inline_extent(),
        )
    }

    assert_eq!(positions("日A"), (vec![0, 1_250], 1_650));
    assert_eq!(positions("A日"), (vec![0, 650], 1_650));
}

#[test]
fn contextual_decimal_punctuation_withdraws_its_ordinary_space() {
    fn positions(
        source: &str,
        role: Option<(usize, ClusterRole)>,
        mode: WritingMode,
    ) -> (Vec<i32>, i32) {
        let clusters = source
            .char_indices()
            .enumerate()
            .map(|(ordinal, (start, character))| {
                let cluster =
                    Cluster::new(start..start.saturating_add(character.len_utf8()), 1_000);
                if role.is_some_and(|(target, _)| target == ordinal) {
                    cluster.with_role(role.expect("role was present").1)
                } else {
                    cluster
                }
            });
        let text = ShapedText::new(
            source,
            Size::square(1_000).expect("positive punctuation size"),
            Frame::FullEm,
            clusters,
        )
        .expect("valid punctuation text");
        let paragraph = Paragraph::builder(text, 5_000)
            .writing_mode(mode)
            .build()
            .expect("valid punctuation paragraph");
        let layout = kumihan::compose(&paragraph, &Style::default());
        let line = &layout.lines()[0];
        (
            line.clusters()
                .iter()
                .map(kumihan::ClusterPlacement::inline)
                .collect(),
            line.inline_extent(),
        )
    }

    assert_eq!(
        positions("一、二", None, WritingMode::VerticalRl),
        (vec![0, 1_000, 2_500], 3_500),
        "an ordinary ideographic comma retains its following half em"
    );
    assert_eq!(
        positions(
            "一、二",
            Some((1, ClusterRole::DigitGroupSeparator)),
            WritingMode::VerticalRl,
        ),
        (vec![0, 1_000, 2_000], 3_000),
        "a vertical digit-group separator is solid"
    );
    assert_eq!(
        positions("一・五", None, WritingMode::VerticalRl),
        (vec![0, 1_250, 2_500], 3_500),
        "an ordinary middle dot retains both quarter em spaces"
    );
    assert_eq!(
        positions(
            "一・五",
            Some((1, ClusterRole::DecimalPoint)),
            WritingMode::VerticalRl,
        ),
        (vec![0, 1_000, 2_000], 3_000),
        "a vertical decimal point is solid"
    );
    assert_eq!(
        positions(
            "一・五",
            Some((1, ClusterRole::DecimalPoint)),
            WritingMode::HorizontalTb,
        ),
        (vec![0, 1_250, 2_500], 3_500),
        "the main decimal exception remains vertical-only"
    );
    for role in [
        ClusterRole::GroupedNumeral,
        ClusterRole::UnitSymbol,
        ClusterRole::Formula,
    ] {
        assert_eq!(
            positions("A・B", Some((1, role)), WritingMode::HorizontalTb),
            (vec![0, 1_000, 2_000], 3_000),
            "the closing Note's construct roles are solid in the locale union"
        );
    }
}

#[test]
fn western_word_space_collapses_only_at_true_line_edges() {
    fn compose_ascii(
        source: &str,
        breaks: impl IntoIterator<Item = Break>,
        alignment: Alignment,
        line_extent: i32,
    ) -> kumihan::Layout {
        let clusters = source.char_indices().map(|(start, character)| {
            Cluster::new(
                start..start.saturating_add(character.len_utf8()),
                if character == ' ' { 333 } else { 500 },
            )
        });
        let text = ShapedText::new(
            source,
            Size::square(1_000).expect("positive word-space size"),
            Frame::Proportional,
            clusters,
        )
        .expect("valid word-space text");
        let paragraph = Paragraph::builder(text, line_extent)
            .breaks(breaks)
            .alignment(alignment)
            .build()
            .expect("valid word-space paragraph");
        kumihan::compose(&paragraph, &Style::default())
    }

    let edged = compose_ascii(" AB ", [], Alignment::Start, 2_000);
    assert_eq!(
        edged.lines()[0]
            .clusters()
            .iter()
            .map(|cluster| (cluster.inline(), cluster.advance()))
            .collect::<Vec<_>>(),
        vec![(0, 0), (0, 500), (500, 500), (1_000, 0)]
    );
    assert_eq!(edged.lines()[0].inline_extent(), 1_000);

    let interior = compose_ascii("A B", [], Alignment::Start, 2_000);
    assert_eq!(
        interior.lines()[0]
            .clusters()
            .iter()
            .map(|cluster| (cluster.inline(), cluster.advance()))
            .collect::<Vec<_>>(),
        vec![(0, 500), (500, 333), (833, 500)],
        "the caller-supplied width is restored away from an edge"
    );

    let moved_to_edge = compose_ascii("A B", [Break::mandatory(2)], Alignment::Start, 2_000);
    assert_eq!(moved_to_edge.lines()[0].inline_extent(), 500);
    assert_eq!(moved_to_edge.lines()[0].clusters()[1].advance(), 0);
    assert_eq!(moved_to_edge.lines()[1].clusters()[0].inline(), 0);

    let justified = compose_ascii(" A B", [Break::mandatory(3)], Alignment::Justify, 2_000);
    assert_eq!(justified.lines()[0].inline_extent(), 500);
    assert_eq!(
        justified.lines()[0]
            .clusters()
            .iter()
            .map(|cluster| (cluster.inline(), cluster.advance()))
            .collect::<Vec<_>>(),
        vec![(0, 0), (0, 500), (500, 0)],
        "line adjustment does not reopen either suppressed edge space"
    );
}

#[test]
fn warichu_builds_two_balanced_sublines_and_can_straddle_main_lines() {
    fn note_text(source: &str) -> ShapedText {
        let last = source.chars().count().saturating_sub(1);
        let clusters = source
            .char_indices()
            .enumerate()
            .map(|(ordinal, (start, character))| {
                let is_bracket = matches!(character, '(' | ')');
                let cluster = Cluster::new(
                    start..start.saturating_add(character.len_utf8()),
                    if is_bracket { 1_000 } else { 500 },
                )
                .with_size(
                    Size::square(if is_bracket { 1_000 } else { 500 })
                        .expect("positive warichu cluster size"),
                );
                if is_bracket && (ordinal == 0 || ordinal == last) {
                    cluster.with_role(ClusterRole::WarichuBracket)
                } else {
                    cluster
                }
            });
        ShapedText::new(
            source,
            Size::square(1_000).expect("positive main-text size"),
            Frame::FullEm,
            clusters,
        )
        .expect("valid warichu text")
    }

    for mode in [WritingMode::HorizontalTb, WritingMode::VerticalRl] {
        let paragraph = Paragraph::builder(note_text("(abcd)"), 3_000)
            .breaks([Break::allowed(2), Break::allowed(3), Break::allowed(4)])
            .constructs([Construct::warichu(0..6)])
            .writing_mode(mode)
            .build()
            .expect("valid balanced warichu paragraph");
        let layout = kumihan::compose(&paragraph, &Style::default());
        assert_eq!(layout.lines().len(), 1);
        let line = &layout.lines()[0];
        assert_eq!(line.inline_extent(), 3_000);
        assert_eq!(line.block_extent(), 1_000);
        assert_eq!(
            line.clusters()
                .iter()
                .map(kumihan::ClusterPlacement::inline)
                .collect::<Vec<_>>(),
            vec![0, 1_000, 1_500, 1_000, 1_500, 2_000]
        );
        assert_eq!(
            line.clusters()
                .iter()
                .map(kumihan::ClusterPlacement::block)
                .collect::<Vec<_>>(),
            if mode == WritingMode::HorizontalTb {
                vec![0, 0, 0, 500, 500, 0]
            } else {
                vec![0, 0, 0, -500, -500, 0]
            }
        );
    }

    let source = " A B ";
    let clusters = source.char_indices().map(|(start, character)| {
        Cluster::new(
            start..start.saturating_add(character.len_utf8()),
            if character == ' ' { 167 } else { 250 },
        )
        .with_size(Size::square(500).expect("positive small-note size"))
    });
    let text = ShapedText::new(
        source,
        Size::square(1_000).expect("positive main-text size"),
        Frame::Proportional,
        clusters,
    )
    .expect("valid word-space warichu text");
    let paragraph = Paragraph::builder(text, 1_000)
        .breaks([Break::allowed(3)])
        .constructs([Construct::warichu(0..5)])
        .build()
        .expect("word-space breaks are valid inside warichu");
    let layout = kumihan::compose(&paragraph, &Style::default());
    assert_eq!(layout.lines().len(), 1);
    assert_eq!(layout.lines()[0].inline_extent(), 250);
    assert_eq!(
        layout.lines()[0]
            .clusters()
            .iter()
            .map(|cluster| (cluster.inline(), cluster.block(), cluster.advance()))
            .collect::<Vec<_>>(),
        vec![
            (0, 0, 0),
            (0, 0, 250),
            (250, 0, 0),
            (0, 500, 250),
            (250, 500, 0)
        ]
    );

    let paragraph = Paragraph::builder(note_text("(abcdefgh)"), 3_000)
        .breaks((2..9).map(Break::allowed))
        .constructs([Construct::warichu(0..10)])
        .build()
        .expect("valid straddling warichu paragraph");
    let layout = kumihan::compose(&paragraph, &Style::default());
    assert_eq!(layout.lines().len(), 2);
    assert_eq!(layout.lines()[0].range(), 0..5);
    assert_eq!(layout.lines()[1].range(), 5..10);
    assert_eq!(
        layout
            .lines()
            .iter()
            .flat_map(kumihan::Line::clusters)
            .map(kumihan::ClusterPlacement::range)
            .collect::<Vec<_>>(),
        (0..10).map(|start| start..start + 1).collect::<Vec<_>>()
    );
}

#[test]
fn furawake_aligns_declared_sublines_and_never_becomes_an_outer_break() {
    for mode in [WritingMode::HorizontalTb, WritingMode::VerticalRl] {
        let text = shaped("甲乙丙", Frame::FullEm, 1_000).expect("valid furiwake fixture");
        let paragraph = Paragraph::builder(text, 3_000)
            .breaks([Break::mandatory(3)])
            .constructs([Construct::furawake(0..9, 2, 200)])
            .writing_mode(mode)
            .build()
            .expect("one declared split builds two furiwake lines");
        let layout = kumihan::compose(&paragraph, &Style::default());
        assert_eq!(layout.lines().len(), 1);
        let line = &layout.lines()[0];
        assert_eq!(line.inline_extent(), 2_000);
        assert_eq!(line.block_extent(), 2_200);
        assert_eq!(
            line.clusters()
                .iter()
                .map(kumihan::ClusterPlacement::inline)
                .collect::<Vec<_>>(),
            vec![0, 0, 1_000]
        );
        assert_eq!(
            line.clusters()
                .iter()
                .map(kumihan::ClusterPlacement::block)
                .collect::<Vec<_>>(),
            if mode == WritingMode::HorizontalTb {
                vec![-600, 600, 600]
            } else {
                vec![600, -600, -600]
            }
        );
    }

    let text = shaped("甲乙丙", Frame::FullEm, 1_000).expect("valid split-count fixture");
    let error = Paragraph::builder(text, 3_000)
        .constructs([Construct::furawake(0..9, 2, 0)])
        .build()
        .expect_err("two furiwake lines require one declared split");
    assert_eq!(error.code(), "input.furawake-split-count");
}

#[test]
fn formula_spacing_width_and_breaks_follow_math_token_context() {
    let source = "文x=y文";
    let clusters = [
        Cluster::new(0..3, 1_000),
        Cluster::new(3..4, 500)
            .with_frame(Frame::Proportional)
            .with_role(ClusterRole::Formula),
        Cluster::new(4..5, 400).with_role(ClusterRole::Formula),
        Cluster::new(5..6, 500)
            .with_frame(Frame::Proportional)
            .with_role(ClusterRole::Formula),
        Cluster::new(6..9, 1_000),
    ];
    let text = ShapedText::new(
        source,
        Size::square(1_000).expect("positive formula size"),
        Frame::FullEm,
        clusters,
    )
    .expect("valid inline formula");
    let paragraph = Paragraph::builder(text, 5_000)
        .constructs([Construct::formula(3..6)])
        .build()
        .expect("valid inline formula paragraph");
    let layout = kumihan::compose(&paragraph, &Style::default());
    assert_eq!(
        layout.lines()[0]
            .clusters()
            .iter()
            .map(|cluster| (cluster.inline(), cluster.advance()))
            .collect::<Vec<_>>(),
        vec![
            (0, 1_250),
            (1_250, 500),
            (1_750, 1_000),
            (2_750, 750),
            (3_500, 1_000)
        ]
    );
    assert_eq!(layout.lines()[0].inline_extent(), 4_500);

    let text = shaped("a=b+c", Frame::Proportional, 500).expect("valid display formula");
    let paragraph = Paragraph::builder(text, 5_000)
        .constructs([Construct::formula(0..5)])
        .build()
        .expect("valid display formula paragraph");
    let layout = kumihan::compose(&paragraph, &Style::default());
    assert_eq!(
        layout.lines()[0]
            .clusters()
            .iter()
            .map(kumihan::ClusterPlacement::advance)
            .collect::<Vec<_>>(),
        vec![750, 1_250, 500, 1_000, 500],
        "display equations have quarter-em equality spacing and solid operators"
    );

    let text = shaped("a=b", Frame::Proportional, 500).expect("valid formula break fixture");
    let paragraph = Paragraph::builder(text, 2_000)
        .breaks([Break::mandatory(1), Break::allowed(2)])
        .constructs([Construct::formula(0..3)])
        .build()
        .expect("either side of an equality token is a valid caller-declared break");
    let layout = kumihan::compose(&paragraph, &Style::default());
    assert_eq!(layout.lines().len(), 2);
    assert_eq!(layout.lines()[0].inline_extent(), 500);
    assert_eq!(layout.lines()[1].inline_extent(), 1_750);

    let text =
        shaped("ab=cd+ef", Frame::Proportional, 500).expect("valid formula priority fixture");
    let paragraph = Paragraph::builder(text, 4_500)
        .breaks([Break::allowed(2), Break::allowed(5)])
        .constructs([Construct::formula(0..8)])
        .build()
        .expect("valid independent formula alternatives");
    let layout = kumihan::compose(&paragraph, &Style::default());
    assert_eq!(
        layout
            .lines()
            .iter()
            .map(kumihan::Line::range)
            .collect::<Vec<_>>(),
        vec![0..2, 2..8],
        "a feasible break before an equality symbol precedes one before an operator"
    );

    let text = shaped("abc", Frame::Proportional, 500).expect("valid solid formula fixture");
    let error = Paragraph::builder(text, 2_000)
        .breaks([Break::allowed(1)])
        .constructs([Construct::formula(0..3)])
        .build()
        .expect_err("formula letters remain indivisible away from a math token");
    assert_eq!(error.code(), "input.break-inside-construct");
}

#[test]
fn emphasis_dots_are_half_sized_centered_and_reserve_their_side() {
    let source = "日本";
    let text = ShapedText::new(
        source,
        Size::square(1_000).expect("positive base size"),
        Frame::FullEm,
        [
            Cluster::new(0..3, 1_000),
            Cluster::new(3..6, 600).with_size(Size::square(600).expect("positive mixed size")),
        ],
    )
    .expect("valid emphasis fixture");

    for (mode, expected_blocks) in [
        (WritingMode::HorizontalTb, [-500, -300]),
        (WritingMode::VerticalRl, [500, 300]),
    ] {
        let paragraph = Paragraph::builder(text.clone(), 2_000)
            .constructs([Construct::emphasis_dots(0..6, '•')])
            .writing_mode(mode)
            .build()
            .expect("valid emphasis paragraph");
        let layout = kumihan::compose(&paragraph, &Style::default());
        let line = &layout.lines()[0];
        assert_eq!(line.block_extent(), 1_500);
        assert_eq!(line.attachments().len(), 2);

        let first = &line.attachments()[0];
        assert_eq!(first.inline(), 250);
        assert_eq!(first.block(), expected_blocks[0]);
        assert_eq!(first.advance(), 0);
        assert_eq!(first.size().inline(), 500);
        assert_eq!(first.size().block(), 500);
        assert_eq!(first.symbol(), Some('•'));

        let second = &line.attachments()[1];
        assert_eq!(second.inline(), 1_150);
        assert_eq!(second.block(), expected_blocks[1]);
        assert_eq!(second.advance(), 0);
        assert_eq!(second.size().inline(), 300);
        assert_eq!(second.size().block(), 300);
        assert_eq!(second.symbol(), Some('•'));
    }

    let paragraph = Paragraph::builder(text, 2_000)
        .constructs([
            Construct::emphasis_dots(0..3, '•'),
            Construct::emphasis_dots(3..6, '•'),
        ])
        .build()
        .expect("two disjoint emphasis runs are valid");
    let layout = kumihan::compose(&paragraph, &Style::default());
    assert_eq!(layout.lines()[0].attachments().len(), 2);
    assert_eq!(
        layout.lines()[0].block_extent(),
        1_500,
        "disjoint emphasis runs share one side rather than stacking"
    );
}

#[test]
fn ruby_is_on_block_start_and_reserves_the_largest_annotation_size() {
    let annotation = ShapedText::new(
        "にほ",
        Size::square(500).expect("positive ruby size"),
        Frame::FullEm,
        [
            Cluster::new(0..3, 500),
            Cluster::new(3..6, 500).with_size(Size::square(700).expect("positive mixed ruby size")),
        ],
    )
    .expect("valid mixed-size ruby annotation");
    let ruby = Ruby::new(
        RubyKind::Group,
        0..6,
        annotation,
        [RubyRun::new(0..6, 0..6)],
    )
    .expect("valid group ruby");

    for (mode, expected_blocks) in [
        (WritingMode::HorizontalTb, [-500, -700]),
        (WritingMode::VerticalRl, [500, 700]),
    ] {
        let paragraph = Paragraph::builder(
            shaped("日本", Frame::FullEm, 1_000).expect("valid ruby base"),
            2_000,
        )
        .constructs([Construct::ruby(ruby.clone())])
        .writing_mode(mode)
        .build()
        .expect("valid ruby paragraph");
        let layout = kumihan::compose(&paragraph, &Style::default());
        let line = &layout.lines()[0];
        assert_eq!(line.block_extent(), 1_700);
        assert_eq!(line.attachments().len(), 2);
        assert_eq!(line.attachments()[0].inline(), 500);
        assert_eq!(line.attachments()[0].block(), expected_blocks[0]);
        assert_eq!(line.attachments()[1].inline(), 1_000);
        assert_eq!(line.attachments()[1].block(), expected_blocks[1]);
    }
}

#[test]
fn ruby_kinds_preserve_base_associations_and_break_semantics() {
    let annotation = shaped("にほん", Frame::FullEm, 500).expect("valid ruby reading");
    let runs = [RubyRun::new(0..3, 0..3), RubyRun::new(3..6, 3..9)];
    let mono =
        Ruby::new(RubyKind::Mono, 0..6, annotation.clone(), runs.clone()).expect("valid mono ruby");
    let jukugo =
        Ruby::new(RubyKind::Jukugo, 0..6, annotation.clone(), runs).expect("valid jukugo ruby");
    let group = Ruby::new(
        RubyKind::Group,
        0..6,
        annotation,
        [RubyRun::new(0..6, 0..9)],
    )
    .expect("valid group ruby");

    let compose_kind = |ruby| {
        let paragraph = Paragraph::builder(
            shaped("日本", Frame::FullEm, 1_000).expect("valid ruby base"),
            2_000,
        )
        .constructs([Construct::ruby(ruby)])
        .build()
        .expect("valid ruby paragraph");
        kumihan::compose(&paragraph, &Style::default())
    };

    let mono_layout = compose_kind(mono.clone());
    let mono_inline: Vec<_> = mono_layout.lines()[0]
        .attachments()
        .iter()
        .map(kumihan::Attachment::inline)
        .collect();
    assert_eq!(mono_inline, [250, 1_000, 1_500]);

    let jukugo_layout = compose_kind(jukugo.clone());
    let jukugo_inline: Vec<_> = jukugo_layout.lines()[0]
        .attachments()
        .iter()
        .map(kumihan::Attachment::inline)
        .collect();
    assert_eq!(jukugo_inline, mono_inline);

    let group_layout = compose_kind(group.clone());
    let group_inline: Vec<_> = group_layout.lines()[0]
        .attachments()
        .iter()
        .map(kumihan::Attachment::inline)
        .collect();
    assert_eq!(group_inline, [250, 750, 1_250]);

    for (ruby, expected_second_base) in [
        (mono.clone(), 2_000),
        (jukugo.clone(), 1_000),
        (group.clone(), 1_000),
    ] {
        let paragraph = Paragraph::builder(
            shaped("日本語", Frame::FullEm, 1_000).expect("valid adjusted ruby base"),
            3_000,
        )
        .breaks([Break::mandatory(6)])
        .constructs([Construct::ruby(ruby)])
        .alignment(Alignment::Justify)
        .build()
        .expect("valid adjusted ruby paragraph");
        let layout = kumihan::compose(&paragraph, &Style::default());
        assert_eq!(
            layout.lines()[0].clusters()[1].inline(),
            expected_second_base,
            "only mono ruby exposes its internal base boundary to line adjustment"
        );
    }

    for ruby in [mono, jukugo] {
        let paragraph = Paragraph::builder(
            shaped("日本", Frame::FullEm, 1_000).expect("valid split ruby base"),
            1_000,
        )
        .breaks([Break::mandatory(3)])
        .constructs([Construct::ruby(ruby)])
        .build()
        .expect("mono and jukugo may split at a declared run boundary");
        let layout = kumihan::compose(&paragraph, &Style::default());
        assert_eq!(layout.lines().len(), 2);
        assert_eq!(layout.lines()[0].attachments().len(), 1);
        assert_eq!(layout.lines()[0].attachments()[0].range(), 0..3);
        assert_eq!(layout.lines()[1].attachments().len(), 2);
        assert_eq!(layout.lines()[1].attachments()[0].range(), 3..6);
        assert_eq!(layout.lines()[1].attachments()[1].range(), 6..9);
    }

    let error = Paragraph::builder(
        shaped("日本", Frame::FullEm, 1_000).expect("valid group ruby base"),
        1_000,
    )
    .breaks([Break::allowed(3)])
    .constructs([Construct::ruby(group)])
    .build()
    .expect_err("group ruby is indivisible");
    assert_eq!(error.code(), "input.break-inside-construct");
}

#[test]
fn phonetic_jukugo_follows_runs_before_expanding_eligible_base_gaps() {
    fn jukugo(
        base: core::ops::Range<usize>,
        reading: &str,
        runs: impl IntoIterator<Item = RubyRun>,
    ) -> Construct {
        let annotation = shaped(reading, Frame::FullEm, 500).expect("valid jukugo reading");
        Construct::ruby(
            Ruby::new(RubyKind::Jukugo, base, annotation, runs).expect("valid jukugo ruby"),
        )
    }

    let phonetic = Style::builder()
        .jukugo_ruby_layout(JukugoRubyLayout::Phonetic)
        .ruby_alignment(RubyAlignment::Katatsuki)
        .build()
        .expect("valid phonetic jukugo style");
    let compose =
        |source: &str, construct: Construct, extent: i32, end: usize, mode: WritingMode| {
            let paragraph = Paragraph::builder(
                shaped(source, Frame::FullEm, 1_000).expect("valid jukugo base"),
                extent,
            )
            .constructs([construct])
            .breaks([Break::mandatory(end)])
            .alignment(Alignment::Start)
            .writing_mode(mode)
            .build()
            .expect("valid phonetic jukugo paragraph");
            kumihan::compose(&paragraph, &phonetic)
        };

    let forward = compose(
        "前日本あ末",
        jukugo(
            3..9,
            "にほんかな",
            [RubyRun::new(3..6, 0..9), RubyRun::new(6..9, 9..15)],
        ),
        4_000,
        12,
        WritingMode::HorizontalTb,
    );
    assert_eq!(
        forward.lines()[0]
            .clusters()
            .iter()
            .map(kumihan::ClusterPlacement::inline)
            .collect::<Vec<_>>(),
        [0, 1_000, 2_000, 3_000],
        "F.2 prefers overhanging the following base and then the following kana"
    );
    assert_eq!(
        forward.lines()[0]
            .attachments()
            .iter()
            .map(kumihan::Attachment::inline)
            .collect::<Vec<_>>(),
        [1_000, 1_500, 2_000, 2_500, 3_000]
    );

    let backward = compose(
        "あ日本後末",
        jukugo(
            3..9,
            "にほんかな",
            [RubyRun::new(3..6, 0..9), RubyRun::new(6..9, 9..15)],
        ),
        4_000,
        12,
        WritingMode::HorizontalTb,
    );
    assert_eq!(
        backward.lines()[0]
            .clusters()
            .iter()
            .map(kumihan::ClusterPlacement::inline)
            .collect::<Vec<_>>(),
        [0, 1_000, 2_000, 3_000],
        "F.2 falls back to the preceding permitted character when the following one forbids overhang"
    );
    assert_eq!(
        backward.lines()[0]
            .attachments()
            .iter()
            .map(kumihan::Attachment::inline)
            .collect::<Vec<_>>(),
        [500, 1_000, 1_500, 2_000, 2_500]
    );

    for mode in [WritingMode::HorizontalTb, WritingMode::VerticalRl] {
        let expanded = compose(
            "前日本後末",
            jukugo(
                3..9,
                "にほんかな",
                [RubyRun::new(3..6, 0..9), RubyRun::new(6..9, 9..15)],
            ),
            4_500,
            12,
            mode,
        );
        assert_eq!(
            expanded.lines()[0]
                .clusters()
                .iter()
                .map(kumihan::ClusterPlacement::inline)
                .collect::<Vec<_>>(),
            [0, 1_250, 2_500, 3_500],
            "F.3 splits the remaining ruby-character width around the run with three ruby characters"
        );
        assert_eq!(
            expanded.lines()[0]
                .attachments()
                .iter()
                .map(kumihan::Attachment::inline)
                .collect::<Vec<_>>(),
            [1_000, 1_500, 2_000, 2_500, 3_000]
        );
        assert_eq!(expanded.lines()[0].inline_extent(), 4_500);
    }

    let four_ruby = compose(
        "前居候後末",
        jukugo(
            3..9,
            "いそうろう",
            [RubyRun::new(3..6, 0..3), RubyRun::new(6..9, 3..15)],
        ),
        4_500,
        12,
        WritingMode::HorizontalTb,
    );
    assert_eq!(
        four_ruby.lines()[0]
            .clusters()
            .iter()
            .map(kumihan::ClusterPlacement::inline)
            .collect::<Vec<_>>(),
        [0, 1_000, 2_250, 3_500],
        "F.4's four-ruby example assigns one ruby-character width around only the eligible base"
    );
    assert_eq!(
        four_ruby.lines()[0]
            .attachments()
            .iter()
            .map(kumihan::Attachment::inline)
            .collect::<Vec<_>>(),
        [1_000, 1_500, 2_000, 2_500, 3_000]
    );

    let line_head = compose(
        "日本後末",
        jukugo(
            0..6,
            "にほんかな",
            [RubyRun::new(0..3, 0..9), RubyRun::new(3..6, 9..15)],
        ),
        3_500,
        9,
        WritingMode::HorizontalTb,
    );
    assert_eq!(
        line_head.lines()[0]
            .clusters()
            .iter()
            .map(kumihan::ClusterPlacement::inline)
            .collect::<Vec<_>>(),
        [0, 1_500, 2_500],
        "F.3 keeps both starts flush at line head and puts the whole assigned space after the first base"
    );
    assert_eq!(
        line_head.lines()[0]
            .attachments()
            .iter()
            .map(kumihan::Attachment::inline)
            .collect::<Vec<_>>(),
        [0, 500, 1_000, 1_500, 2_000]
    );

    let line_end = compose(
        "前居候末",
        jukugo(
            3..9,
            "いそうろう",
            [RubyRun::new(3..6, 0..3), RubyRun::new(6..9, 3..15)],
        ),
        3_500,
        9,
        WritingMode::HorizontalTb,
    );
    assert_eq!(
        line_end.lines()[0]
            .clusters()
            .iter()
            .map(kumihan::ClusterPlacement::inline)
            .collect::<Vec<_>>(),
        [0, 1_000, 2_500],
        "F.3 keeps both ends flush at line end and puts the whole assigned space before the final base"
    );
    assert_eq!(
        line_end.lines()[0]
            .attachments()
            .iter()
            .map(kumihan::Attachment::inline)
            .collect::<Vec<_>>(),
        [1_000, 1_500, 2_000, 2_500, 3_000]
    );

    let proportional = compose(
        "前日本語後末",
        jukugo(
            3..12,
            "にほんごくみはんじ",
            [
                RubyRun::new(3..6, 0..9),
                RubyRun::new(6..9, 9..21),
                RubyRun::new(9..12, 21..27),
            ],
        ),
        6_500,
        15,
        WritingMode::HorizontalTb,
    );
    assert_eq!(
        proportional.lines()[0]
            .clusters()
            .iter()
            .map(kumihan::ClusterPlacement::inline)
            .collect::<Vec<_>>(),
        [0, 1_322, 3_072, 4_500, 5_500],
        "F.3 apportions the total expansion in the 3:4 solid-reading-length ratio and sends the integer remainder leading"
    );
    assert_eq!(
        proportional.lines()[0]
            .attachments()
            .iter()
            .map(kumihan::Attachment::inline)
            .collect::<Vec<_>>(),
        [
            1_000, 1_500, 2_000, 2_500, 3_000, 3_500, 4_000, 4_500, 5_000
        ]
    );
}

#[test]
fn long_ruby_respects_neighbor_and_indent_overhang_budgets() {
    fn group_ruby(base: core::ops::Range<usize>, reading: &str) -> Ruby {
        let annotation = shaped(reading, Frame::FullEm, 500).expect("valid long ruby reading");
        let annotation_end = annotation.source().len();
        Ruby::new(
            RubyKind::Group,
            base.clone(),
            annotation,
            [RubyRun::new(base, 0..annotation_end)],
        )
        .expect("valid long group ruby")
    }

    let paragraph = Paragraph::builder(
        shaped("前日本後末", Frame::FullEm, 1_000).expect("valid long-ruby base"),
        5_000,
    )
    .constructs([Construct::ruby(group_ruby(3..9, "にほんごかな"))])
    .breaks([Break::mandatory(12)])
    .alignment(Alignment::Start)
    .build()
    .expect("valid long-ruby paragraph");
    let layout = kumihan::compose(&paragraph, &Style::default());
    assert_eq!(
        layout.lines()[0]
            .clusters()
            .iter()
            .map(kumihan::ClusterPlacement::inline)
            .collect::<Vec<_>>(),
        [0, 1_500, 2_500, 4_000],
        "a centered long group ruby forces its two half-surpluses off ideographic neighbors"
    );
    assert_eq!(layout.lines()[0].inline_extent(), 5_000);
    assert_eq!(
        layout.lines()[0]
            .attachments()
            .iter()
            .map(kumihan::Attachment::inline)
            .collect::<Vec<_>>(),
        [1_000, 1_500, 2_000, 2_500, 3_000, 3_500]
    );

    let compose_preceding = |preceding: char, style: Style| {
        let source = format!("{preceding}日後末");
        let paragraph = Paragraph::builder(
            shaped(&source, Frame::FullEm, 1_000).expect("valid overhang-neighbor base"),
            3_500,
        )
        .constructs([Construct::ruby(group_ruby(3..6, "にほん"))])
        .breaks([Break::mandatory(9)])
        .alignment(Alignment::Start)
        .build()
        .expect("valid overhang-neighbor paragraph");
        kumihan::compose(&paragraph, &style)
    };
    let ideograph = compose_preceding('前', Style::default());
    let opening = compose_preceding('「', Style::default());
    let ideographic_space = compose_preceding('　', Style::default());
    assert_eq!(ideograph.lines()[0].clusters()[1].inline(), 1_250);
    assert_eq!(ideograph.lines()[0].attachments()[0].inline(), 1_000);
    for permitted in [&opening, &ideographic_space] {
        assert_eq!(permitted.lines()[0].clusters()[1].inline(), 1_000);
        assert_eq!(permitted.lines()[0].attachments()[0].inline(), 750);
        assert_eq!(permitted.lines()[0].clusters()[2].inline(), 2_250);
    }

    let jis_katakana = compose_preceding('ア', Style::jis_reading_2020());
    let preferred_katakana = compose_preceding('ア', Style::default());
    let any_ideograph = compose_preceding(
        '前',
        Style::builder()
            .ruby_overhang_kana(RubyOverhangKana::Any)
            .build()
            .expect("valid any-neighbor ruby style"),
    );
    let any_fullwidth_latin = compose_preceding(
        'Ａ',
        Style::builder()
            .ruby_overhang_kana(RubyOverhangKana::Any)
            .build()
            .expect("valid any-neighbor ruby style"),
    );
    let no_hiragana = compose_preceding(
        'あ',
        Style::builder()
            .ruby_overhang_kana(RubyOverhangKana::None)
            .build()
            .expect("valid no-neighbor ruby style"),
    );
    assert_eq!(preferred_katakana.lines()[0].clusters()[1].inline(), 1_000);
    assert_eq!(jis_katakana.lines()[0].clusters()[1].inline(), 1_250);
    assert_eq!(any_ideograph.lines()[0].clusters()[1].inline(), 1_000);
    assert_eq!(
        any_fullwidth_latin.lines()[0].clusters()[1].inline(),
        1_000,
        "the Any alternative means every neighboring character, not only Japanese scripts"
    );
    assert_eq!(no_hiragana.lines()[0].clusters()[1].inline(), 1_250);
    let katatsuki_ideograph = compose_preceding(
        '前',
        Style::builder()
            .ruby_alignment(RubyAlignment::Katatsuki)
            .build()
            .expect("valid katatsuki ruby style"),
    );
    assert_eq!(katatsuki_ideograph.lines()[0].clusters()[1].inline(), 1_250);
    assert_eq!(
        katatsuki_ideograph.lines()[0].attachments()[0].inline(),
        1_000,
        "katatsuki falls back to the centered method when adjacent cl-19 forbids overhang"
    );

    let compose_indent = |style: Style| {
        let paragraph = Paragraph::builder(
            shaped("日後末", Frame::FullEm, 1_000).expect("valid ruby-indent base"),
            3_500,
        )
        .constructs([Construct::ruby(group_ruby(0..3, "にほん"))])
        .breaks([Break::mandatory(6)])
        .first_line_indent(1_000)
        .alignment(Alignment::Start)
        .build()
        .expect("valid ruby-indent paragraph");
        kumihan::compose(&paragraph, &style)
    };
    let permitted = compose_indent(Style::default());
    let prohibited = compose_indent(
        Style::builder()
            .ruby_overhang_indent(RubyOverhangIndent::Prohibited)
            .build()
            .expect("valid prohibited-indent style"),
    );
    assert_eq!(permitted.lines()[0].clusters()[0].inline(), 1_000);
    assert_eq!(permitted.lines()[0].attachments()[0].inline(), 750);
    assert_eq!(permitted.lines()[0].inline_extent(), 3_250);
    assert_eq!(prohibited.lines()[0].clusters()[0].inline(), 1_250);
    assert_eq!(prohibited.lines()[0].attachments()[0].inline(), 1_000);
    assert_eq!(prohibited.lines()[0].inline_extent(), 3_500);

    let fixpoint = Paragraph::builder(
        shaped("前日後末", Frame::FullEm, 1_000).expect("valid overhang-fixpoint base"),
        2_000,
    )
    .constructs([Construct::ruby(group_ruby(3..6, "にほんご"))])
    .breaks([Break::allowed(3), Break::allowed(6), Break::allowed(9)])
    .alignment(Alignment::Start)
    .build()
    .expect("valid overhang-fixpoint paragraph");
    let fixpoint = kumihan::compose(&fixpoint, &Style::default());
    assert_eq!(
        fixpoint
            .lines()
            .iter()
            .map(kumihan::Line::range)
            .collect::<Vec<_>>(),
        [0..3, 3..6, 6..12],
        "break search remeasures the ruby after each candidate changes its neighbors"
    );
    assert!(
        fixpoint
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code() != "layout.overfull")
    );
}

#[test]
fn construct_run_boundaries_expand_at_third_order_but_not_inside_one_run() {
    fn positions(
        constructs: impl IntoIterator<Item = Construct>,
        mode: WritingMode,
        inline_extent: i32,
    ) -> Vec<i32> {
        let paragraph = Paragraph::builder(
            shaped("日本語文", Frame::FullEm, 1_000).expect("valid construct-run fixture"),
            inline_extent,
        )
        .constructs(constructs)
        .breaks([Break::mandatory(9)])
        .alignment(Alignment::Justify)
        .writing_mode(mode)
        .build()
        .expect("valid construct-run paragraph");
        kumihan::compose(&paragraph, &Style::default()).lines()[0]
            .clusters()
            .iter()
            .map(kumihan::ClusterPlacement::inline)
            .collect()
    }

    let mark = shaped("注", Frame::FullEm, 500).expect("valid script annotation");
    assert_eq!(
        positions(
            [
                Construct::script(0..3, mark.clone()),
                Construct::script(3..6, mark.clone()),
            ],
            WritingMode::HorizontalTb,
            3_500,
        ),
        [0, 1_250, 2_500],
        "a distinct ornamented complex receives the quarter-em third-order opportunity first"
    );
    assert_eq!(
        positions(
            [Construct::script(0..6, mark)],
            WritingMode::HorizontalTb,
            3_500,
        ),
        [0, 1_000, 2_500],
        "the same ornamented complex stays solid internally"
    );

    let annotation = shaped("にほん", Frame::FullEm, 500).expect("valid ruby annotation");
    let runs = [RubyRun::new(0..3, 0..3), RubyRun::new(3..6, 3..9)];
    let mono =
        Ruby::new(RubyKind::Mono, 0..6, annotation.clone(), runs.clone()).expect("valid mono ruby");
    assert_eq!(
        positions([Construct::ruby(mono)], WritingMode::HorizontalTb, 3_250),
        [0, 1_250, 2_250],
        "each mono-ruby association is a distinct simple-ruby complex"
    );

    let jukugo = Ruby::new(RubyKind::Jukugo, 0..6, annotation, runs).expect("valid jukugo ruby");
    assert_eq!(
        positions([Construct::ruby(jukugo)], WritingMode::HorizontalTb, 3_250,),
        [0, 1_000, 2_250],
        "one jukugo-ruby compound has no internal expansion opportunity"
    );

    assert_eq!(
        positions(
            [
                Construct::tate_chu_yoko(0..3),
                Construct::tate_chu_yoko(3..6),
            ],
            WritingMode::VerticalRl,
            3_250,
        ),
        [0, 1_250, 2_250],
        "distinct tate-chu-yoko runs receive the quarter-em opportunity"
    );

    let same_tcy = Paragraph::builder(
        shaped("日本語文", Frame::FullEm, 1_000).expect("valid tate-chu-yoko fixture"),
        2_250,
    )
    .constructs([Construct::tate_chu_yoko(0..6)])
    .breaks([Break::mandatory(9)])
    .alignment(Alignment::Justify)
    .writing_mode(WritingMode::VerticalRl)
    .build()
    .expect("valid single tate-chu-yoko run");
    let same_tcy = kumihan::compose(&same_tcy, &Style::default());
    assert_eq!(
        same_tcy.lines()[0]
            .clusters()
            .iter()
            .map(kumihan::ClusterPlacement::inline)
            .collect::<Vec<_>>(),
        [0, 0, 1_250],
        "members of one tate-chu-yoko run share an inline item and cannot expand internally"
    );

    let mixed = ShapedText::new(
        "日本語文末",
        Size::square(1_000).expect("positive mixed-size default"),
        Frame::FullEm,
        [
            Cluster::new(0..3, 1_000)
                .with_size(Size::square(800).expect("positive small complex size")),
            Cluster::new(3..6, 1_000)
                .with_size(Size::square(1_200).expect("positive large complex size")),
            Cluster::new(6..9, 1_000),
            Cluster::new(9..12, 1_000),
            Cluster::new(12..15, 1_000),
        ],
    )
    .expect("valid mixed-size complex fixture");
    let mark = shaped("注", Frame::FullEm, 500).expect("valid mixed-size annotation");
    let mixed = Paragraph::builder(mixed, 4_750)
        .constructs([
            Construct::script(0..3, mark.clone()),
            Construct::script(3..6, mark.clone()),
            Construct::script(6..9, mark),
        ])
        .breaks([Break::mandatory(12)])
        .alignment(Alignment::Justify)
        .build()
        .expect("valid mixed-size complex paragraph");
    let mixed = kumihan::compose(&mixed, &Style::default());
    assert_eq!(
        mixed.lines()[0]
            .clusters()
            .iter()
            .map(kumihan::ClusterPlacement::inline)
            .collect::<Vec<_>>(),
        [0, 1_200, 2_500, 3_750],
        "third-order shares use each preceding complex member's em: 200 then 300"
    );
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

    let text = shaped("abc", Frame::Proportional, 1_000).expect("valid formula fixture");
    let internal = Paragraph::builder(text, 4_000)
        .constructs([Construct::formula(0..3)])
        .breaks([Break::allowed(1)])
        .build()
        .expect_err("formula is indivisible away from a math token");
    assert_eq!(internal.code(), "input.break-inside-construct");
}

#[test]
fn mono_ruby_requires_one_run_for_each_base_cluster() {
    let annotation = shaped("にほん", Frame::FullEm, 500).expect("valid annotation");
    let ruby = Ruby::new(RubyKind::Mono, 0..6, annotation, [RubyRun::new(0..6, 0..9)])
        .expect("run partitions are locally valid until base shaping is known");
    let text = shaped("日本", Frame::FullEm, 1_000).expect("valid base");
    let error = Paragraph::builder(text, 4_000)
        .constructs([Construct::ruby(ruby)])
        .build()
        .expect_err("mono ruby cannot associate one run with two base clusters");
    assert_eq!(error.code(), "input.mono-ruby-run-shape");
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
fn segmenter_offsets_can_include_paragraph_boundaries_verbatim() {
    let text = shaped("日本", Frame::FullEm, 1_000).expect("valid segmenter fixture");
    let paragraph = Paragraph::builder(text, 1_000)
        .breaks([Break::allowed(0), Break::allowed(3), Break::allowed(6)])
        .build()
        .expect("segmenter offsets include zero and source length");
    assert_eq!(paragraph.breaks().len(), 2);
    assert_eq!(paragraph.breaks()[0].offset(), 3);
    assert!(paragraph.breaks()[1].is_mandatory());
    assert_eq!(
        kumihan::compose(&paragraph, &Style::default())
            .lines()
            .len(),
        2
    );
}

#[test]
fn table_two_prohibits_a_closing_bracket_at_every_kinsoku_level() {
    let text = shaped("日）日", Frame::FullEm, 1_000).expect("valid kinsoku fixture");
    let paragraph = Paragraph::builder(text, 1_000)
        .breaks([Break::allowed(3), Break::allowed(6)])
        .alignment(Alignment::Start)
        .build()
        .expect("valid break opportunities");

    let layout = kumihan::compose(&paragraph, &Style::newspaper_2020());
    assert_eq!(layout.lines()[0].range(), 0..6);
}

fn lines_at_only_boundary(text: ShapedText, offset: usize, style: &Style) -> Option<usize> {
    let paragraph = Paragraph::builder(text, 1_000)
        .breaks([Break::allowed(offset)])
        .alignment(Alignment::Start)
        .build()
        .ok()?;
    Some(kumihan::compose(&paragraph, style).lines().len())
}

#[test]
fn table_two_note_five_only_keeps_the_five_named_pairs_together() {
    let same = shaped("——", Frame::FullEm, 1_000).expect("valid em-dash pair");
    assert_eq!(lines_at_only_boundary(same, 3, &Style::default()), Some(1));

    let different = shaped("—…", Frame::FullEm, 1_000).expect("valid mixed marks");
    assert_eq!(
        lines_at_only_boundary(different, 3, &Style::default()),
        Some(2)
    );

    let kunojiten = shaped("〳〵", Frame::FullEm, 1_000).expect("valid kunojiten pair");
    assert_eq!(
        lines_at_only_boundary(kunojiten, 3, &Style::default()),
        Some(1)
    );
}

#[test]
fn table_two_note_ten_reads_the_grouped_numeral_choice() {
    let make_text = || {
        ShapedText::new(
            "1A",
            Size::square(1_000).expect("positive size"),
            Frame::Proportional,
            [
                Cluster::new(0..1, 1_000)
                    .with_frame(Frame::HalfEm)
                    .with_role(ClusterRole::GroupedNumeral),
                Cluster::new(1..2, 1_000),
            ],
        )
        .expect("valid grouped numeral fixture")
    };

    assert_eq!(
        lines_at_only_boundary(make_text(), 1, &Style::default()),
        Some(2)
    );
    let unbreakable = Style::builder()
        .grouped_numeral_before_western(GroupedNumeralBeforeWestern::Unbreakable)
        .build()
        .expect("consistent grouped-numeral style");
    assert_eq!(
        lines_at_only_boundary(make_text(), 1, &unbreakable),
        Some(1)
    );
}

#[test]
fn table_two_note_eleven_distinguishes_text_quantity_symbols_and_digits() {
    let with_first = |source: &str, role: Option<ClusterRole>| {
        ShapedText::new(
            source,
            Size::square(1_000).expect("positive size"),
            Frame::FullEm,
            [
                role.map_or_else(
                    || Cluster::new(0..1, 1_000).with_frame(Frame::Proportional),
                    |role| {
                        Cluster::new(0..1, 1_000)
                            .with_frame(Frame::Proportional)
                            .with_role(role)
                    },
                ),
                Cluster::new(1..4, 1_000),
            ],
        )
        .expect("valid postfixed-abbreviation fixture")
    };

    assert_eq!(
        lines_at_only_boundary(with_first("A％", None), 1, &Style::default()),
        Some(2)
    );
    assert_eq!(
        lines_at_only_boundary(
            with_first("A％", Some(ClusterRole::QuantitySymbol)),
            1,
            &Style::default(),
        ),
        Some(1)
    );
    assert_eq!(
        lines_at_only_boundary(with_first("5％", None), 1, &Style::default()),
        Some(1)
    );
}

#[test]
fn c_three_relaxes_only_the_boundaries_named_at_each_level() {
    let before_hyphen = shaped("日‐", Frame::FullEm, 1_000).expect("valid hyphen fixture");
    assert_eq!(
        lines_at_only_boundary(before_hyphen, 3, &Style::newspaper_2020()),
        Some(2)
    );

    let before_middle_dot = shaped("日・", Frame::FullEm, 1_000).expect("valid middle-dot fixture");
    assert_eq!(
        lines_at_only_boundary(before_middle_dot.clone(), 3, &Style::magazine_2020()),
        Some(2)
    );
    assert_eq!(
        lines_at_only_boundary(before_middle_dot, 3, &Style::jlreq_2020()),
        Some(1)
    );
}

#[test]
fn both_relaxation_mechanisms_allow_kana_but_very_strict_does_not() {
    let make_text = || shaped("日ー", Frame::FullEm, 1_000).expect("valid prolonged-mark fixture");
    assert_eq!(
        lines_at_only_boundary(make_text(), 3, &Style::jlreq_2020()),
        Some(2)
    );
    let matrix = Style::builder()
        .relaxation_mechanism(RelaxationMechanism::Matrix)
        .build()
        .expect("consistent matrix style");
    assert_eq!(lines_at_only_boundary(make_text(), 3, &matrix), Some(2));

    let very_strict = Style::builder()
        .kinsoku_level(KinsokuLevel::VeryStrict)
        .grouped_numeral_before_western(GroupedNumeralBeforeWestern::Unbreakable)
        .relaxation_mechanism(RelaxationMechanism::Matrix)
        .build()
        .expect("consistent very-strict style");
    assert_eq!(
        lines_at_only_boundary(make_text(), 3, &very_strict),
        Some(1)
    );
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

#[test]
fn line_extent_is_occupied_width_not_the_shifted_end_coordinate() {
    for (alignment, origin) in [
        (Alignment::Start, 0),
        (Alignment::Center, 1_500),
        (Alignment::End, 3_000),
    ] {
        let text = shaped("日", Frame::FullEm, 1_000).expect("valid alignment fixture");
        let paragraph = Paragraph::builder(text, 4_000)
            .alignment(alignment)
            .build()
            .expect("valid aligned paragraph");
        let layout = kumihan::compose(&paragraph, &Style::default());
        assert_eq!(layout.lines()[0].inline_origin(), origin);
        assert_eq!(layout.lines()[0].inline_extent(), 1_000);
    }
}
