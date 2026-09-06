// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Every typed policy choice, asked whether any input can tell its answers
//! apart.
//!
//! `xtask api` holds `docs/public-api.toml` against `spec/derived/questions.tsv`
//! in both directions: twenty-two JLReq questions, twenty-two typed choices,
//! and the right number of variants in each. What it cannot ask is whether the
//! composer reads any of them. A choice whose answers no document distinguishes
//! is a knob that turns and does nothing, which is worse than a missing one —
//! a caller who sets it believes they have chosen.
//!
//! So this composes one corpus under every value of every choice and compares
//! the results. The corpus is deliberately wide rather than targeted: kinsoku
//! candidates, brackets and punctuation at line ends and heads, ruby of all
//! three kinds with a reading longer than its base, an iteration mark, grouped
//! numerals beside Western text, a measure tight enough to make the adjustment
//! ladder run in both directions, and a code point outside the tables. A
//! question that stays silent across all of it is reported by name.
//!
//! This is a coverage statement, not a conformance one: the conformance suite
//! says a question is answered *correctly*, and this says only that it is
//! answered at all. It is cheap enough to keep because it is the shape of check
//! that found three construct defects — sweep the parameter rather than trust
//! the one value a fixture happened to pick.

use core::fmt::Write as _;
use jlreq_core::style::{
    AdjustmentPreference, AmbiguousContext, ExpansionOrder, GroupRubyDistribution,
    GroupedNumeralBeforeWestern, GroupedNumeralQualification, HangingPunctuation,
    IterationMarkAtLineHead, JapaneseLatinExpansionCeiling, JukugoRubyLayout, KinsokuLevel,
    LineEndFullStopComma, LineEndPunctuation, LineHeadOpeningBracket, ReductionTable,
    RelaxationMechanism, Remainder, RubyAlignment, RubyOverhangIndent, RubyOverhangKana,
    SentenceMedialDividingMark, UnlistedCodePoint,
};

use jlreq_core::{
    Cluster, Composer, Construct, Frame, Paragraph, Ruby, RubyKind, RubyRun, Size, Style,
    WritingMode,
};

/// One em in the units these fixtures are measured in.
const EM: i32 = 1_000;

type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

fn shaped(source: &str) -> Fallible<jlreq_core::ShapedText> {
    let clusters: Vec<_> = source
        .char_indices()
        .map(|(start, character)| {
            let end = start.saturating_add(character.len_utf8());
            let advance = if character.is_ascii() { EM / 2 } else { EM };
            let cluster = Cluster::new(start..end, advance);
            if character.is_ascii() {
                cluster.with_frame(Frame::Proportional)
            } else {
                cluster
            }
        })
        .collect();
    Ok(jlreq_core::ShapedText::new(
        source,
        Size::square(EM)?,
        Frame::FullEm,
        clusters,
    )?)
}

fn ruby_paragraph(kind: RubyKind, extent: i32) -> Fallible<Paragraph> {
    let base = "日本語組版と数字12の例。";
    let annotation = shaped("にほんごくみはん")?;
    // Mono ruby wants one run per base cluster; the other two take the whole
    // base at once. The reading is longer than the base either way, which is
    // what makes the overhang and distribution questions have something to say.
    let runs = if kind == RubyKind::Mono {
        vec![
            RubyRun::new(0..3, 0..6),
            RubyRun::new(3..6, 6..12),
            RubyRun::new(6..9, 12..24),
        ]
    } else {
        vec![RubyRun::new(0..9, 0..24)]
    };
    let ruby = Ruby::new(kind, 0..9, annotation, runs)?;
    Ok(Paragraph::builder(shaped(base)?, extent)
        .constructs([Construct::ruby(ruby)])
        .build()?)
}

