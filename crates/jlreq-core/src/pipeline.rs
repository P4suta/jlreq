// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::{vec, vec::Vec};
use core::ops::Range;

use crate::construct::{
    ConstructKind, Ruby, RubyKind, is_math_operator, is_math_symbol, is_math_token,
};
use crate::layout::{
    Attachment, ClusterPlacement, CoordinateTransform, Diagnostic, Layout, Line, PlacementOrigin,
    Severity,
};
use crate::limits::{ComposeError, CompositionLimits, CompositionResource};
use crate::model::{ClusterRole, Frame, Size, WritingMode};
use crate::paragraph::{Alignment, Paragraph, TabAlignment, TabStop, Widow};
use crate::style::{
    AdjustmentPreference, AmbiguousContext, GroupRubyDistribution, GroupedNumeralBeforeWestern,
    GroupedNumeralQualification, HangingPunctuation, IterationMarkAtLineHead,
    JapaneseLatinExpansionCeiling, JukugoRubyLayout, KinsokuLevel, LineEndFullStopComma,
    LineEndPunctuation, LineHeadOpeningBracket, ReductionTable, RelaxationMechanism, Remainder,
    RubyAlignment, RubyOverhangIndent, RubyOverhangKana, SentenceMedialDividingMark, Style,
    UnlistedCodePoint,
};

const INFINITE_COST: u128 = u128::MAX;
// These stage files expand in this module so the private pipeline contract and public API
// remain unchanged while each composition phase can be maintained independently.
include!("pipeline/composition.rs");
include!("pipeline/search.rs");
include!("pipeline/adjustment.rs");
include!("pipeline/ruby.rs");
include!("pipeline/special.rs");
include!("pipeline/placement.rs");
#[cfg(test)]
mod tests {
    use alloc::{string::String, vec, vec::Vec};
    use core::ops::Range;

    use crate::construct::{Construct, Ruby, RubyKind, RubyRun};
    use crate::model::{Cluster, ClusterRole, Frame, ShapedText, Size, WritingMode};
    use crate::paragraph::{Break, Paragraph, Widow};
    use crate::style::{Remainder, Style};

    fn text(source: &str) -> ShapedText {
        let clusters = source.char_indices().map(|(start, character)| {
            Cluster::new(start..start.saturating_add(character.len_utf8()), 1_000)
        });
        ShapedText::new(
            source,
            Size::square(1_000).expect("positive fixture size"),
            Frame::FullEm,
            clusters,
        )
        .expect("valid fixture text")
    }

    fn mapped_text(
        source: &str,
        frame: Frame,
        mut map: impl FnMut(usize, Cluster) -> Cluster,
    ) -> ShapedText {
        let clusters = source
            .char_indices()
            .enumerate()
            .map(|(ordinal, (start, character))| {
                map(
                    ordinal,
                    Cluster::new(start..start.saturating_add(character.len_utf8()), 1_000),
                )
            });
        ShapedText::new(
            source,
            Size::square(1_000).expect("positive fixture size"),
            frame,
            clusters,
        )
        .expect("valid mapped text")
    }

    fn ruby(kind: RubyKind, base: Range<usize>, annotation: &str, runs: Vec<RubyRun>) -> Ruby {
        Ruby::new(kind, base, text(annotation), runs).expect("valid ruby fixture")
    }

    fn break_everywhere(source: &str, extent: i32, mode: WritingMode) -> Paragraph {
        Paragraph::builder(text(source), extent)
            .breaks(
                source
                    .char_indices()
                    .skip(1)
                    .map(|(offset, _)| Break::allowed(offset)),
            )
            .writing_mode(mode)
            .build()
            .expect("valid generated paragraph")
    }

    fn placement(
        ordinal: usize,
        range: Range<usize>,
        inline: i32,
        advance: i32,
        transform: crate::CoordinateTransform,
    ) -> crate::ClusterPlacement {
        crate::ClusterPlacement {
            origin: crate::PlacementOrigin::Cluster(ordinal),
            range,
            inline,
            block: 0,
            advance,
            size: Size::square(9).expect("positive placement size"),
            frame: Frame::FullEm,
            writing_mode: WritingMode::HorizontalTb,
            transform,
        }
    }

    fn line(range: Range<usize>, clusters: Vec<crate::ClusterPlacement>) -> crate::Line {
        crate::Line {
            range,
            inline_origin: 0,
            block_origin: 1_000,
            inline_extent: 10_000,
            block_extent: 1_000,
            clusters,
            attachments: Vec::new(),
        }
    }

    fn oracle_chosen(
        paragraph: &Paragraph,
        style: &Style,
        candidates: &[super::Candidate],
    ) -> Vec<usize> {
        let mut nodes = vec![
            super::Node {
                cost: super::INFINITE_COST,
                previous: 0,
                line_count: 0,
            };
            candidates.len()
        ];
        nodes[0] = super::Node {
            cost: 0,
            previous: 0,
            line_count: 0,
        };
        for end in 1..candidates.len() {
            let candidate = candidates[end];
            if !candidate.mandatory && !super::break_is_legal(paragraph, style, candidate.offset) {
                continue;
            }
            for start in 0..end {
                if nodes[start].cost == super::INFINITE_COST
                    || candidates[start.saturating_add(1)..end]
                        .iter()
                        .any(|inner| inner.mandatory)
                {
                    continue;
                }
                let line_number = nodes[start].line_count;
                let measured = super::measure_line(
                    paragraph,
                    style,
                    candidates[start].offset,
                    candidate.offset,
                    line_number,
                );
                let available = i64::from(paragraph.line_extent);
                let width = super::width_after_available_reduction(
                    paragraph,
                    style,
                    candidates[start].offset,
                    candidate.offset,
                    measured,
                    available,
                );
                let is_last = end.saturating_add(1) == candidates.len();
                let mut edge = super::non_negative_cost(super::line_badness(
                    available.saturating_sub(width),
                    is_last,
                    style.adjustment_preference(),
                ));
                if candidate.discretionary {
                    edge = edge.saturating_add(100_000);
                }
                edge = edge.saturating_add(super::non_negative_cost(super::warichu_break_penalty(
                    paragraph,
                    candidate.offset,
                )));
                edge = edge.saturating_add(super::non_negative_cost(super::formula_break_penalty(
                    paragraph,
                    candidate.offset,
                )));
                if is_last {
                    edge = edge.saturating_add(super::non_negative_cost(super::widow_penalty(
                        paragraph,
                        candidates[start].offset,
                        candidate.offset,
                    )));
                }
                let cost = nodes[start].cost.saturating_add(edge);
                if cost < nodes[end].cost {
                    nodes[end] = super::Node {
                        cost,
                        previous: start,
                        line_count: line_number.saturating_add(1),
                    };
                }
            }
        }
        let mut chosen = Vec::new();
        let mut cursor = nodes.len().saturating_sub(1);
        chosen.push(cursor);
        while cursor != 0 {
            cursor = nodes[cursor].previous;
            chosen.push(cursor);
        }
        chosen.reverse();
        chosen
    }

    #[test]
    fn prepared_and_composer_reset_clear_every_cached_field() {
        let mut prepared = super::PreparedParagraph::new();
        prepared.candidate_ordinals.push(1);
        prepared.legal_candidates.push(true);
        prepared.natural_prefix.push(2);
        prepared.minimum_prefix.push(3);
        prepared.reduction_prefix.push(4);
        prepared.line_end_reduction.push(5);
        prepared.regular = true;
        prepared.clear();
        assert!(prepared.candidate_ordinals.is_empty());
        assert!(prepared.legal_candidates.is_empty());
        assert!(prepared.natural_prefix.is_empty());
        assert!(prepared.minimum_prefix.is_empty());
        assert!(prepared.reduction_prefix.is_empty());
        assert!(prepared.line_end_reduction.is_empty());
        assert!(!prepared.regular);

        let mut composer = super::Composer::new();
        composer.transitions = 9;
        composer.candidates.push(super::Candidate {
            offset: 1,
            mandatory: false,
            discretionary: true,
        });
        composer.nodes.push(super::Node {
            cost: 1,
            previous: 2,
            line_count: 3,
        });
        composer.chosen.push(1);
        composer.line_advances.push(2);
        composer.line_adjustments.push(3);
        composer
            .line_scratch
            .expansion_sites
            .push(super::ExpansionSite::None);
        composer.line_scratch.construct_ordinals.push(4);
        composer.line_scratch.distribution.assigned.push(5);
        composer.prepared.regular = true;
        composer.reset_for_call();
        assert_eq!(composer.transitions, 0);
        assert!(composer.candidates.is_empty());
        assert!(composer.nodes.is_empty());
        assert!(composer.chosen.is_empty());
        assert!(composer.line_advances.is_empty());
        assert!(composer.line_adjustments.is_empty());
        assert!(composer.line_scratch.expansion_sites.is_empty());
        assert!(composer.line_scratch.construct_ordinals.is_empty());
        assert!(composer.line_scratch.distribution.assigned.is_empty());
        assert!(!composer.prepared.regular);
        assert!(!composer.prepared.fast_measure);
    }

    #[test]
    fn numeric_prefix_and_limit_helpers_have_inclusive_edges() {
        assert_eq!(super::non_negative_cost(-1), 0);
        assert_eq!(super::non_negative_cost(0), 0);
        assert_eq!(super::non_negative_cost(17), 17);
        assert!(
            super::check_limit(crate::CompositionResource::Clusters, 3, 3).is_ok(),
            "the declared limit is inclusive"
        );
        let error = super::check_limit(crate::CompositionResource::Clusters, 3, 4)
            .expect_err("one past the limit");
        assert_eq!(error.limit(), 3);
        assert_eq!(error.observed(), 4);

        let prefix = [0, 2, 5, 9];
        assert_eq!(super::range_sum(&prefix, 1, 3), 7);
        assert_eq!(super::range_sum(&prefix, 2, 2), 0);
        assert_eq!(super::range_sum(&prefix, 0, 9), i64::MAX);
        let prepared = super::PreparedParagraph {
            minimum_prefix: prefix.to_vec(),
            ..super::PreparedParagraph::new()
        };
        assert_eq!(super::fast_minimum_width(&prepared, 1, 3), 3);
        assert_eq!(super::fast_minimum_width(&prepared, 1, 0), 0);

        let paragraph = break_everywhere("abc", 3_000, WritingMode::HorizontalTb);
        assert_eq!(super::cluster_index_at_or_after(&paragraph, 0), 0);
        assert_eq!(super::cluster_index_at_or_after(&paragraph, 1), 1);
        assert_eq!(super::cluster_index_at_or_after(&paragraph, 2), 2);
        assert_eq!(super::cluster_index_at_or_after(&paragraph, 3), 3);
        assert_eq!(super::cluster_index_at_or_after(&paragraph, 4), 3);

        let current = super::Node {
            cost: 10,
            previous: 3,
            line_count: 9,
        };
        assert!(super::search_candidate_precedes(9, 99, current));
        assert!(super::search_candidate_precedes(10, 2, current));
        assert!(!super::search_candidate_precedes(10, 3, current));
        assert!(!super::search_candidate_precedes(10, 4, current));
        assert!(!super::search_candidate_precedes(11, 0, current));

        let preference = crate::style::AdjustmentPreference::LeastAdjustment;
        let strict_bound = super::non_negative_cost(super::line_badness(-1, false, preference));
        assert!(!super::search_lower_bound_exceeds(
            10, 10, false, preference, 0
        ));
        assert!(!super::search_lower_bound_exceeds(
            9, 10, false, preference, 0
        ));
        assert!(super::search_lower_bound_exceeds(
            11,
            10,
            false,
            preference,
            strict_bound.saturating_sub(1)
        ));
        assert!(!super::search_lower_bound_exceeds(
            11,
            10,
            false,
            preference,
            strict_bound
        ));

        assert!(super::line_should_justify(
            crate::Alignment::Justify,
            false,
            1,
            2
        ));
        assert!(!super::line_should_justify(
            crate::Alignment::Justify,
            true,
            1,
            2
        ));
        assert!(!super::line_should_justify(
            crate::Alignment::Justify,
            false,
            0,
            2
        ));
        assert!(!super::line_should_justify(
            crate::Alignment::Justify,
            false,
            1,
            1
        ));
        assert!(!super::line_should_justify(
            crate::Alignment::Start,
            false,
            1,
            2
        ));
        assert_eq!(super::line_adjustment_need(-1, false), -1);
        assert_eq!(super::line_adjustment_need(0, true), 0);
        assert_eq!(super::line_adjustment_need(1, true), 1);
        assert_eq!(super::line_adjustment_need(1, false), 0);
    }

