// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The policy space: one row and one named `Question` identifier per place JLReq permits more than one answer.
//!
//! Do not edit. `cargo run -p xtask -- generate` writes this file, and
//! `generate --check` fails when regenerating it would change a byte. A hand
//! edit is a bug even when it is correct, because the next revision of the
//! specification will not carry it forward (ADR 0009).
//!
//! - Source: `spec/derived/questions.tsv`
//! - Source SHA-256: `aa988703189460344a18ed4c7556b104d12eba8681a7e95516c5c27fe4ce36dc`
//! - Specification: JLReq, 2020-08-11
//! - Generator: `xtask/src/generate.rs`, `xtask/src/policy.rs`
//! - Generator SHA-256: `45cdc44882a71f48759c6368ee0199b8bf920ed8a5dd6d927e8644e1805bc772`
//! - Entries: 22

use crate::policy::{ChoiceRecord, Exclusion, Question, QuestionRecord};
use crate::rule::RuleId;

/// Every place JLReq permits more than one answer, in the specification's
/// own reading order.
///
/// JLReq: §B.2, §C.2, §C.3, §D, §E.2
pub(crate) const QUESTIONS: &[QuestionRecord] = &[
    QuestionRecord {
        path: "kinsoku.level",
        rule: RuleId(50),
        choices: &[
            ChoiceRecord {
                name: "very-loose",
                statement: "Very loose (Newspapers)",
                rule: RuleId(50),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "loose",
                statement: "Loose (Magazines)",
                rule: RuleId(50),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "strict",
                statement: "Strict (Default, general publications)",
                rule: RuleId(50),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "very-strict",
                statement: "Very strict (General publications)",
                rule: RuleId(50),
                preferred: false,
                excludes: &[
                    Exclusion {
                        question: Question(12),
                        choice: 0,
                        rule: RuleId(50),
                    },
                    Exclusion {
                        question: Question(21),
                        choice: 0,
                        rule: RuleId(50),
                    },
                ],
            },
        ],
        presets: [2, 2, 2, 1, 0],
    },
    QuestionRecord {
        path: "adjustment.reduction_table",
        rule: RuleId(51),
        choices: &[
            ChoiceRecord {
                name: "table-3",
                statement: "Table 3 follows the method adopted by this document",
                rule: RuleId(51),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "table-4",
                statement: "Table 4 supplies an alternative way specified by JIS X 4051",
                rule: RuleId(51),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "table-5",
                statement: "Table 5, taking partially different approaches from the previous two, represents yet another method which can be seen in books or other publications",
                rule: RuleId(51),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 1, 2, 0, 0],
    },
    QuestionRecord {
        path: "spacing.line_end_punctuation",
        rule: RuleId(60),
        choices: &[
            ChoiceRecord {
                name: "half-em",
                statement: "The preferred spacing between closing brackets (cl-02) and the line end is a half em.",
                rule: RuleId(60),
                preferred: true,
                excludes: &[],
            },
            ChoiceRecord {
                name: "solid",
                statement: "The alternative is to set solid (JIS X 4051 adopts solid setting method",
                rule: RuleId(60),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 1, 0, 0, 0],
    },
    QuestionRecord {
        path: "spacing.line_end_full_stop_comma",
        rule: RuleId(64),
        choices: &[
            ChoiceRecord {
                name: "preferred",
                statement: "The preferred spacing between full stops (cl-06) or commas (cl-07) and the line end is a half em.",
                rule: RuleId(64),
                preferred: true,
                excludes: &[],
            },
            ChoiceRecord {
                name: "jis",
                statement: "The alternative is to set solid (JIS X 4051 specifies that the spacing after full stop (cl-06) is a half em and the spacing after comma (cl-07) is solid",
                rule: RuleId(64),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 1, 0, 0, 0],
    },
    QuestionRecord {
        path: "spacing.line_head_opening_bracket",
        rule: RuleId(4),
        choices: &[
            ChoiceRecord {
                name: "pattern-1",
                statement: "The first line indent after the line feed is set full-width (one em) and the next line after the first line break starts with no space",
                rule: RuleId(4),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "pattern-2",
                statement: "The first line indent after the line feed is set one and a half em and the next line indent after the first line break is set to a half em",
                rule: RuleId(4),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "pattern-3",
                statement: "The first line indent after the line feed is set at a half em and the next line after the first line break is set tentsuki",
                rule: RuleId(4),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 2, 0, 0],
    },
    QuestionRecord {
        path: "ruby.overhang_kana",
        rule: RuleId(65),
        choices: &[
            ChoiceRecord {
                name: "kana",
                statement: "the preferred approach is to allow the ruby text to be extended up to the size of the ruby character over the katakana",
                rule: RuleId(65),
                preferred: true,
                excludes: &[],
            },
            ChoiceRecord {
                name: "jis",
                statement: "if it is required to conform to JIS X 4051, ruby text shall not be extended over the katakana because katakana characters belong to the ideographic character class in JIS X 4051",
                rule: RuleId(65),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "any",
                statement: "one of which is to allow ruby text to be extended up to the size of the ruby character over any character including ideographic (cl-19) as well as hiragana (cl-15) and katakana (cl-16) characters",
                rule: RuleId(65),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "none",
                statement: "another is NOT to allow ruby text to be extended over any character from hiragana (cl-15), katakana (cl-16) and ideographic characters (cl-19)",
                rule: RuleId(65),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 1, 0, 0, 0],
    },
    QuestionRecord {
        path: "ruby.overhang_indent",
        rule: RuleId(66),
        choices: &[
            ChoiceRecord {
                name: "permitted",
                statement: "The preferred approach is to apply the same for the full-width line head indent at the beginning of a paragraph.",
                rule: RuleId(66),
                preferred: true,
                excludes: &[],
            },
            ChoiceRecord {
                name: "prohibited",
                statement: "The alternative approach is not to allow ruby text to be extended over the line head indent.",
                rule: RuleId(66),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
    QuestionRecord {
        path: "ruby.alignment",
        rule: RuleId(22),
        choices: &[
            ChoiceRecord {
                name: "nakatsuki",
                statement: "attach a ruby character so that its vertical center matches that of the base character",
                rule: RuleId(22),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "katatsuki",
                statement: "attach a ruby character so that the top of its virtual body is aligned with the top of that of the base character",
                rule: RuleId(22),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
    QuestionRecord {
        path: "ruby.group_distribution",
        rule: RuleId(23),
        choices: &[
            ChoiceRecord {
                name: "jis",
                statement: "add 1 unit of spacing between the start of the base text and the start of the ruby text, and between the end of the ruby text and the end of the base text. This will give a balanced appearance, and is the method specified in JIS X 4051",
                rule: RuleId(23),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "flush",
                statement: "Another way is to first align the leading characters for both the base text and ruby text and the ends of both trailing characters, and then add the same amount of inter-character spacing between the rest of the ruby characters",
                rule: RuleId(23),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
    QuestionRecord {
        path: "ruby.jukugo_layout",
        rule: RuleId(24),
        choices: &[
            ChoiceRecord {
                name: "group",
                statement: "The available methods include the layout as specified in JIS X 4051",
                rule: RuleId(24),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "phonetic",
                statement: "layout decided by the phonetic structure of the kanji compound word and the type of script of the adjacent characters",
                rule: RuleId(24),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
    QuestionRecord {
        path: "kinsoku.iteration_mark_at_line_head",
        rule: RuleId(72),
        choices: &[
            ChoiceRecord {
                name: "prohibited",
                statement: "Follow the principle by applying some sort of line adjustment.",
                rule: RuleId(72),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "permitted",
                statement: "Allow IDEOGRAPHIC ITERATION MARK \"々\" to be placed either at the line head or at the head of an inline cutting note.",
                rule: RuleId(72),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "replaced",
                statement: "Replace IDEOGRAPHIC ITERATION MARK \"々\" with the corresponding character.",
                rule: RuleId(72),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
    QuestionRecord {
        path: "adjustment.hanging_punctuation",
        rule: RuleId(42),
        choices: &[
            ChoiceRecord {
                name: "none",
                statement: "This method is not formally defined in JIS X 4051, however JIS X 4051 does provide explanatory material about it.",
                rule: RuleId(42),
                preferred: true,
                excludes: &[],
            },
            ChoiceRecord {
                name: "hanging",
                statement: "Line adjustment by hanging punctuation is a method of avoiding line head wrap of full stops (cl-06) and commas (cl-07).",
                rule: RuleId(42),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 1, 0, 0],
    },
    QuestionRecord {
        path: "kinsoku.grouped_numeral_before_western",
        rule: RuleId(85),
        choices: &[
            ChoiceRecord {
                name: "breakable",
                statement: "one is to allow a line to break between preceding grouped numerals (cl-24) and trailing Western characters (cl-27)",
                rule: RuleId(85),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "unbreakable",
                statement: "and the other is not to",
                rule: RuleId(85),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
    QuestionRecord {
        path: "spacing.sentence_medial_dividing_mark",
        rule: RuleId(5),
        choices: &[
            ChoiceRecord {
                name: "solid",
                statement: "add no spacing",
                rule: RuleId(5),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "quarter-em",
                statement: "a quarter em spacing before and after the dividing punctuation mark",
                rule: RuleId(5),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
    QuestionRecord {
        path: "adjustment.japanese_latin_expansion_ceiling",
        rule: RuleId(44),
        choices: &[
            ChoiceRecord {
                name: "half-em",
                statement: "half em spacing",
                rule: RuleId(44),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "third-em",
                statement: "one third em spacing",
                rule: RuleId(44),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "rigid",
                statement: "is regarded as a fixed spacing, and spacing adaptation is not applied",
                rule: RuleId(44),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
    QuestionRecord {
        path: "adjustment.expansion_order",
        rule: RuleId(44),
        choices: &[
            ChoiceRecord {
                name: "jis",
                statement: "In JIS X 4051, the following processing order is defined.",
                rule: RuleId(44),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "implementation",
                statement: "it depends on each layout processing system whether inter-character spacing should be added equally",
                rule: RuleId(44),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
    QuestionRecord {
        path: "adjustment.preference",
        rule: RuleId(50),
        choices: &[
            ChoiceRecord {
                name: "least-adjustment",
                statement: "the very strict rule is for the best appearance at the line head",
                rule: RuleId(50),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "even-texture",
                statement: "the strict rule is best to avoid inter-character spacing adjustment",
                rule: RuleId(50),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
    QuestionRecord {
        path: "adjustment.remainder",
        rule: RuleId(43),
        choices: &[
            ChoiceRecord {
                name: "leading",
                statement: "The same width reduction is applied to all spaces on the target line at the same time.",
                rule: RuleId(43),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "trailing",
                statement: "The same width reduction is applied to all spaces on the target line at the same time.",
                rule: RuleId(43),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
    QuestionRecord {
        path: "classification.unlisted_code_point",
        rule: RuleId(46),
        choices: &[
            ChoiceRecord {
                name: "by-frame",
                statement: "Furthermore JIS X 4051 states that it is implementation-defined how to handle characters that are not explicitly mentioned, e.g. whether they should belong to either class or not.",
                rule: RuleId(46),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "ideographic",
                statement: "Furthermore JIS X 4051 states that it is implementation-defined how to handle characters that are not explicitly mentioned, e.g. whether they should belong to either class or not.",
                rule: RuleId(46),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
    QuestionRecord {
        path: "classification.ambiguous_context",
        rule: RuleId(46),
        choices: &[
            ChoiceRecord {
                name: "lowest-class",
                statement: "In this particular case, Japanese design is better.",
                rule: RuleId(46),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "highest-class",
                statement: "In this case, English spelling is indicated using parentheses in a Japanese line of text.",
                rule: RuleId(46),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
    QuestionRecord {
        path: "classification.grouped_numeral_qualification",
        rule: RuleId(46),
        choices: &[
            ChoiceRecord {
                name: "by-width",
                statement: "Sequences of European numerals which are not full-width and are handled as Japanese text, the decimal point or the comma and space used as a decimal place indicator in numbers.",
                rule: RuleId(46),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "by-role",
                statement: "Sequences of European numerals which are not full-width and are handled as Japanese text, the decimal point or the comma and space used as a decimal place indicator in numbers.",
                rule: RuleId(46),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
    QuestionRecord {
        path: "kinsoku.relaxation_mechanism",
        rule: RuleId(50),
        choices: &[
            ChoiceRecord {
                name: "reclassify",
                statement: "the character shall be treated as a member of the ideographic character (cl-19) class",
                rule: RuleId(76),
                preferred: false,
                excludes: &[],
            },
            ChoiceRecord {
                name: "matrix",
                statement: "Breaking a line is allowed before or after the following character classes even though Table 2 prohibits it.",
                rule: RuleId(50),
                preferred: false,
                excludes: &[],
            },
        ],
        presets: [0, 0, 0, 0, 0],
    },
];

impl Question {
    /// The place at `kinsoku.level`.
    ///
    /// JLReq: §C.3
    pub const KINSOKU_LEVEL: Self = Self(0);

    /// The place at `adjustment.reduction_table`.
    ///
    /// JLReq: §D
    pub const REDUCTION_TABLE: Self = Self(1);

    /// The place at `spacing.line_end_punctuation`.
    ///
    /// JLReq: §B.2#2
    pub const LINE_END_PUNCTUATION: Self = Self(2);

    /// The place at `spacing.line_end_full_stop_comma`.
    ///
    /// JLReq: §B.2#6
    pub const LINE_END_FULL_STOP_COMMA: Self = Self(3);

    /// The place at `spacing.line_head_opening_bracket`.
    ///
    /// JLReq: §3.1.5
    pub const LINE_HEAD_OPENING_BRACKET: Self = Self(4);

    /// The place at `ruby.overhang_kana`.
    ///
    /// JLReq: §B.2#7
    pub const RUBY_OVERHANG_KANA: Self = Self(5);

    /// The place at `ruby.overhang_indent`.
    ///
    /// JLReq: §B.2#8
    pub const RUBY_OVERHANG_INDENT: Self = Self(6);

    /// The place at `ruby.alignment`.
    ///
    /// JLReq: §3.3.5
    pub const RUBY_ALIGNMENT: Self = Self(7);

    /// The place at `ruby.group_distribution`.
    ///
    /// JLReq: §3.3.6
    pub const GROUP_RUBY_DISTRIBUTION: Self = Self(8);

    /// The place at `ruby.jukugo_layout`.
    ///
    /// JLReq: §3.3.7
    pub const JUKUGO_RUBY_LAYOUT: Self = Self(9);

    /// The place at `kinsoku.iteration_mark_at_line_head`.
    ///
    /// JLReq: §B.2#14
    pub const ITERATION_MARK_AT_LINE_HEAD: Self = Self(10);

    /// The place at `adjustment.hanging_punctuation`.
    ///
    /// JLReq: §3.8.2
    pub const HANGING_PUNCTUATION: Self = Self(11);

    /// The place at `kinsoku.grouped_numeral_before_western`.
    ///
    /// JLReq: §C.2#10
    pub const GROUPED_NUMERAL_BEFORE_WESTERN: Self = Self(12);

    /// The place at `spacing.sentence_medial_dividing_mark`.
    ///
    /// JLReq: §3.1.6
    pub const SENTENCE_MEDIAL_DIVIDING_MARK: Self = Self(13);

    /// The place at `adjustment.japanese_latin_expansion_ceiling`.
    ///
    /// JLReq: §3.8.4
    pub const JAPANESE_LATIN_EXPANSION_CEILING: Self = Self(14);

    /// The place at `adjustment.expansion_order`.
    ///
    /// JLReq: §3.8.4
    pub const EXPANSION_ORDER: Self = Self(15);

    /// The place at `adjustment.preference`.
    ///
    /// JLReq: §C.3
    pub const ADJUSTMENT_PREFERENCE: Self = Self(16);

    /// The place at `adjustment.remainder`.
    ///
    /// JLReq: §3.8.3
    pub const REMAINDER: Self = Self(17);

    /// The place at `classification.unlisted_code_point`.
    ///
    /// JLReq: §3.9.2
    pub const UNLISTED_CODE_POINT: Self = Self(18);

    /// The place at `classification.ambiguous_context`.
    ///
    /// JLReq: §3.9.2
    pub const AMBIGUOUS_CONTEXT: Self = Self(19);

    /// The place at `classification.grouped_numeral_qualification`.
    ///
    /// JLReq: §3.9.2
    pub const GROUPED_NUMERAL_QUALIFICATION: Self = Self(20);

    /// The place at `kinsoku.relaxation_mechanism`.
    ///
    /// JLReq: §C.3
    pub const RELAXATION_MECHANISM: Self = Self(21);
}