/// The same text with every cluster proportional, so that a code point the
/// tables do not list can be classified by its frame rather than as an
/// ideograph and the two answers differ.
fn proportional(source: &str) -> Fallible<jlreq_core::ShapedText> {
    let clusters: Vec<_> = source
        .char_indices()
        .map(|(start, character)| {
            let end = start.saturating_add(character.len_utf8());
            Cluster::new(start..end, EM / 2).with_frame(Frame::Proportional)
        })
        .collect();
    Ok(jlreq_core::ShapedText::new(
        source,
        Size::square(EM)?,
        Frame::Proportional,
        clusters,
    )?)
}

/// A reading shorter than the base it annotates, which is the case the two ruby
/// alignments answer differently.
fn short_ruby_paragraph(extent: i32) -> Fallible<Paragraph> {
    let ruby = Ruby::new(
        RubyKind::Group,
        0..9,
        shaped("よ")?,
        [RubyRun::new(0..9, 0..3)],
    )?;
    Ok(
        Paragraph::builder(shaped("日本語組版と数字の例。")?, extent)
            .constructs([Construct::ruby(ruby)])
            .build()?,
    )
}

/// Documents chosen so that every question has something to be asked about.
fn corpus() -> Fallible<Vec<Paragraph>> {
    let mut corpus = Vec::new();
    // Every whole and half em from two to twelve, so that a closing bracket, a
    // full stop, an iteration mark or a numeral group lands on a line edge for
    // some measure rather than for none: a question about what happens at a
    // line end is silent until something is at one.
    for steps in 4..=24_i32 {
        let extent = steps.saturating_mul(EM) / 2;
        for mode in [WritingMode::HorizontalTb, WritingMode::VerticalRl] {
            for source in [
                // Brackets and punctuation that can fall at a line end or head.
                "「引用」と（注記）、そして句読点。",
                // Dividing marks in the middle of a sentence, and at its end.
                "本当？そうだ！ですか。ええ。",
                // Grouped numerals beside Western text, and a full stop last.
                "第12章とChapter 12の対照。",
                // An iteration mark that can land at a line head.
                "人々の時々の同じ々。",
                // Japanese-Latin boundaries with room to expand or reduce.
                "日本語とLatinとABCと日本語の混在。",
                // A code point outside the tables, once as an ideograph-shaped
                // cluster and once proportional, which is what makes
                // `classification.unlisted_code_point` have two answers.
                "記号\u{2603}と本文。",
            ] {
                corpus.push(
                    Paragraph::builder(shaped(source)?, extent)
                        .writing_mode(mode)
                        .build()?,
                );
            }
            corpus.push(
                Paragraph::builder(proportional("記号☃と±本文。")?, extent)
                    .writing_mode(mode)
                    .build()?,
            );
        }
        for kind in [RubyKind::Mono, RubyKind::Group, RubyKind::Jukugo] {
            corpus.push(ruby_paragraph(kind, extent)?);
        }
        // A reading shorter than its base, which is what distinguishes the two
        // ways of aligning ruby over it.
        corpus.push(short_ruby_paragraph(extent)?);
    }
    Ok(corpus)
}

/// Everything the composer decided, as one comparable string.
fn render(corpus: &[Paragraph], style: &Style) -> Fallible<String> {
    let mut composer = Composer::new();
    let mut rendered = String::new();
    for paragraph in corpus {
        match composer.compose(paragraph, style) {
            Ok(layout) => {
                for line in layout.lines() {
                    write!(
                        rendered,
                        "{:?}/{}/{}/{}/{}|",
                        line.range(),
                        line.inline_origin(),
                        line.block_origin(),
                        line.inline_extent(),
                        line.block_extent()
                    )?;
                    for cluster in line.clusters() {
                        write!(
                            rendered,
                            "{},{},{};",
                            cluster.inline(),
                            cluster.block(),
                            cluster.advance()
                        )?;
                    }
                    for attachment in line.attachments() {
                        write!(rendered, "a{},{};", attachment.inline(), attachment.block())?;
                    }
                }
                for diagnostic in layout.diagnostics() {
                    write!(rendered, "!{diagnostic:?}")?;
                }
            },
            Err(error) => write!(rendered, "refused {error:?}")?,
        }
        rendered.push('\n');
    }
    Ok(rendered)
}