    #[test]
    fn indexed_and_special_search_charge_exact_transition_work() {
        let regular = Paragraph::builder(text("AB"), 4_000)
            .build()
            .expect("valid regular paragraph");
        let mut composer = super::Composer::new();
        composer
            .compose(&regular, &Style::default())
            .expect("regular search succeeds");
        assert_eq!(composer.transitions, 1);

        let formula = Paragraph::builder(text("AB"), 4_000)
            .constructs([Construct::formula(0..2)])
            .build()
            .expect("valid non-regular paragraph");
        composer
            .compose(&formula, &Style::default())
            .expect("special search succeeds");
        assert_eq!(composer.transitions, 4);
    }

    #[test]
    fn fast_measure_trims_leading_space_only_when_it_is_not_also_trailing() {
        fn assert_fast_matches_full(source: &str, expected: i64) {
            let paragraph = Paragraph::builder(
                mapped_text(source, Frame::Proportional, |_, cluster| cluster),
                4_000,
            )
            .build()
            .expect("valid proportional paragraph");
            let style = Style::default();
            let mut composer = super::Composer::new();
            composer.prepare_candidates(&paragraph);
            composer.prepare_indexes(&paragraph, &style);
            assert!(composer.prepared.regular);
            let cluster_end = paragraph.text.clusters().len();
            assert_eq!(
                super::measure_line(&paragraph, &style, 0, source.len(), 0),
                expected
            );
            assert_eq!(
                super::fast_measure_line(&composer.prepared, &paragraph, &style, 0, cluster_end, 0),
                expected
            );
        }

        assert_fast_matches_full(" ", 0);
        assert_fast_matches_full(" A", 1_000);
    }

    #[test]
    fn static_construct_prefixes_match_full_measurement_and_keep_legacy_charges() {
        let cases = [
            Paragraph::builder(text("ABC"), 4_000)
                .constructs([Construct::emphasis_dots(1..2, '・')])
                .build()
                .expect("valid emphasis paragraph"),
            Paragraph::builder(text("ABC"), 4_000)
                .constructs([Construct::reference_mark(1..2, text("*"))])
                .build()
                .expect("valid reference-mark paragraph"),
            Paragraph::builder(text("ABC"), 4_000)
                .constructs([Construct::script(1..2, text("x"))])
                .build()
                .expect("valid script paragraph"),
            Paragraph::builder(text("A=B"), 4_000)
                .constructs([Construct::formula(0..3)])
                .build()
                .expect("valid formula paragraph"),
            Paragraph::builder(text("ABC"), 4_000)
                .constructs([Construct::tate_chu_yoko(1..3)])
                .writing_mode(WritingMode::VerticalRl)
                .build()
                .expect("valid tate-chu-yoko paragraph"),
        ];
        let style = Style::default();
        for paragraph in cases {
            let mut composer = super::Composer::new();
            composer.prepare_candidates(&paragraph);
            composer.prepare_indexes(&paragraph, &style);
            assert!(composer.prepared.fast_measure);
            assert!(!composer.prepared.regular);
            let cluster_end = paragraph.text.clusters().len();
            let measured =
                super::measure_line(&paragraph, &style, 0, paragraph.text.source().len(), 0);
            let fast =
                super::fast_measure_line(&composer.prepared, &paragraph, &style, 0, cluster_end, 0);
            assert_eq!(fast, measured);
            for available in [1_000_i64, 4_000] {
                assert_eq!(
                    super::fast_width_after_available_reduction(
                        &composer.prepared,
                        &paragraph,
                        &style,
                        0,
                        cluster_end,
                        fast,
                        available,
                    ),
                    super::width_after_available_reduction(
                        &paragraph,
                        &style,
                        0,
                        paragraph.text.source().len(),
                        measured,
                        available,
                    )
                );
            }
        }
    }

    #[test]
    fn construct_cluster_ranges_and_internal_boundaries_are_exact() {
        let furawake = Paragraph::builder(text("abc"), 3_000)
            .breaks([Break::allowed(1)])
            .constructs([Construct::furawake(0..2, 2, 17)])
            .build()
            .expect("valid furawake");
        assert_eq!(
            super::furawake_cluster_range(&furawake, 0),
            Some((0..2, 2, 17))
        );
        assert_eq!(
            super::furawake_cluster_range(&furawake, 1),
            Some((0..2, 2, 17))
        );
        assert_eq!(super::furawake_cluster_range(&furawake, 2), None);
        assert!(!super::is_internal_furawake_offset(&furawake, 0));
        assert!(super::is_internal_furawake_offset(&furawake, 1));
        assert!(!super::is_internal_furawake_offset(&furawake, 2));

        let jidori = Paragraph::builder(text("abc"), 3_000)
            .constructs([Construct::jidori(0..2, 3)])
            .build()
            .expect("valid jidori");
        assert_eq!(super::jidori_cluster_range(&jidori, 0), Some((0..2, 3)));
        assert_eq!(super::jidori_cluster_range(&jidori, 1), Some((0..2, 3)));
        assert_eq!(super::jidori_cluster_range(&jidori, 2), None);
        assert!(super::is_internal_jidori_boundary(&jidori, 0));
        assert!(!super::is_internal_jidori_boundary(&jidori, 1));
        assert!(!super::is_internal_jidori_boundary(&jidori, 3));
    }

    #[test]
    fn reduction_site_special_cases_are_a_closed_table() {
        use crate::style::ReductionTable;

        let cases = [
            (5, 5, ReductionTable::Table3, vec![(0, 4), (1, 4)]),
            (5, 5, ReductionTable::Table4, vec![(0, 2), (1, 2)]),
            (5, 5, ReductionTable::Table5, vec![]),
            (6, 5, ReductionTable::Table3, vec![(1, 4)]),
            (6, 5, ReductionTable::Table4, vec![(1, 2)]),
            (6, 5, ReductionTable::Table5, vec![]),
            (7, 5, ReductionTable::Table3, vec![(0, 5), (1, 4)]),
            (7, 5, ReductionTable::Table4, vec![(1, 2)]),
            (7, 5, ReductionTable::Table5, vec![(0, 3)]),
        ];
        for (before, after, table, expected) in cases {
            let mut sites = Vec::new();
            assert!(super::append_special_reduction_sites(
                table,
                before,
                after,
                [300, 200],
                [400, 800],
                9,
                &mut sites
            ));
            assert_eq!(
                sites
                    .iter()
                    .map(|site| {
                        let component = usize::from(site.weight == 800);
                        (component, site.stage)
                    })
                    .collect::<Vec<_>>(),
                expected,
                "{before}/{after} {table:?}"
            );
            assert!(sites.iter().all(|site| site.boundary == 9));
        }
        let mut sites = Vec::new();
        assert!(!super::append_special_reduction_sites(
            ReductionTable::Table3,
            19,
            19,
            [300, 200],
            [400, 800],
            9,
            &mut sites
        ));
        assert!(sites.is_empty());
    }

    #[test]
    fn reduction_rounding_and_site_guards_preserve_exact_units() {
        assert_eq!(super::quarter_rounded_up(0), 0);
        assert_eq!(super::quarter_rounded_up(1), 1);
        assert_eq!(super::quarter_rounded_up(4), 1);
        assert_eq!(super::quarter_rounded_up(5), 2);

        let mut sites = Vec::new();
        super::push_reduction_site(&mut sites, 2, 10, 0, 1, false);
        super::push_reduction_site(&mut sites, 2, 10, 1, 0, false);
        assert!(sites.is_empty());
        super::push_reduction_site(&mut sites, 2, 10, 1, 1, true);
        assert_eq!(
            sites,
            [super::ReductionSite {
                boundary: 2,
                weight: 10,
                capacity: 1,
                stage: 1,
                discrete: true,
            }]
        );

        let mut adjustments = [7, 11];
        super::distribute_reduction(0, &sites, Remainder::Leading, &mut adjustments);
        assert_eq!(adjustments, [7, 11]);
        super::distribute_adjustment(0, &[(0, 1, None)], Remainder::Leading, &mut adjustments);
        assert_eq!(adjustments, [7, 11]);
    }

    #[test]
    fn line_adjustment_stages_do_not_promote_bounded_stage_one_sites() {
        let paragraph = Paragraph::builder(
            mapped_text("A 〉", Frame::Proportional, |ordinal, cluster| {
                if ordinal == 1 {
                    Cluster::new(cluster.range(), 200)
                } else {
                    cluster
                }
            }),
            4_000,
        )
        .constructs([Construct::formula(0..2)])
        .build()
        .expect("valid isolated western-space expansion site");
        assert_eq!(
            super::boundary_expansion_site(&paragraph, &Style::default(), 1),
            super::ExpansionSite::Site {
                weight: 1_000,
                bounded: Some((300, 1)),
                residual: false,
            }
        );

        let mut adjustments = vec![99];
        super::prepare_line_adjustments(
            &paragraph,
            &Style::default(),
            0,
            3,
            1_000,
            &mut adjustments,
        );
        assert_eq!(adjustments, [0, 300, 0]);

        super::prepare_line_adjustments(&paragraph, &Style::default(), 0, 3, 0, &mut adjustments);
        assert_eq!(adjustments, [0, 0, 0]);

        let reducible = Paragraph::builder(
            mapped_text(" A", Frame::Proportional, |_, cluster| cluster),
            3_000,
        )
        .build()
        .expect("valid western-space reduction site");
        super::prepare_line_adjustments(
            &reducible,
            &Style::default(),
            0,
            2,
            -100,
            &mut adjustments,
        );
        assert_eq!(adjustments, [-100, 0]);
    }

    #[test]
    fn reductions_reject_empty_lines_and_distribute_exactly() {
        let closing = Paragraph::builder(text("〉"), 2_000)
            .build()
            .expect("valid closing bracket");
        assert!(super::reduction_sites(&closing, &Style::default(), 1, 1).is_empty());

        let sites = [
            super::ReductionSite {
                boundary: 0,
                weight: 1,
                capacity: 1,
                stage: 1,
                discrete: false,
            },
            super::ReductionSite {
                boundary: 1,
                weight: 3,
                capacity: 3,
                stage: 1,
                discrete: false,
            },
        ];
        let mut leading = [0, 0];
        super::distribute_reduction(3, &sites, Remainder::Leading, &mut leading);
        assert_eq!(leading, [-1, -2]);
        let mut trailing = [0, 0];
        super::distribute_reduction(3, &sites, Remainder::Trailing, &mut trailing);
        assert_eq!(trailing, [0, -3]);

        let expansion_sites = [(0, 1, Some(1)), (1, 3, Some(3))];
        let mut leading = [0, 0];
        super::distribute_adjustment(3, &expansion_sites, Remainder::Leading, &mut leading);
        assert_eq!(leading, [1, 2]);
        let mut trailing = [0, 0];
        super::distribute_adjustment(3, &expansion_sites, Remainder::Trailing, &mut trailing);
        assert_eq!(trailing, [0, 3]);
    }

    #[test]
    fn expansion_ceiling_rounds_each_policy_and_ignores_other_pairs() {
        use crate::style::JapaneseLatinExpansionCeiling;

        for (ceiling, expected) in [
            (JapaneseLatinExpansionCeiling::HalfEm, 4),
            (JapaneseLatinExpansionCeiling::ThirdEm, 3),
            (JapaneseLatinExpansionCeiling::Rigid, 2),
        ] {
            let style = Style::builder()
                .japanese_latin_expansion_ceiling(ceiling)
                .build()
                .expect("valid style");
            assert_eq!(super::expansion_ceiling(&style, 19, 27, 7, 99), expected);
            assert_eq!(super::expansion_ceiling(&style, 27, 19, 7, 99), expected);
            assert_eq!(super::expansion_ceiling(&style, 19, 19, 7, 99), 99);
        }
        let thirds = Style::builder()
            .japanese_latin_expansion_ceiling(JapaneseLatinExpansionCeiling::ThirdEm)
            .build()
            .expect("valid thirds style");
        assert_eq!(super::expansion_ceiling(&thirds, 19, 27, 2, 99), 1);
        assert_eq!(super::expansion_ceiling(&thirds, 19, 27, 6, 99), 2);
    }