macro_rules! ask {
    ($silent:expr, $corpus:expr, $question:literal, $setter:ident, [$($value:expr),+ $(,)?]) => {{
        let mut answers: Vec<String> = Vec::new();
        for value in [$($value),+] {
            // Some answers are only legal beside a particular answer to another
            // question — very-strict kinsoku excludes a breakable
            // grouped-numeral boundary — and the builder says so. A refusal is
            // an observable answer too, so it is recorded rather than
            // unwrapped; what would not be observable is two values that
            // compose identically *and* build identically.
            answers.push(match Style::builder().$setter(value).build() {
                Ok(style) => render($corpus, &style)?,
                Err(error) => format!("refused {error:?}"),
            });
        }
        if answers.windows(2).all(|pair| pair[0] == pair[1]) {
            $silent.push($question);
        }
    }};
}

/// Questions this corpus cannot tell the answers of apart.
///
/// A name here is **not** a defect on its own: a question the composer answers
/// only for an input this corpus does not hold looks exactly the same from
/// here. Each of these is read by the pipeline and has a targeted test of its
/// own in `public_api.rs`; what the list records is the reach of this corpus,
/// and the corpus is the first thing to widen when a name is added.
///
/// One is different and is the reason this file exists.
/// **`adjustment.expansion_order` selects nothing at all** — `Style` stores it,
/// a getter and a setter expose it, and no line of the composer reads it.
/// `docs/decisions/expansion-ladder-scope.md` already records why: its
/// `implementation` answer rests on a §3.8.4 Note whose only coordinate
/// `docs/conformance-deferrals.toml` classifies non-observable, so both
/// reference engines answer the same layout either way and there is nothing to
/// publish until a coordinate exists that tells them apart. The sweep found it
/// without being told, which is the argument for keeping the sweep.
///
/// Ordered as the questions are asked below, because that is the order the
/// failure message prints.
const KNOWN_SILENT: &[&str] = &[
    "spacing.line_end_punctuation",
    "spacing.line_end_full_stop_comma",
    "ruby.overhang_indent",
    "ruby.alignment",
    "kinsoku.iteration_mark_at_line_head",
    "kinsoku.grouped_numeral_before_western",
    "spacing.sentence_medial_dividing_mark",
    "adjustment.japanese_latin_expansion_ceiling",
    // Reads nothing. See above.
    "adjustment.expansion_order",
    "adjustment.preference",
    "classification.unlisted_code_point",
    "classification.grouped_numeral_qualification",
    "kinsoku.relaxation_mechanism",
];