    #[test]
    fn expansion_complex_identity_covers_every_kind_member_and_edge() {
        let script = Paragraph::builder(text("日本外"), 5_000)
            .constructs([Construct::script(0..6, text("注"))])
            .build()
            .expect("valid script complex");
        for ordinal in 0..2 {
            assert_eq!(
                super::expansion_complex_at(&script, ordinal),
                Some(super::ComplexIdentity {
                    kind: super::ComplexKind::Ornamented,
                    construct: 0,
                    member: 0,
                })
            );
        }
        assert_eq!(super::expansion_complex_at(&script, 2), None);
        assert_eq!(super::expansion_complex_at(&script, 3), None);

        let mono = ruby(
            RubyKind::Mono,
            0..6,
            "にほ",
            vec![RubyRun::new(0..3, 0..3), RubyRun::new(3..6, 3..6)],
        );
        let mono_paragraph = Paragraph::builder(text("日本外"), 5_000)
            .constructs([Construct::ruby(mono)])
            .build()
            .expect("valid mono ruby complex");
        for (ordinal, member) in [(0, 0), (1, 1)] {
            assert_eq!(
                super::expansion_complex_at(&mono_paragraph, ordinal),
                Some(super::ComplexIdentity {
                    kind: super::ComplexKind::SimpleRuby,
                    construct: 0,
                    member,
                })
            );
        }
        assert_eq!(super::expansion_complex_at(&mono_paragraph, 2), None);

        for (kind, expected, runs) in [
            (
                RubyKind::Group,
                super::ComplexKind::SimpleRuby,
                vec![RubyRun::new(0..6, 0..6)],
            ),
            (
                RubyKind::Jukugo,
                super::ComplexKind::JukugoRuby,
                vec![RubyRun::new(0..3, 0..3), RubyRun::new(3..6, 3..6)],
            ),
        ] {
            let ruby = ruby(kind, 0..6, "にほ", runs);
            let paragraph = Paragraph::builder(text("日本"), 4_000)
                .constructs([Construct::ruby(ruby)])
                .build()
                .expect("valid group-like ruby complex");
            for ordinal in 0..2 {
                assert_eq!(
                    super::expansion_complex_at(&paragraph, ordinal),
                    Some(super::ComplexIdentity {
                        kind: expected,
                        construct: 0,
                        member: 0,
                    })
                );
            }
        }

        let horizontal = Paragraph::builder(text("12"), 3_000)
            .constructs([Construct::tate_chu_yoko(0..2)])
            .build()
            .expect("valid horizontal tate-chu-yoko");
        assert_eq!(super::expansion_complex_at(&horizontal, 0), None);
        let vertical = Paragraph::builder(text("12日"), 4_000)
            .constructs([Construct::tate_chu_yoko(0..2)])
            .writing_mode(WritingMode::VerticalRl)
            .build()
            .expect("valid vertical tate-chu-yoko");
        for ordinal in 0..2 {
            assert_eq!(
                super::expansion_complex_at(&vertical, ordinal),
                Some(super::ComplexIdentity {
                    kind: super::ComplexKind::TateChuYoko,
                    construct: 0,
                    member: 0,
                })
            );
        }
        assert_eq!(super::expansion_complex_at(&vertical, 2), None);
    }

    #[test]
    fn boundary_expansion_sites_cover_spaces_constructs_classes_and_weights() {
        use crate::style::JapaneseLatinExpansionCeiling;

        let western = |advance: i32, following: &str| {
            let source = alloc::format!(" {following}");
            Paragraph::builder(
                mapped_text(&source, Frame::Proportional, |ordinal, cluster| {
                    if ordinal == 0 {
                        Cluster::new(cluster.range(), advance)
                    } else {
                        cluster
                    }
                }),
                5_000,
            )
            .build()
            .expect("valid western-space fixture")
        };
        assert_eq!(
            super::boundary_expansion_site(&western(200, "A"), &Style::default(), 0),
            super::ExpansionSite::Site {
                weight: 1_000,
                bounded: Some((300, 1)),
                residual: true,
            }
        );
        assert_eq!(
            super::boundary_expansion_site(&western(200, "〜"), &Style::default(), 0),
            super::ExpansionSite::Site {
                weight: 1_000,
                bounded: Some((300, 1)),
                residual: true,
            }
        );
        assert_eq!(
            super::boundary_expansion_site(&western(1_000, "A"), &Style::default(), 0),
            super::ExpansionSite::Site {
                weight: 1_000,
                bounded: None,
                residual: true,
            }
        );
        assert_eq!(
            super::boundary_expansion_site(&western(200, "〉"), &Style::default(), 0),
            super::ExpansionSite::Site {
                weight: 1_000,
                bounded: Some((300, 1)),
                residual: false,
            }
        );

        let plain_formula_pair = Paragraph::builder(
            mapped_text("日A", Frame::FullEm, |ordinal, cluster| {
                if ordinal == 1 {
                    cluster.with_frame(Frame::Proportional)
                } else {
                    cluster
                }
            }),
            4_000,
        )
        .build()
        .expect("valid Japanese/Latin pair");
        assert!(matches!(
            super::boundary_expansion_site(&plain_formula_pair, &Style::default(), 0),
            super::ExpansionSite::Site { .. }
        ));
        let internal_formula = Paragraph::builder(plain_formula_pair.text.clone(), 4_000)
            .constructs([Construct::formula(0..4)])
            .build()
            .expect("valid internal formula pair");
        assert_eq!(
            super::boundary_expansion_site(&internal_formula, &Style::default(), 0),
            super::ExpansionSite::None
        );

        for source in ["——", "〳〵"] {
            let paragraph = Paragraph::builder(text(source), 4_000)
                .build()
                .expect("valid inseparable pair");
            assert_eq!(
                super::boundary_expansion_site(&paragraph, &Style::default(), 0),
                super::ExpansionSite::None
            );
        }
        let different = Paragraph::builder(text("—…"), 4_000)
            .build()
            .expect("valid different inseparable pair");
        assert!(matches!(
            super::boundary_expansion_site(&different, &Style::default(), 0),
            super::ExpansionSite::Site { .. }
        ));

        let plain_quantity = Paragraph::builder(
            mapped_text("A%", Frame::Proportional, |_, cluster| cluster),
            4_000,
        )
        .build()
        .expect("valid ordinary 27/13 pair");
        assert!(matches!(
            super::boundary_expansion_site(&plain_quantity, &Style::default(), 0),
            super::ExpansionSite::Site { .. }
        ));
        let role_quantity = Paragraph::builder(
            mapped_text("A%", Frame::Proportional, |ordinal, cluster| {
                if ordinal == 0 {
                    cluster.with_role(ClusterRole::QuantitySymbol)
                } else {
                    cluster
                }
            }),
            4_000,
        )
        .build()
        .expect("valid role-qualified quantity pair");
        assert_eq!(
            super::boundary_expansion_site(&role_quantity, &Style::default(), 0),
            super::ExpansionSite::None
        );
        let digit_quantity = Paragraph::builder(
            mapped_text("1%", Frame::Proportional, |_, cluster| cluster),
            4_000,
        )
        .build()
        .expect("valid digit quantity pair");
        assert_eq!(
            super::boundary_expansion_site(&digit_quantity, &Style::default(), 0),
            super::ExpansionSite::None
        );
        let role_before_ideograph = Paragraph::builder(
            mapped_text("A日", Frame::Proportional, |ordinal, cluster| {
                if ordinal == 0 {
                    cluster.with_role(ClusterRole::QuantitySymbol)
                } else {
                    cluster
                }
            }),
            4_000,
        )
        .build()
        .expect("valid quantity role before an ideograph");
        assert!(matches!(
            super::boundary_expansion_site(&role_before_ideograph, &Style::default(), 0),
            super::ExpansionSite::Site { .. }
        ));

        let unequal_ideographs = Paragraph::builder(
            mapped_text("日本", Frame::FullEm, |ordinal, cluster| {
                if ordinal == 1 {
                    cluster.with_size(Size::square(2_000).expect("positive size"))
                } else {
                    cluster
                }
            }),
            5_000,
        )
        .build()
        .expect("valid unequal ideographs");
        assert_eq!(
            super::boundary_expansion_site(&unequal_ideographs, &Style::default(), 0),
            super::ExpansionSite::Site {
                weight: 1_000,
                bounded: Some((250, 3)),
                residual: false,
            }
        );
        let annotated = Paragraph::builder(unequal_ideographs.text.clone(), 5_000)
            .constructs([Construct::script(0..3, text("注"))])
            .build()
            .expect("valid unequal script boundary");
        assert_eq!(
            super::boundary_expansion_site(&annotated, &Style::default(), 0),
            super::ExpansionSite::Site {
                weight: 2_000,
                bounded: Some((500, 2)),
                residual: false,
            }
        );

        let rigid = Style::builder()
            .japanese_latin_expansion_ceiling(JapaneseLatinExpansionCeiling::Rigid)
            .build()
            .expect("valid rigid expansion style");
        assert_eq!(
            super::boundary_expansion_site(&plain_formula_pair, &rigid, 0),
            super::ExpansionSite::None
        );
    }

    #[test]
    fn construct_classification_covers_every_special_owner() {
        let annotation = text("注");
        let cases = [
            (
                Construct::ruby(ruby(
                    RubyKind::Group,
                    0..3,
                    "注",
                    vec![RubyRun::new(0..3, 0..3)],
                )),
                22,
            ),
            (
                Construct::ruby(ruby(
                    RubyKind::Jukugo,
                    0..3,
                    "注",
                    vec![RubyRun::new(0..3, 0..3)],
                )),
                23,
            ),
            (Construct::emphasis_dots(0..3, '・'), 21),
            (Construct::script(0..3, annotation.clone()), 21),
            (Construct::reference_mark(0..3, annotation), 20),
        ];
        for (construct, expected) in cases {
            let paragraph = Paragraph::builder(text("日"), 1_000)
                .constructs([construct])
                .build()
                .expect("valid owned cluster");
            assert_eq!(super::class_of_cluster(&paragraph, 0), expected);
        }

        let vertical = Paragraph::builder(text("12"), 1_000)
            .constructs([Construct::tate_chu_yoko(0..2)])
            .writing_mode(WritingMode::VerticalRl)
            .build()
            .expect("valid tate-chu-yoko");
        assert_eq!(super::class_of_cluster(&vertical, 0), 30);
        assert_eq!(super::class_of_cluster(&vertical, 1), 30);
    }

    #[test]
    fn contextual_punctuation_roles_are_solid_only_in_named_contexts() {
        let vertical = Paragraph::builder(
            mapped_text(
                "（）・、・",
                Frame::FullEm,
                |ordinal, cluster| match ordinal {
                    0 | 1 => cluster.with_role(ClusterRole::WarichuBracket),
                    2 => cluster.with_role(ClusterRole::DecimalPoint),
                    3 => cluster.with_role(ClusterRole::DigitGroupSeparator),
                    _ => cluster.with_role(ClusterRole::GroupedNumeral),
                },
            ),
            10_000,
        )
        .writing_mode(WritingMode::VerticalRl)
        .build()
        .expect("valid contextual punctuation");
        let characters = ['（', '）', '・', '、', '・'];
        for (ordinal, character) in characters.into_iter().enumerate() {
            assert!(super::contextual_punctuation_is_solid(
                &vertical,
                &vertical.text.clusters()[ordinal],
                character
            ));
        }

        let horizontal = Paragraph::builder(
            mapped_text("・、A", Frame::FullEm, |ordinal, cluster| match ordinal {
                0 => cluster.with_role(ClusterRole::DecimalPoint),
                1 => cluster.with_role(ClusterRole::DigitGroupSeparator),
                _ => cluster.with_role(ClusterRole::WarichuBracket),
            }),
            10_000,
        )
        .build()
        .expect("valid horizontal punctuation");
        assert!(!super::contextual_punctuation_is_solid(
            &horizontal,
            &horizontal.text.clusters()[0],
            '・'
        ));
        assert!(!super::contextual_punctuation_is_solid(
            &horizontal,
            &horizontal.text.clusters()[1],
            '、'
        ));
        assert!(!super::contextual_punctuation_is_solid(
            &horizontal,
            &horizontal.text.clusters()[2],
            'A'
        ));
    }

    #[test]
    fn western_space_and_rounding_predicates_have_exact_edges() {
        let proportional = Paragraph::builder(
            mapped_text(" A", Frame::Proportional, |_, cluster| cluster),
            2_000,
        )
        .build()
        .expect("valid proportional text");
        assert!(super::is_western_word_space(&proportional, 0));
        assert!(!super::is_western_word_space(&proportional, 1));
        assert!(!super::is_western_word_space(&proportional, 2));

        let full_em = Paragraph::builder(text(" "), 1_000)
            .build()
            .expect("valid full-em space");
        assert!(!super::is_western_word_space(&full_em, 0));
        let formula_role = Paragraph::builder(
            mapped_text(" ", Frame::Proportional, |_, cluster| {
                cluster.with_role(ClusterRole::Formula)
            }),
            1_000,
        )
        .build()
        .expect("valid role-tagged space");
        assert!(!super::is_western_word_space(&formula_role, 0));

        for (value, expected) in [(0, 0), (1, 1), (2, 1), (3, 2), (4, 2)] {
            assert_eq!(super::half_rounded_up(value), expected);
        }

        let inseparable = crate::generated::appendix_a::LISTINGS
            .iter()
            .find(|listing| listing.class == crate::spec::INSEPARABLE && listing.key[1] == 0)
            .and_then(|listing| char::from_u32(listing.key[0]))
            .expect("Appendix A has a one-character cl-08 member");
        assert!(super::is_inseparable_character(inseparable));
        assert!(!super::is_inseparable_character('A'));
    }

    #[test]
    fn line_end_and_sentence_medial_spacing_obey_style_edges() {
        use crate::style::{LineEndFullStopComma, LineEndPunctuation, SentenceMedialDividingMark};

        let closing = Paragraph::builder(text("）"), 2_000)
            .build()
            .expect("valid closing bracket");
        let solid = Style::builder()
            .line_end_punctuation(LineEndPunctuation::Solid)
            .build()
            .expect("valid style");
        assert_eq!(super::line_end_space_after(&closing, &solid, 0), 0);

        let comma = Paragraph::builder(text("、"), 2_000)
            .build()
            .expect("valid comma");
        let jis = Style::builder()
            .line_end_full_stop_comma(LineEndFullStopComma::Jis)
            .build()
            .expect("valid style");
        assert_eq!(super::line_end_space_after(&comma, &jis, 0), 0);

        let terminator = Paragraph::builder(
            mapped_text("!）", Frame::FullEm, |ordinal, cluster| {
                if ordinal == 0 {
                    cluster.with_role(ClusterRole::SentenceTerminator)
                } else {
                    cluster
                }
            }),
            3_000,
        )
        .build()
        .expect("valid terminator");
        assert_eq!(
            super::ordinary_boundary_space_after_with_style(&terminator, &Style::default(), 0),
            0
        );

        let medial = Paragraph::builder(
            mapped_text("!日", Frame::FullEm, |ordinal, cluster| {
                if ordinal == 0 {
                    cluster.with_role(ClusterRole::SentenceMedial)
                } else {
                    cluster
                }
            }),
            3_000,
        )
        .build()
        .expect("valid medial mark");
        let quarter = Style::builder()
            .sentence_medial_dividing_mark(SentenceMedialDividingMark::QuarterEm)
            .build()
            .expect("valid style");
        assert_eq!(
            super::ordinary_boundary_space_after_with_style(&medial, &quarter, 0),
            250
        );

        let unqualified = Paragraph::builder(text("!日"), 3_000)
            .build()
            .expect("valid unqualified dividing mark");
        assert_eq!(
            super::ordinary_boundary_space_after_with_style(&unqualified, &quarter, 0),
            0
        );
        let wrong_class = Paragraph::builder(
            mapped_text("日日", Frame::FullEm, |ordinal, cluster| {
                if ordinal == 0 {
                    cluster.with_role(ClusterRole::SentenceMedial)
                } else {
                    cluster
                }
            }),
            3_000,
        )
        .build()
        .expect("valid role on a non-dividing class");
        assert_eq!(
            super::ordinary_boundary_space_after_with_style(&wrong_class, &quarter, 0),
            0
        );
    }

    #[test]
    fn table_one_blank_lookup_matches_every_generated_cell() {
        for before in (0_u8..=30).filter(|class| *class != 17 && *class != 18) {
            for after in (0_u8..=30).filter(|class| *class != 17 && *class != 18) {
                let cell = crate::generated::table1::cell(before, after)
                    .expect("every transcribed table-one coordinate is indexed");
                assert_eq!(
                    super::table_one_cell_is_blank(before, after),
                    cell.terms.is_empty(),
                    "cl-{before:02}/cl-{after:02}"
                );
            }
        }
        assert!(super::table_one_cell_is_blank(0, 0));
    }

    #[test]
    fn formula_boundaries_distinguish_internal_tokens_and_outer_neighbors() {
        let internal = Paragraph::builder(
            mapped_text("A=B", Frame::Proportional, |_, cluster| cluster),
            5_000,
        )
        .constructs([Construct::formula(0..3)])
        .build()
        .expect("valid whole formula");
        assert_eq!(super::formula_boundary_space_after(&internal, 0), Some(250));
        assert_eq!(super::formula_boundary_space_after(&internal, 1), Some(250));

        let adjacent_symbols = Paragraph::builder(
            mapped_text("==", Frame::Proportional, |_, cluster| cluster),
            3_000,
        )
        .constructs([Construct::formula(0..2)])
        .build()
        .expect("valid adjacent formula symbols");
        assert_eq!(
            super::formula_boundary_space_after(&adjacent_symbols, 0),
            Some(0)
        );

        let left_partial = Paragraph::builder(
            mapped_text("A=日", Frame::Proportional, |_, cluster| cluster),
            5_000,
        )
        .constructs([Construct::formula(0..2)])
        .build()
        .expect("valid formula touching only the paragraph start");
        assert_eq!(
            super::formula_boundary_space_after(&left_partial, 0),
            Some(0)
        );
        let right_partial = Paragraph::builder(
            mapped_text("日=A", Frame::Proportional, |_, cluster| cluster),
            5_000,
        )
        .constructs([Construct::formula(3..5)])
        .build()
        .expect("valid formula touching only the paragraph end");
        assert_eq!(
            super::formula_boundary_space_after(&right_partial, 1),
            Some(0)
        );

        let adjacent_formulas = Paragraph::builder(
            mapped_text("AB", Frame::Proportional, |_, cluster| cluster),
            3_000,
        )
        .constructs([Construct::formula(0..1), Construct::formula(1..2)])
        .build()
        .expect("valid adjacent formula constructs");
        assert_eq!(
            super::formula_boundary_space_after(&adjacent_formulas, 0),
            Some(0)
        );

        let outer = Paragraph::builder(
            mapped_text("日A", Frame::FullEm, |ordinal, cluster| {
                if ordinal == 1 {
                    cluster
                        .with_frame(Frame::Proportional)
                        .with_role(ClusterRole::Formula)
                } else {
                    cluster
                }
            }),
            4_000,
        )
        .constructs([Construct::formula(3..4)])
        .build()
        .expect("valid outer formula");
        assert_eq!(super::formula_boundary_space_after(&outer, 0), Some(250));

        let trailing_outer = Paragraph::builder(
            mapped_text("A日", Frame::FullEm, |ordinal, cluster| {
                if ordinal == 0 {
                    cluster
                        .with_frame(Frame::Proportional)
                        .with_role(ClusterRole::Formula)
                } else {
                    cluster
                }
            }),
            4_000,
        )
        .constructs([Construct::formula(0..1)])
        .build()
        .expect("valid formula before a Japanese neighbor");
        assert_eq!(
            super::formula_boundary_space_after(&trailing_outer, 0),
            Some(250)
        );

        let plain = Paragraph::builder(text("日本"), 3_000)
            .build()
            .expect("valid plain text");
        assert_eq!(super::formula_boundary_space_after(&plain, 0), None);
    }

    #[test]
    fn japanese_formula_neighbor_exclusions_are_independent() {
        let source = "日 （）、。・+";
        let paragraph = Paragraph::builder(text(source), 20_000)
            .build()
            .expect("valid neighbor set");
        assert!(super::is_japanese_formula_neighbor(
            &paragraph,
            &paragraph.text.clusters()[0]
        ));
        for cluster in &paragraph.text.clusters()[1..] {
            assert!(!super::is_japanese_formula_neighbor(&paragraph, cluster));
        }

        let proportional =
            Paragraph::builder(mapped_text("日", Frame::Proportional, |_, c| c), 2_000)
                .build()
                .expect("valid proportional neighbor");
        assert!(!super::is_japanese_formula_neighbor(
            &proportional,
            &proportional.text.clusters()[0]
        ));
    }

    #[test]
    fn formula_outer_spacing_requires_a_japanese_neighbor_and_eligible_endpoint() {
        let eligible = Paragraph::builder(
            mapped_text("日A", Frame::FullEm, |ordinal, cluster| {
                if ordinal == 1 {
                    cluster.with_frame(Frame::Proportional)
                } else {
                    cluster
                }
            }),
            4_000,
        )
        .build()
        .expect("valid eligible formula endpoint");
        assert!(super::formula_endpoint_needs_quarter(
            &eligible,
            &eligible.text.clusters()[1]
        ));
        assert_eq!(
            super::formula_outer_boundary_space(
                &eligible,
                &eligible.text.clusters()[0],
                &eligible.text.clusters()[1]
            ),
            250
        );

        let rigid = Paragraph::builder(text("日A"), 4_000)
            .build()
            .expect("valid rigid formula endpoint");
        assert!(!super::formula_endpoint_needs_quarter(
            &rigid,
            &rigid.text.clusters()[1]
        ));
        assert_eq!(
            super::formula_outer_boundary_space(
                &rigid,
                &rigid.text.clusters()[0],
                &rigid.text.clusters()[1]
            ),
            0
        );

        let non_japanese = Paragraph::builder(
            mapped_text(" A", Frame::FullEm, |ordinal, cluster| {
                if ordinal == 1 {
                    cluster.with_frame(Frame::Proportional)
                } else {
                    cluster
                }
            }),
            4_000,
        )
        .build()
        .expect("valid non-Japanese formula neighbor");
        assert_eq!(
            super::formula_outer_boundary_space(
                &non_japanese,
                &non_japanese.text.clusters()[0],
                &non_japanese.text.clusters()[1]
            ),
            0
        );

        let math = Paragraph::builder(
            mapped_text("+", Frame::Proportional, |_, cluster| cluster),
            2_000,
        )
        .build()
        .expect("valid mathematical endpoint");
        assert!(!super::formula_endpoint_needs_quarter(
            &math,
            &math.text.clusters()[0]
        ));

        let grouped = Paragraph::builder(
            mapped_text("A", Frame::FullEm, |_, cluster| {
                cluster.with_role(ClusterRole::GroupedNumeral)
            }),
            2_000,
        )
        .build()
        .expect("valid grouped-numeral endpoint");
        assert!(super::formula_endpoint_needs_quarter(
            &grouped,
            &grouped.text.clusters()[0]
        ));
    }