#[test]
fn every_typed_policy_choice_changes_something() -> Fallible<()> {
    let corpus = corpus()?;
    let mut silent = Vec::new();

    ask!(
        silent,
        &corpus,
        "kinsoku.level",
        kinsoku_level,
        [
            KinsokuLevel::VeryLoose,
            KinsokuLevel::Loose,
            KinsokuLevel::Strict,
            KinsokuLevel::VeryStrict,
        ]
    );
    ask!(
        silent,
        &corpus,
        "adjustment.reduction_table",
        reduction_table,
        [
            ReductionTable::Table3,
            ReductionTable::Table4,
            ReductionTable::Table5
        ]
    );
    ask!(
        silent,
        &corpus,
        "spacing.line_end_punctuation",
        line_end_punctuation,
        [LineEndPunctuation::HalfEm, LineEndPunctuation::Solid]
    );
    ask!(
        silent,
        &corpus,
        "spacing.line_end_full_stop_comma",
        line_end_full_stop_comma,
        [LineEndFullStopComma::Preferred, LineEndFullStopComma::Jis]
    );
    ask!(
        silent,
        &corpus,
        "spacing.line_head_opening_bracket",
        line_head_opening_bracket,
        [
            LineHeadOpeningBracket::Pattern1,
            LineHeadOpeningBracket::Pattern2,
            LineHeadOpeningBracket::Pattern3
        ]
    );
    ask!(
        silent,
        &corpus,
        "ruby.overhang_kana",
        ruby_overhang_kana,
        [
            RubyOverhangKana::Kana,
            RubyOverhangKana::Jis,
            RubyOverhangKana::Any,
            RubyOverhangKana::None
        ]
    );
    ask!(
        silent,
        &corpus,
        "ruby.overhang_indent",
        ruby_overhang_indent,
        [
            RubyOverhangIndent::Permitted,
            RubyOverhangIndent::Prohibited
        ]
    );
    ask!(
        silent,
        &corpus,
        "ruby.alignment",
        ruby_alignment,
        [RubyAlignment::Nakatsuki, RubyAlignment::Katatsuki]
    );
    ask!(
        silent,
        &corpus,
        "ruby.group_distribution",
        group_ruby_distribution,
        [GroupRubyDistribution::Jis, GroupRubyDistribution::Flush]
    );
    ask!(
        silent,
        &corpus,
        "ruby.jukugo_layout",
        jukugo_ruby_layout,
        [JukugoRubyLayout::Group, JukugoRubyLayout::Phonetic]
    );
    ask!(
        silent,
        &corpus,
        "kinsoku.iteration_mark_at_line_head",
        iteration_mark_at_line_head,
        [
            IterationMarkAtLineHead::Prohibited,
            IterationMarkAtLineHead::Permitted,
            IterationMarkAtLineHead::Replaced
        ]
    );
    ask!(
        silent,
        &corpus,
        "adjustment.hanging_punctuation",
        hanging_punctuation,
        [HangingPunctuation::None, HangingPunctuation::Hanging]
    );
    ask!(
        silent,
        &corpus,
        "kinsoku.grouped_numeral_before_western",
        grouped_numeral_before_western,
        [
            GroupedNumeralBeforeWestern::Breakable,
            GroupedNumeralBeforeWestern::Unbreakable
        ]
    );
    ask!(
        silent,
        &corpus,
        "spacing.sentence_medial_dividing_mark",
        sentence_medial_dividing_mark,
        [
            SentenceMedialDividingMark::Solid,
            SentenceMedialDividingMark::QuarterEm
        ]
    );
    ask!(
        silent,
        &corpus,
        "adjustment.japanese_latin_expansion_ceiling",
        japanese_latin_expansion_ceiling,
        [
            JapaneseLatinExpansionCeiling::HalfEm,
            JapaneseLatinExpansionCeiling::ThirdEm,
            JapaneseLatinExpansionCeiling::Rigid
        ]
    );
    ask!(
        silent,
        &corpus,
        "adjustment.expansion_order",
        expansion_order,
        [ExpansionOrder::Jis, ExpansionOrder::Implementation]
    );
    ask!(
        silent,
        &corpus,
        "adjustment.preference",
        adjustment_preference,
        [
            AdjustmentPreference::LeastAdjustment,
            AdjustmentPreference::EvenTexture
        ]
    );
    ask!(
        silent,
        &corpus,
        "adjustment.remainder",
        remainder,
        [Remainder::Leading, Remainder::Trailing]
    );
    ask!(
        silent,
        &corpus,
        "classification.unlisted_code_point",
        unlisted_code_point,
        [UnlistedCodePoint::ByFrame, UnlistedCodePoint::Ideographic]
    );
    ask!(
        silent,
        &corpus,
        "classification.ambiguous_context",
        ambiguous_context,
        [
            AmbiguousContext::LowestClass,
            AmbiguousContext::HighestClass
        ]
    );
    ask!(
        silent,
        &corpus,
        "classification.grouped_numeral_qualification",
        grouped_numeral_qualification,
        [
            GroupedNumeralQualification::ByWidth,
            GroupedNumeralQualification::ByRole
        ]
    );
    ask!(
        silent,
        &corpus,
        "kinsoku.relaxation_mechanism",
        relaxation_mechanism,
        [RelaxationMechanism::Reclassify, RelaxationMechanism::Matrix]
    );

    assert_eq!(
        silent, KNOWN_SILENT,
        "a typed policy choice this corpus cannot tell the answers of apart"
    );
    Ok(())
}