    #[test]
    fn jidori_plan_distributes_surplus_only_inside_a_complete_range() {
        let paragraph = Paragraph::builder(text("日本語"), 5_000)
            .constructs([Construct::jidori(0..9, 4)])
            .build()
            .expect("valid jidori paragraph");
        let plan = super::jidori_plan(&paragraph, &Style::default(), 0..3, 4, 0, 3, 0);
        assert_eq!(plan.range, 0..3);
        assert_eq!(plan.extra_after, [500, 500, 0]);
        assert_eq!(plan.extra_after(0), 500);
        assert_eq!(plan.extra_after(1), 500);
        assert_eq!(plan.extra_after(2), 0);
        assert_eq!(plan.extra_after(3), 0);
        assert_eq!(
            super::jidori_extra_after(&paragraph, &Style::default(), 0, 1, 3, 0),
            0
        );
        assert_eq!(
            super::jidori_extra_after(&paragraph, &Style::default(), 0, 0, 2, 0),
            0
        );

        let uneven = Paragraph::builder(
            mapped_text("日本語", Frame::FullEm, |ordinal, cluster| {
                if ordinal == 0 {
                    Cluster::new(cluster.range(), 999)
                } else {
                    cluster
                }
            }),
            5_000,
        )
        .build()
        .expect("valid uneven jidori members");
        let trailing = Style::builder()
            .remainder(Remainder::Trailing)
            .build()
            .expect("valid trailing-remainder style");
        assert_eq!(
            super::jidori_plan(&uneven, &trailing, 0..3, 4, 0, 3, 0).extra_after,
            [500, 501, 0]
        );

        let closing = Paragraph::builder(text("〉日"), 4_000)
            .build()
            .expect("valid jidori boundary spacing fixture");
        assert_eq!(super::boundary_space_after(&closing, 0), 500);
        assert_eq!(
            super::jidori_plan(&closing, &Style::default(), 0..2, 3, 0, 2, 0).extra_after,
            [500, 0]
        );
    }

    #[test]
    fn ruby_span_visitation_switches_between_runs_groups_and_phonetic_layout() {
        use crate::style::JukugoRubyLayout;

        let per_run = ruby(
            RubyKind::Jukugo,
            0..6,
            "にほんご",
            vec![RubyRun::new(0..3, 0..6), RubyRun::new(3..6, 6..12)],
        );
        let paragraph = Paragraph::builder(text("日本"), 4_000)
            .constructs([Construct::ruby(per_run)])
            .build()
            .expect("valid jukugo");
        let mut spans = Vec::new();
        super::visit_ruby_spans(&paragraph, &Style::default(), |_, base, annotation| {
            spans.push((base, annotation));
        });
        assert_eq!(spans, [(0..3, 0..6), (3..6, 6..12)]);

        let grouped = ruby(
            RubyKind::Jukugo,
            0..6,
            "にほんごく",
            vec![RubyRun::new(0..3, 0..9), RubyRun::new(3..6, 9..15)],
        );
        let paragraph = Paragraph::builder(text("日本"), 4_000)
            .constructs([Construct::ruby(grouped)])
            .build()
            .expect("valid grouped jukugo");
        spans.clear();
        super::visit_ruby_spans(&paragraph, &Style::default(), |_, base, annotation| {
            spans.push((base, annotation));
        });
        assert_eq!(spans, [(0..6, 0..15)]);

        let phonetic = Style::builder()
            .jukugo_ruby_layout(JukugoRubyLayout::Phonetic)
            .build()
            .expect("valid style");
        spans.clear();
        super::visit_ruby_spans(&paragraph, &phonetic, |_, base, annotation| {
            spans.push((base, annotation));
        });
        assert!(spans.is_empty());
    }

    #[test]
    fn group_ruby_distribution_and_overhang_have_exact_integer_geometry() {
        use crate::style::{GroupRubyDistribution, JukugoRubyLayout};

        let group = ruby(
            RubyKind::Group,
            0..6,
            "にほんご",
            vec![RubyRun::new(0..6, 0..12)],
        );
        let paragraph = Paragraph::builder(text("日本"), 8_000)
            .constructs([Construct::ruby(group.clone())])
            .build()
            .expect("valid group ruby");
        let jis =
            super::group_ruby_base_plan(&paragraph, &Style::default(), &group, &(0..6), &(0..12))
                .expect("annotation is wider than the base");
        assert_eq!(jis.base, 0..2);
        assert_eq!(jis.leading, 500);
        assert_eq!(jis.trailing, 500);
        assert_eq!(jis.gap_after(0), 1_000);
        assert_eq!(jis.gap_after(1), 0);

        let flush_style = Style::builder()
            .group_ruby_distribution(GroupRubyDistribution::Flush)
            .build()
            .expect("valid style");
        let flush =
            super::group_ruby_base_plan(&paragraph, &flush_style, &group, &(0..6), &(0..12))
                .expect("flush distribution");
        assert_eq!(flush.leading, 0);
        assert_eq!(flush.trailing, 0);
        assert_eq!(flush.gap_after(0), 2_000);

        let mono = ruby(RubyKind::Mono, 0..3, "にほ", vec![RubyRun::new(0..3, 0..6)]);
        let mono_paragraph = Paragraph::builder(text("日"), 4_000)
            .constructs([Construct::ruby(mono.clone())])
            .build()
            .expect("valid mono ruby");
        assert!(
            super::group_ruby_base_plan(
                &mono_paragraph,
                &Style::default(),
                &mono,
                &(0..3),
                &(0..6)
            )
            .is_none()
        );
        let overhang =
            super::ruby_span_overhang(&mono_paragraph, &Style::default(), &mono, 0..3, 0..6, 0, 1)
                .expect("mono annotation overhangs");
        assert_eq!(overhang.base, 0..1);
        assert_eq!(overhang.leading, 500);
        assert_eq!(overhang.trailing, 500);
        assert_eq!(overhang.ruby_em, 1_000);
        assert!(
            super::ruby_span_overhang(&mono_paragraph, &Style::default(), &mono, 0..3, 0..6, 1, 2)
                .is_none()
        );

        let jukugo = ruby(
            RubyKind::Jukugo,
            0..6,
            "にほんご",
            vec![RubyRun::new(0..3, 0..6), RubyRun::new(3..6, 6..12)],
        );
        let jukugo_paragraph = Paragraph::builder(text("日本"), 8_000)
            .constructs([Construct::ruby(jukugo.clone())])
            .build()
            .expect("valid jukugo");
        assert!(
            super::group_ruby_base_plan(
                &jukugo_paragraph,
                &Style::default(),
                &jukugo,
                &(0..6),
                &(0..12)
            )
            .is_some()
        );
        assert!(
            super::group_ruby_base_plan(
                &jukugo_paragraph,
                &Style::default(),
                &jukugo,
                &(0..3),
                &(0..6)
            )
            .is_none()
        );
        let phonetic = Style::builder()
            .jukugo_ruby_layout(JukugoRubyLayout::Phonetic)
            .build()
            .expect("valid style");
        assert!(
            super::group_ruby_base_plan(&jukugo_paragraph, &phonetic, &jukugo, &(0..6), &(0..12))
                .is_none()
        );

        let spaced_base = Paragraph::builder(text("〉日"), 4_000)
            .build()
            .expect("valid ruby base spacing fixture");
        assert_eq!(
            super::ruby_base_width(&spaced_base, &Style::default(), &(0..2)),
            2_500
        );
        assert_eq!(
            super::ruby_base_width(&spaced_base, &Style::default(), &(0..1)),
            1_000
        );
    }

    #[test]
    fn ruby_boundary_separation_rejects_partial_groups_and_honors_line_end() {
        let group = ruby(
            RubyKind::Group,
            0..6,
            "にほんご",
            vec![RubyRun::new(0..6, 0..12)],
        );
        let group_paragraph = Paragraph::builder(text("日本"), 8_000)
            .constructs([Construct::ruby(group)])
            .build()
            .expect("valid group ruby");
        assert_eq!(
            super::ruby_boundary_separation_after(&group_paragraph, &Style::default(), 0, 1, 2, 0),
            0
        );
        assert_eq!(
            super::ruby_boundary_separation_after(&group_paragraph, &Style::default(), 0, 0, 1, 0),
            0
        );

        let mono = ruby(RubyKind::Mono, 3..4, "にほ", vec![RubyRun::new(3..4, 0..6)]);
        let mono_paragraph = Paragraph::builder(text("外A〉"), 8_000)
            .constructs([Construct::ruby(mono)])
            .build()
            .expect("valid middle mono ruby");
        assert_eq!(
            super::ruby_boundary_separation_after(&mono_paragraph, &Style::default(), 1, 0, 3, 0),
            0
        );
        assert_eq!(
            super::ruby_boundary_separation_after(&mono_paragraph, &Style::default(), 1, 0, 2, 0),
            500
        );
    }

    #[test]
    fn ruby_neighbor_allowance_distinguishes_each_punctuation_side() {
        let leading = Paragraph::builder(text("（日A日・日あ"), 20_000)
            .build()
            .expect("valid neighbor text");
        assert_eq!(
            super::ruby_neighbor_overhang_allowance(
                &leading,
                &Style::default(),
                0,
                super::RubySide::Leading,
                333
            ),
            333
        );
        assert_eq!(
            super::ruby_neighbor_overhang_allowance(
                &leading,
                &Style::default(),
                2,
                super::RubySide::Leading,
                333
            ),
            0
        );
        assert_eq!(
            super::ruby_neighbor_overhang_allowance(
                &leading,
                &Style::default(),
                4,
                super::RubySide::Leading,
                333
            ),
            333
        );
        assert_eq!(
            super::ruby_neighbor_overhang_allowance(
                &leading,
                &Style::default(),
                6,
                super::RubySide::Leading,
                333
            ),
            333
        );

        let trailing = Paragraph::builder(text("A）A。A、"), 20_000)
            .build()
            .expect("valid trailing neighbors");
        for ordinal in [1, 3, 5] {
            assert_eq!(
                super::ruby_neighbor_overhang_allowance(
                    &trailing,
                    &Style::default(),
                    ordinal,
                    super::RubySide::Trailing,
                    333
                ),
                333
            );
        }
        assert_eq!(
            super::ruby_neighbor_overhang_allowance(
                &trailing,
                &Style::default(),
                0,
                super::RubySide::Trailing,
                333
            ),
            0
        );

        let leading_trailing_marks = Paragraph::builder(text("〉日。日、日"), 20_000)
            .build()
            .expect("valid leading closing marks");
        for ordinal in [0, 2, 4] {
            assert_eq!(
                super::ruby_neighbor_overhang_allowance(
                    &leading_trailing_marks,
                    &Style::default(),
                    ordinal,
                    super::RubySide::Leading,
                    333
                ),
                333
            );
        }

        let trailing_opening = Paragraph::builder(text("日〈"), 4_000)
            .build()
            .expect("valid trailing opening bracket");
        assert_eq!(
            super::ruby_neighbor_overhang_allowance(
                &trailing_opening,
                &Style::default(),
                1,
                super::RubySide::Trailing,
                333
            ),
            333
        );
    }

    #[test]
    fn phonetic_expansion_apportionment_excludes_short_runs_and_respects_ties() {
        let runs = [
            super::PhoneticJukugoRun {
                base: 0..1,
                annotation: 0..1,
                base_start: 0,
                base_end: 1,
                annotation_start: 0,
                annotation_width: 2,
                ruby_em: 1,
                annotation_count: 3,
            },
            super::PhoneticJukugoRun {
                base: 1..2,
                annotation: 1..2,
                base_start: 1,
                base_end: 2,
                annotation_start: 0,
                annotation_width: 100,
                ruby_em: 1,
                annotation_count: 2,
            },
            super::PhoneticJukugoRun {
                base: 2..3,
                annotation: 2..3,
                base_start: 2,
                base_end: 3,
                annotation_start: 0,
                annotation_width: 1,
                ruby_em: 1,
                annotation_count: 4,
            },
        ];
        assert_eq!(
            super::apportion_phonetic_expansion(&runs, 5, Remainder::Leading),
            [4, 0, 1]
        );
        assert_eq!(
            super::apportion_phonetic_expansion(&runs, 5, Remainder::Trailing),
            [3, 0, 2]
        );

        let reordered = [runs[1].clone(), runs[0].clone(), runs[2].clone()];
        assert_eq!(
            super::apportion_phonetic_expansion(&reordered, 5, Remainder::Leading),
            [0, 4, 1]
        );
    }

    #[test]
    fn phonetic_plan_builder_keeps_external_and_internal_boundaries_distinct() {
        let spaced = Paragraph::builder(text("〉日"), 4_000)
            .build()
            .expect("valid phonetic base spacing fixture");
        let whole = [super::PhoneticJukugoRun {
            base: 0..2,
            annotation: 0..1,
            base_start: 0,
            base_end: 0,
            annotation_start: 0,
            annotation_width: 2_000,
            ruby_em: 1_000,
            annotation_count: 3,
        }];
        let plan = super::build_phonetic_jukugo_plan(
            &spaced,
            &Style::default(),
            &whole,
            0,
            super::PhoneticEdges {
                line: super::LineContext {
                    start: 0,
                    end: 2,
                    index: 0,
                },
                leading_allowance: 0,
                trailing_allowance: 0,
            },
        )
        .expect("the annotation fits the spaced base");
        assert_eq!((plan.runs[0].base_start, plan.runs[0].base_end), (0, 2_500));

        let inset = Paragraph::builder(text("AAA"), 5_000)
            .build()
            .expect("valid inset phonetic run");
        let raw = [super::PhoneticJukugoRun {
            base: 1..2,
            annotation: 0..1,
            base_start: 0,
            base_end: 0,
            annotation_start: 0,
            annotation_width: 1_000,
            ruby_em: 100,
            annotation_count: 3,
        }];
        let plan = super::build_phonetic_jukugo_plan(
            &inset,
            &Style::default(),
            &raw,
            2,
            super::PhoneticEdges {
                line: super::LineContext {
                    start: 0,
                    end: 3,
                    index: 0,
                },
                leading_allowance: 0,
                trailing_allowance: 0,
            },
        )
        .expect("the inset annotation fits after symmetric expansion");
        assert_eq!((plan.runs[0].base_start, plan.runs[0].base_end), (1, 1_001));
        assert_eq!(plan.gap_after(0), 1);
        assert_eq!(plan.gap_after(1), 1);
    }

    #[test]
    fn phonetic_jukugo_plan_is_present_only_for_the_requested_complete_runs() {
        use crate::style::JukugoRubyLayout;

        let wide_ruby = ruby(
            RubyKind::Jukugo,
            0..6,
            "にほんごくご",
            vec![RubyRun::new(0..3, 0..9), RubyRun::new(3..6, 9..18)],
        );
        let paragraph = Paragraph::builder(text("日本"), 10_000)
            .constructs([Construct::ruby(wide_ruby.clone())])
            .build()
            .expect("valid phonetic jukugo");
        assert!(
            super::phonetic_jukugo_plan(&paragraph, &Style::default(), &wide_ruby, 0, 2, 0)
                .is_none()
        );
        let phonetic = Style::builder()
            .jukugo_ruby_layout(JukugoRubyLayout::Phonetic)
            .build()
            .expect("valid style");
        let plan = super::phonetic_jukugo_plan(&paragraph, &phonetic, &wide_ruby, 0, 2, 0)
            .expect("phonetic runs can be placed");
        assert_eq!(plan.runs.len(), 2);
        assert!(plan.runs[0].annotation_start < plan.runs[1].annotation_start);
        assert!(
            plan.runs[0]
                .annotation_start
                .saturating_add(plan.runs[0].annotation_width)
                <= plan.runs[1].annotation_start
        );
        assert_eq!(plan.gap_after(9), 0);

        let compact_ruby = ruby(
            RubyKind::Jukugo,
            0..6,
            "にほ",
            vec![RubyRun::new(0..3, 0..3), RubyRun::new(3..6, 3..6)],
        );
        let compact = Paragraph::builder(text("日本"), 6_000)
            .constructs([Construct::ruby(compact_ruby.clone())])
            .build()
            .expect("valid compact phonetic jukugo");
        let right = super::phonetic_jukugo_plan(&compact, &phonetic, &compact_ruby, 1, 2, 1)
            .expect("right run is complete");
        assert_eq!(right.runs.len(), 1);
        assert_eq!(right.runs[0].base, 1..2);
        let left = super::phonetic_jukugo_plan(&compact, &phonetic, &compact_ruby, 0, 1, 0)
            .expect("left run is complete");
        assert_eq!(left.runs.len(), 1);
        assert_eq!(left.runs[0].base, 0..1);

        let indented_ruby = ruby(
            RubyKind::Jukugo,
            0..3,
            "にほん",
            vec![RubyRun::new(0..3, 0..9)],
        );
        let indented = Paragraph::builder(text("日"), 6_000)
            .constructs([Construct::ruby(indented_ruby.clone())])
            .first_line_indent(500)
            .build()
            .expect("valid indented phonetic ruby");
        let permitted = Style::builder()
            .jukugo_ruby_layout(JukugoRubyLayout::Phonetic)
            .ruby_overhang_indent(crate::style::RubyOverhangIndent::Permitted)
            .build()
            .expect("valid indent-overhang style");
        let prohibited = Style::builder()
            .jukugo_ruby_layout(JukugoRubyLayout::Phonetic)
            .ruby_overhang_indent(crate::style::RubyOverhangIndent::Prohibited)
            .build()
            .expect("valid no-indent-overhang style");
        assert_eq!(
            super::phonetic_jukugo_plan(&indented, &permitted, &indented_ruby, 0, 1, 0)
                .expect("first-line indent permits overhang")
                .leading_gap,
            750
        );
        assert_eq!(
            super::phonetic_jukugo_plan(&indented, &prohibited, &indented_ruby, 0, 1, 0)
                .expect("prohibited overhang is absorbed as expansion")
                .leading_gap,
            1_000
        );
    }

    #[test]
    fn proportional_shares_are_exact_bounded_and_directional() {
        assert_eq!(
            super::proportional_shares(5, &[1, 1, 1], Remainder::Leading),
            [2, 2, 1]
        );
        assert_eq!(
            super::proportional_shares(5, &[1, 1, 1], Remainder::Trailing),
            [1, 2, 2]
        );
        assert_eq!(
            super::proportional_shares(4, &[1, -1, 2], Remainder::Leading),
            [2, 0, 2]
        );
        assert_eq!(
            super::proportional_shares(4, &[1, -1, 2], Remainder::Trailing),
            [1, 0, 3]
        );
        assert_eq!(
            super::proportional_shares(0, &[1, 2], Remainder::Leading),
            [0, 0]
        );
        assert_eq!(
            super::proportional_shares(5, &[0, -1], Remainder::Leading),
            [0, 0]
        );
        assert!(super::proportional_shares(5, &[], Remainder::Leading).is_empty());
        assert_eq!(
            super::proportional_shares(i64::MAX, &[i32::MAX, i32::MAX], Remainder::Leading),
            [i32::MAX, i32::MAX]
        );
    }

    #[test]
    fn placement_bounds_annotation_counts_and_line_fit_use_closed_ranges() {
        let fixture_line = line(
            0..3,
            vec![
                placement(0, 0..1, 10, 5, crate::CoordinateTransform::Identity),
                placement(1, 1..2, 20, 7, crate::CoordinateTransform::Identity),
                placement(2, 2..3, 30, 100, crate::CoordinateTransform::TateChuYoko),
            ],
        );
        assert_eq!(
            super::bounds_for_range(&fixture_line, &(0..2)),
            Some((10, 27))
        );
        assert_eq!(
            super::bounds_for_range(&fixture_line, &(1..3)),
            Some((20, 39))
        );
        assert_eq!(super::bounds_for_range(&fixture_line, &(4..5)), None);
        assert_eq!(super::placement_inline_end(&fixture_line.clusters[0]), 15);
        assert_eq!(super::placement_inline_end(&fixture_line.clusters[2]), 39);

        let annotation = text("日本語");
        assert_eq!(super::annotation_cluster_count(&annotation, &(0..9)), 3);
        assert_eq!(super::annotation_cluster_count(&annotation, &(0..6)), 2);
        assert_eq!(super::annotation_cluster_count(&annotation, &(1..8)), 1);
        assert_eq!(super::annotation_cluster_count(&annotation, &(9..9)), 0);
        assert!(super::range_fits_line(&(0..3), &fixture_line));
        assert!(super::range_fits_line(&(1..2), &fixture_line));
        assert!(!super::range_fits_line(&(0..4), &fixture_line));
        assert!(!super::range_fits_line(&(0..3), &line(1..3, Vec::new())));
    }

    #[test]
    fn badness_widow_penalty_and_diagnostic_have_exact_thresholds() {
        use crate::style::AdjustmentPreference;

        assert_eq!(
            super::line_badness(-3, false, AdjustmentPreference::LeastAdjustment),
            10_009_000
        );
        assert_eq!(
            super::line_badness(10, true, AdjustmentPreference::LeastAdjustment),
            1
        );
        assert_eq!(
            super::line_badness(10, false, AdjustmentPreference::LeastAdjustment),
            100
        );
        assert_eq!(
            super::line_badness(10, false, AdjustmentPreference::EvenTexture),
            200
        );

        let paragraph = Paragraph::builder(text("日本"), 3_000)
            .widow(Widow::MinimumClusters(2))
            .build()
            .expect("valid widow paragraph");
        assert_eq!(super::widow_penalty(&paragraph, 0, 3), 1_000_000_000);
        assert_eq!(super::widow_penalty(&paragraph, 0, 6), 0);
        let mut layout = crate::Layout::default();
        layout.lines.push(line(
            0..3,
            vec![placement(
                0,
                0..3,
                0,
                1_000,
                crate::CoordinateTransform::Identity,
            )],
        ));
        super::add_widow_diagnostic(&paragraph, &mut layout);
        assert_eq!(layout.diagnostics.len(), 1);
        assert_eq!(layout.diagnostics[0].code(), "layout.widow");
        assert_eq!(layout.diagnostics[0].range(), Some(0..3));

        layout.lines[0].clusters.push(placement(
            1,
            3..6,
            1_000,
            1_000,
            crate::CoordinateTransform::Identity,
        ));
        layout.diagnostics.clear();
        super::add_widow_diagnostic(&paragraph, &mut layout);
        assert!(layout.diagnostics.is_empty());
    }

    #[test]
    fn furawake_lanes_and_placements_keep_outer_boundaries_outside() {
        let plain = Paragraph::builder(
            mapped_text("日A", Frame::FullEm, |ordinal, cluster| {
                if ordinal == 1 {
                    cluster.with_frame(Frame::Proportional)
                } else {
                    cluster
                }
            }),
            4_000,
        )
        .build()
        .expect("valid punctuation paragraph");
        assert_eq!(super::construct_lane_width(&plain, &(0..1)), 1_000);
        assert_eq!(super::construct_lane_width(&plain, &(0..2)), 2_250);

        let source = "日A日A";
        let paragraph = Paragraph::builder(
            mapped_text(source, Frame::FullEm, |ordinal, cluster| {
                if ordinal % 2 == 1 {
                    cluster.with_frame(Frame::Proportional)
                } else {
                    cluster
                }
            }),
            8_000,
        )
        .breaks([Break::allowed(3)])
        .constructs([Construct::furawake(0..source.len(), 2, 100)])
        .build()
        .expect("valid furawake paragraph");
        let segment = super::furawake_segment(&paragraph, 0..4, 2, 100, 4);
        assert_eq!(segment.range, 0..4);
        assert_eq!(segment.lanes, [0..1, 1..4]);
        assert_eq!(segment.block_extents, [1_000, 1_000]);
        assert_eq!(segment.block_extent, 2_100);
        assert_eq!(segment.advance, 3_500);
        let mut placed = Vec::new();
        super::place_furawake_segment(&paragraph, &segment, 100, 0, &mut placed);
        assert_eq!(
            placed
                .iter()
                .map(|item| (item.inline, item.block, item.advance))
                .collect::<Vec<_>>(),
            [
                (100, -550, 1_000),
                (100, 550, 1_250),
                (1_350, 550, 1_250),
                (2_600, 550, 1_000)
            ]
        );

        let outer = Paragraph::builder(
            mapped_text("日A日", Frame::FullEm, |ordinal, cluster| {
                if ordinal == 1 {
                    cluster.with_frame(Frame::Proportional)
                } else {
                    cluster
                }
            }),
            6_000,
        )
        .build()
        .expect("valid outer-boundary fixture");
        assert_eq!(
            super::furawake_segment(&outer, 0..2, 1, 0, 2).advance,
            2_250
        );
        assert_eq!(
            super::furawake_segment(&outer, 0..2, 1, 0, 3).advance,
            2_500
        );

        let offset_source = "X日A日";
        let offset = Paragraph::builder(
            mapped_text(offset_source, Frame::FullEm, |ordinal, cluster| {
                if ordinal == 2 {
                    cluster.with_frame(Frame::Proportional)
                } else {
                    cluster
                }
            }),
            8_000,
        )
        .breaks([Break::allowed(1), Break::allowed(4)])
        .constructs([Construct::furawake(1..offset_source.len(), 2, 0)])
        .build()
        .expect("valid nonzero-start furawake");
        let offset_segment = super::furawake_segment(&offset, 1..4, 2, 0, 4);
        assert_eq!(offset_segment.lanes, [1..2, 2..4]);
        assert_eq!(
            super::effective_cluster_body_advance(&offset, 1),
            offset_segment.advance
        );
        assert_eq!(super::effective_cluster_body_advance(&offset, 2), 0);
        assert_eq!(super::effective_cluster_body_advance(&offset, 3), 0);
    }

    #[test]
    fn warichu_split_trim_geometry_and_break_penalty_are_exact() {
        let proportional = mapped_text("( abc)", Frame::Proportional, |ordinal, cluster| {
            if ordinal == 0 || ordinal == 5 {
                cluster.with_role(ClusterRole::WarichuBracket)
            } else {
                cluster
            }
        });
        let declared = Paragraph::builder(proportional.clone(), 10_000)
            .breaks([Break::allowed(3)])
            .constructs([Construct::warichu(0..6)])
            .build()
            .expect("valid declared warichu split");
        let segment = super::warichu_segment(&declared, 0..6, 0, 6);
        assert_eq!(segment.range, 0..6);
        assert_eq!(segment.leading_bracket, Some(0));
        assert_eq!(segment.first_lane, 1..3);
        assert_eq!(segment.second_lane, 3..5);
        assert_eq!(segment.trailing_bracket, Some(5));
        assert_eq!((segment.first_width, segment.second_width), (1_000, 2_000));
        assert_eq!(segment.advance, 4_000);
        assert_eq!(super::warichu_member_advance(&declared, 1, &(1..3)), 0);

        let automatic = Paragraph::builder(proportional, 10_000)
            .constructs([Construct::warichu(0..6)])
            .build()
            .expect("valid automatic warichu split");
        assert_eq!(super::choose_warichu_split(&automatic, 1..5), 4);

        let tied_text = mapped_text("abc", Frame::Proportional, |ordinal, cluster| {
            if ordinal == 1 {
                Cluster::new(cluster.range(), 0)
            } else {
                cluster
            }
        });
        let tied = Paragraph::builder(tied_text, 4_000)
            .build()
            .expect("valid tied split paragraph");
        assert_eq!(super::choose_warichu_split(&tied, 0..3), 1);

        let two = Paragraph::builder(text("ab"), 4_000)
            .build()
            .expect("valid two-member warichu fixture");
        let two_segment = super::warichu_segment(&two, 0..2, 0, 2);
        let mut placed = Vec::new();
        super::place_warichu_segment(&two, &two_segment, 50, 0, &mut placed);
        assert_eq!(
            placed
                .iter()
                .map(|item| (item.inline, item.block, item.advance))
                .collect::<Vec<_>>(),
            [(50, -500, 1_000), (50, 500, 1_000)]
        );

        let vertical_two = Paragraph::builder(text("ab"), 4_000)
            .writing_mode(WritingMode::VerticalRl)
            .build()
            .expect("valid vertical two-member fixture");
        let vertical_segment = super::warichu_segment(&vertical_two, 0..2, 0, 2);
        let mut vertical_placed = Vec::new();
        super::place_warichu_segment(
            &vertical_two,
            &vertical_segment,
            50,
            0,
            &mut vertical_placed,
        );
        assert_eq!(
            vertical_placed
                .iter()
                .map(|item| (item.inline, item.block, item.advance))
                .collect::<Vec<_>>(),
            [(50, 500, 1_000), (50, -500, 1_000)]
        );

        let outer = Paragraph::builder(
            mapped_text("日A日", Frame::FullEm, |ordinal, cluster| {
                if ordinal == 1 {
                    cluster.with_frame(Frame::Proportional)
                } else {
                    cluster
                }
            }),
            6_000,
        )
        .build()
        .expect("valid outer-boundary warichu fixture");
        assert_eq!(super::warichu_segment(&outer, 0..2, 0, 2).advance, 1_000);
        assert_eq!(super::warichu_segment(&outer, 0..2, 0, 3).advance, 1_250);

        let penalty = Paragraph::builder(text("abcd"), 8_000)
            .breaks([Break::allowed(1), Break::allowed(2), Break::allowed(3)])
            .constructs([Construct::warichu(0..4)])
            .build()
            .expect("valid penalty paragraph");
        assert_eq!(super::warichu_break_penalty(&penalty, 0), 0);
        assert_eq!(super::warichu_break_penalty(&penalty, 1), 2_000_000);
        assert_eq!(super::warichu_break_penalty(&penalty, 2), 0);
        assert_eq!(super::warichu_break_penalty(&penalty, 3), 2_000_000);
        assert_eq!(super::warichu_break_penalty(&penalty, 4), 0);
    }

    #[test]
    fn tab_measurement_skips_a_stop_at_the_current_cursor() {
        use crate::paragraph::{TabAlignment, TabStop};

        let paragraph = Paragraph::builder(
            mapped_text("A\tBC", Frame::Proportional, |_, cluster| cluster),
            10_000,
        )
        .tab_stops([
            TabStop::new(1_000, TabAlignment::Start).expect("valid stop"),
            TabStop::new(3_000, TabAlignment::Start).expect("valid stop"),
        ])
        .build()
        .expect("valid tab paragraph");
        assert_eq!(
            super::segment_width(
                &paragraph,
                &Style::default(),
                2,
                4,
                super::LineContext {
                    start: 0,
                    end: 4,
                    index: 0,
                }
            ),
            2_000
        );
        assert_eq!(
            super::measure_line(&paragraph, &Style::default(), 0, 4, 0),
            5_000
        );
        let mut advances = [1_000; 4];
        super::apply_tabs(&paragraph, &Style::default(), 0, 4, 0, &mut advances);
        assert_eq!(advances, [1_000, 2_000, 1_000, 1_000]);
    }

    #[test]
    fn kinsoku_helpers_cover_every_level_and_relaxation_mechanism() {
        use crate::style::{
            GroupedNumeralBeforeWestern, IterationMarkAtLineHead, KinsokuLevel, RelaxationMechanism,
        };

        assert_eq!(super::kinsoku_level_bit(KinsokuLevel::VeryLoose), 0b0001);
        assert_eq!(super::kinsoku_level_bit(KinsokuLevel::Loose), 0b0010);
        assert_eq!(super::kinsoku_level_bit(KinsokuLevel::Strict), 0b0100);
        assert_eq!(super::kinsoku_level_bit(KinsokuLevel::VeryStrict), 0b1000);

        let strict = Style::default();
        assert_eq!(super::reclassified_break_class(&strict, 10, Some('ぁ')), 16);
        assert_eq!(super::reclassified_break_class(&strict, 11, Some('ぁ')), 15);
        assert_eq!(super::reclassified_break_class(&strict, 11, Some('ァ')), 16);
        assert_eq!(super::reclassified_break_class(&strict, 19, Some('日')), 19);
        let iteration = Style::builder()
            .iteration_mark_at_line_head(IterationMarkAtLineHead::Permitted)
            .build()
            .expect("valid iteration style");
        assert_eq!(
            super::reclassified_break_class(&iteration, 12, Some('々')),
            19
        );
        let very_strict = Style::builder()
            .kinsoku_level(KinsokuLevel::VeryStrict)
            .iteration_mark_at_line_head(IterationMarkAtLineHead::Permitted)
            .grouped_numeral_before_western(GroupedNumeralBeforeWestern::Unbreakable)
            .relaxation_mechanism(RelaxationMechanism::Matrix)
            .build()
            .expect("valid very-strict style");
        assert_eq!(
            super::reclassified_break_class(&very_strict, 12, Some('々')),
            12
        );
        assert_eq!(
            super::reclassified_break_class(&very_strict, 10, Some('ぁ')),
            10
        );

        let very_loose = Style::builder()
            .kinsoku_level(KinsokuLevel::VeryLoose)
            .build()
            .expect("valid very-loose style");
        assert!(super::c_3_relaxes_boundary(&very_loose, 3, 19, None, None));
        assert!(super::c_3_relaxes_boundary(
            &very_loose,
            8,
            8,
            Some('〳'),
            Some('〵')
        ));
        assert!(!super::c_3_relaxes_boundary(
            &very_loose,
            19,
            19,
            Some('日'),
            Some('本')
        ));

        let loose = Style::builder()
            .kinsoku_level(KinsokuLevel::Loose)
            .build()
            .expect("valid loose style");
        for pair in [
            (19, 19, Some('・'), Some('日')),
            (8, 8, Some('…'), Some('…')),
            (19, 19, Some('%'), Some('日')),
        ] {
            assert!(super::c_3_relaxes_boundary(
                &loose, pair.0, pair.1, pair.2, pair.3
            ));
        }
        let matrix = Style::builder()
            .relaxation_mechanism(RelaxationMechanism::Matrix)
            .build()
            .expect("valid matrix style");
        assert_eq!(super::reclassified_break_class(&matrix, 10, Some('ぁ')), 10);
        assert!(super::c_3_relaxes_boundary(&matrix, 10, 19, None, None));
        assert!(!super::c_3_relaxes_boundary(&strict, 10, 19, None, None));
        assert!(!super::c_3_relaxes_boundary(
            &very_strict,
            3,
            10,
            Some('々'),
            Some('%')
        ));

        assert!(super::c_3_relaxes_boundary(
            &iteration,
            19,
            19,
            Some('々'),
            None
        ));
        assert!(super::c_3_relaxes_boundary(
            &iteration,
            19,
            19,
            None,
            Some('々')
        ));
        assert!(!super::c_3_relaxes_boundary(
            &strict,
            19,
            19,
            Some('々'),
            None
        ));
        assert!(!super::c_3_relaxes_boundary(
            &iteration,
            19,
            19,
            Some('日'),
            Some('本')
        ));
        let loose_matrix = Style::builder()
            .kinsoku_level(KinsokuLevel::Loose)
            .relaxation_mechanism(RelaxationMechanism::Matrix)
            .build()
            .expect("valid loose matrix style");
        assert!(super::c_3_relaxes_boundary(
            &loose_matrix,
            10,
            19,
            None,
            None
        ));

        assert!(super::cl_08_same_kind(Some('—'), Some('—')));
        assert!(super::cl_08_same_kind(Some('〳'), Some('〵')));
        assert!(!super::cl_08_same_kind(Some('〳'), Some('A')));
        assert!(!super::cl_08_same_kind(Some('A'), Some('〵')));
        assert!(!super::cl_08_same_kind(Some('—'), Some('…')));
        assert!(!super::cl_08_same_kind(None, Some('〵')));
    }

    #[test]
    fn break_legality_keeps_common_prohibitions_and_tab_cut() {
        use crate::paragraph::{TabAlignment, TabStop};

        let opening = Paragraph::builder(text("（A"), 4_000)
            .breaks([Break::allowed(3)])
            .build()
            .expect("valid opening paragraph");
        assert!(super::break_is_legal(&opening, &Style::default(), 0));
        assert!(!super::break_is_legal(&opening, &Style::default(), 3));
        assert!(super::break_is_legal(
            &opening,
            &Style::default(),
            opening.text.source().len()
        ));

        let closing = Paragraph::builder(text("A）"), 4_000)
            .breaks([Break::allowed(1)])
            .build()
            .expect("valid closing paragraph");
        assert!(!super::break_is_legal(&closing, &Style::default(), 1));

        let opening_before_iteration = Paragraph::builder(text("（々"), 4_000)
            .breaks([Break::allowed(3)])
            .build()
            .expect("valid opening/iteration paragraph");
        let iteration = Style::builder()
            .iteration_mark_at_line_head(crate::style::IterationMarkAtLineHead::Permitted)
            .build()
            .expect("valid iteration style");
        assert!(!super::break_is_legal(
            &opening_before_iteration,
            &iteration,
            3
        ));

        let percent_before_closing = Paragraph::builder(text("%）"), 4_000)
            .breaks([Break::allowed(1)])
            .build()
            .expect("valid percent/closing paragraph");
        assert!(!super::break_is_legal(
            &percent_before_closing,
            &Style::magazine_2020(),
            1
        ));

        let tab = Paragraph::builder(
            mapped_text("A\t", Frame::Proportional, |_, cluster| cluster),
            4_000,
        )
        .tab_stops([TabStop::new(2_000, TabAlignment::Start).expect("valid stop")])
        .build()
        .expect("valid tab paragraph");
        assert!(super::break_is_legal(&tab, &Style::default(), 1));
    }

    #[test]
    fn formula_and_attachment_geometry_are_centered_with_integer_division() {
        let formula = Paragraph::builder(
            mapped_text("A=+B", Frame::Proportional, |_, cluster| cluster),
            8_000,
        )
        .constructs([Construct::formula(0..4)])
        .build()
        .expect("valid independent formula");
        assert_eq!(super::formula_break_penalty(&formula, 0), 0);
        assert_eq!(super::formula_break_penalty(&formula, 1), 0);
        assert_eq!(super::formula_break_penalty(&formula, 2), 100_000_000);
        assert_eq!(super::formula_break_penalty(&formula, 3), 200_000_000);
        assert_eq!(super::formula_break_penalty(&formula, 4), 0);

        let script = Paragraph::builder(text("ab"), 4_000)
            .constructs([Construct::script(0..2, text("x"))])
            .build()
            .expect("valid script paragraph");
        let mut script_line = line(
            0..2,
            vec![
                placement(0, 0..1, 0, 1_000, crate::CoordinateTransform::Identity),
                placement(1, 1..2, 1_000, 1_000, crate::CoordinateTransform::Identity),
            ],
        );
        let mut construct_ordinals = Vec::new();
        super::place_attachments(
            &script,
            &Style::default(),
            0,
            0,
            2,
            &mut script_line,
            &mut construct_ordinals,
        );
        assert_eq!(script_line.attachments.len(), 1);
        assert_eq!(script_line.attachments[0].inline(), 500);
        assert_eq!(script_line.attachments[0].block(), 0);
        assert_eq!(script_line.block_extent, 2_000);

        let emphasis = Paragraph::builder(text("ab"), 4_000)
            .constructs([Construct::emphasis_dots(0..2, '・')])
            .build()
            .expect("valid emphasis paragraph");
        let mut emphasis_line = line(0..2, script_line.clusters.clone());
        super::place_attachments(
            &emphasis,
            &Style::default(),
            0,
            0,
            2,
            &mut emphasis_line,
            &mut construct_ordinals,
        );
        assert_eq!(
            emphasis_line
                .attachments
                .iter()
                .map(|attachment| (attachment.inline(), attachment.block(), attachment.symbol()))
                .collect::<Vec<_>>(),
            [(497, 995, Some('・')), (1_497, 995, Some('・'))]
        );
    }

    #[test]
    fn local_orientation_distinguishes_horizontal_vertical_and_tate_chu_yoko() {
        let horizontal = Paragraph::builder(
            mapped_text("A", Frame::Proportional, |_, cluster| cluster),
            2_000,
        )
        .build()
        .expect("valid horizontal paragraph");
        assert_eq!(
            super::local_orientation(&horizontal, 0, Frame::Proportional),
            (
                WritingMode::HorizontalTb,
                crate::CoordinateTransform::Identity
            )
        );

        let vertical = Paragraph::builder(
            mapped_text("A日", Frame::FullEm, |ordinal, cluster| {
                if ordinal == 0 {
                    cluster.with_frame(Frame::Proportional)
                } else {
                    cluster
                }
            }),
            3_000,
        )
        .writing_mode(WritingMode::VerticalRl)
        .build()
        .expect("valid vertical paragraph");
        assert_eq!(
            super::local_orientation(&vertical, 0, Frame::Proportional),
            (
                WritingMode::VerticalRl,
                crate::CoordinateTransform::RotateClockwise
            )
        );
        assert_eq!(
            super::local_orientation(&vertical, 1, Frame::FullEm),
            (
                WritingMode::VerticalRl,
                crate::CoordinateTransform::Identity
            )
        );

        let tcy = Paragraph::builder(text("12"), 3_000)
            .constructs([Construct::tate_chu_yoko(0..2)])
            .writing_mode(WritingMode::VerticalRl)
            .build()
            .expect("valid tate-chu-yoko paragraph");
        assert_eq!(
            super::local_orientation(&tcy, 0, Frame::FullEm),
            (
                WritingMode::HorizontalTb,
                crate::CoordinateTransform::TateChuYoko
            )
        );

        let partial_tcy = Paragraph::builder(text("12日"), 4_000)
            .constructs([Construct::tate_chu_yoko(0..2)])
            .writing_mode(WritingMode::VerticalRl)
            .build()
            .expect("valid partial tate-chu-yoko paragraph");
        assert_eq!(
            super::local_orientation(&partial_tcy, 2, Frame::FullEm),
            (
                WritingMode::VerticalRl,
                crate::CoordinateTransform::Identity
            )
        );
    }

    #[test]
    fn optimal_search_uses_the_whole_paragraph() {
        let source = "日本語組版";
        let paragraph = Paragraph::builder(text(source), 4_000)
            .breaks(
                source
                    .char_indices()
                    .skip(1)
                    .map(|(offset, _)| Break::allowed(offset)),
            )
            .widow(Widow::MinimumClusters(2))
            .build()
            .expect("valid paragraph");
        let layout = crate::compose(&paragraph, &Style::default()).expect("composition succeeds");
        assert_eq!(layout.lines().len(), 2);
        assert_eq!(layout.lines()[0].clusters().len(), 3);
    }

    #[test]
    fn vertical_lines_progress_toward_negative_block_coordinates() {
        let paragraph = Paragraph::builder(text("日本"), 1_000)
            .breaks(vec![Break::allowed(3)])
            .writing_mode(WritingMode::VerticalRl)
            .build()
            .expect("valid paragraph");
        let layout = crate::compose(&paragraph, &Style::default()).expect("composition succeeds");
        assert_eq!(layout.lines().len(), 2);
        assert!(layout.lines()[1].block_origin() < layout.lines()[0].block_origin());
    }

    #[test]
    fn distinct_ornamented_complexes_lower_to_table_six_stage_three() {
        let paragraph = Paragraph::builder(text("日本"), 2_000)
            .constructs([
                Construct::script(0..3, text("注")),
                Construct::script(3..6, text("記")),
            ])
            .build()
            .expect("valid ornamented paragraph");
        assert_eq!(
            super::boundary_expansion_site(&paragraph, &Style::default(), 0),
            super::ExpansionSite::Site {
                weight: 1_000,
                bounded: Some((250, 3)),
                residual: false,
            }
        );
    }

    #[test]
    fn extreme_capped_remainders_do_not_take_per_unit_work() {
        let sites = [
            super::ReductionSite {
                boundary: 0,
                weight: i32::MAX,
                capacity: i32::MAX,
                stage: 1,
                discrete: false,
            },
            super::ReductionSite {
                boundary: 1,
                weight: 1,
                capacity: i32::MAX,
                stage: 1,
                discrete: false,
            },
        ];
        let amount = i64::from(i32::MAX).saturating_mul(2);
        let mut reductions = [0, 0];
        super::distribute_reduction(amount, &sites, Remainder::Leading, &mut reductions);
        assert_eq!(reductions, [i32::MIN.saturating_add(1); 2]);

        let mut expansions = [0, 0];
        super::distribute_adjustment(
            amount,
            &[(0, i32::MAX, Some(i32::MAX)), (1, 1, Some(i32::MAX))],
            Remainder::Trailing,
            &mut expansions,
        );
        assert_eq!(expansions, [i32::MAX; 2]);
    }

    #[test]
    fn capped_round_robin_preserves_leading_and_trailing_ties() {
        let capacities = [2, 3, 3];
        assert_eq!(
            super::capped_round_robin(5, &capacities, Remainder::Leading),
            [2, 2, 1]
        );
        assert_eq!(
            super::capped_round_robin(5, &capacities, Remainder::Trailing),
            [1, 2, 2]
        );
        assert_eq!(
            super::capped_round_robin(4, &[1, 3, 3], Remainder::Leading),
            [1, 2, 1]
        );
        assert_eq!(
            super::capped_round_robin(4, &[3, 3, 1], Remainder::Trailing),
            [1, 2, 1]
        );
    }

    #[test]
    fn indexed_search_matches_the_quadratic_oracle_across_profiles_and_directions() {
        let styles = [
            Style::jlreq_2020(),
            Style::book_2020(),
            Style::magazine_2020(),
            Style::newspaper_2020(),
            Style::jis_reading_2020(),
        ];
        for mode in [WritingMode::HorizontalTb, WritingMode::VerticalRl] {
            for style in styles {
                for extent in [1_000, 2_000, 3_000, 4_000] {
                    let paragraph = break_everywhere("日（A）本、語", extent, mode);
                    let mut composer = super::Composer::new();
                    composer
                        .compose(&paragraph, &style)
                        .expect("small oracle fixture is within limits");
                    let oracle = oracle_chosen(&paragraph, &style, &composer.candidates);
                    assert_eq!(composer.chosen, oracle, "mode={mode:?}, extent={extent}");
                }
            }
        }
    }

    #[test]
    #[ignore = "the release performance gate runs this explicitly"]
    fn ten_thousand_cluster_standard_paragraph_stays_below_the_search_budget() {
        fn transitions(cluster_count: usize) -> usize {
            let source: String = "日".repeat(cluster_count);
            let paragraph = break_everywhere(&source, 20_000, WritingMode::HorizontalTb);
            let mut composer = super::Composer::new();
            composer
                .compose(&paragraph, &Style::default())
                .expect("standard paragraph stays within default limits");
            composer.transitions
        }

        let thousand = transitions(1_000);
        let ten_thousand = transitions(10_000);
        assert!(
            ten_thousand <= 500_000,
            "observed {ten_thousand} transitions"
        );
        assert!(
            ten_thousand <= thousand.saturating_mul(12),
            "1k={thousand}, 10k={ten_thousand}"
        );
    }

    #[test]
    #[ignore = "the release pathological-input gate runs this explicitly"]
    fn zero_width_pathological_paragraph_stops_at_the_default_search_budget() {
        let source: String = "日".repeat(4_100);
        let clusters = source.char_indices().map(|(start, character)| {
            Cluster::new(start..start.saturating_add(character.len_utf8()), 0)
        });
        let shaped = ShapedText::new(
            &source,
            Size::square(1_000).expect("positive fixture size"),
            Frame::FullEm,
            clusters,
        )
        .expect("valid zero-width fixture");
        let paragraph = Paragraph::builder(shaped, 20_000)
            .breaks(
                source
                    .char_indices()
                    .skip(1)
                    .map(|(offset, _)| Break::allowed(offset)),
            )
            .build()
            .expect("pathological fixture remains inside static resource limits");
        let mut composer = super::Composer::new();
        let error = composer
            .compose(&paragraph, &Style::default())
            .expect_err("exact search must stop at the default transition budget");

        let limit = crate::CompositionLimits::DEFAULT_MAX_SEARCH_TRANSITIONS;
        assert_eq!(
            error.resource(),
            crate::CompositionResource::SearchTransitions
        );
        assert_eq!(error.limit(), limit);
        assert_eq!(error.observed(), limit.saturating_add(1));
        assert_eq!(composer.transitions, limit);
    }
}
