// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The policy space: one derivation of the `derive` gate, and one unit of the `generate` gate
//! built directly on it.
//!
//! `crate::derive` is stage 1 of the specification data pipeline and this module is one
//! entry in its registry, reading the ReSpec-rendered snapshot vendored at
//! `spec/snapshot/index.html`:
//!
//! - `spec/derived/questions.tsv` — every place JLReq permits more than one answer, with the
//!   address that permits it, every answer with the sentence it rests on and the section
//!   stating it, the answer `Policy::JLREQ` selects and the four further presets, and the
//!   exclusions `Policy::with` reads.
//!
//! `crate::generate` is stage 2, and the unit at the bottom of this file turns that file into
//! `crates/jlreq-spec/src/generated/policy.rs`: `QUESTIONS`, and one named `Question` constant
//! per row.
//!
//! Everything else about a derived or a generated file — that neither is edited by hand,
//! which sources they were read from and their digests, byte identity on a second run — is
//! the frame's, so this module is the reading, the resolving and nothing else.
//!
//! # Why the membership is written here and not scanned
//!
//! A question is "a section that states two readings", which is a reading of prose rather
//! than a property a scanner computes, and a heuristic would publish permitted alternatives
//! the specification does not permit. Three of the twenty-two rows below show why no scan
//! could find them: §3.8.3 permits the remainder question by saying *nothing* about where
//! the units that do not divide evenly go, §C.3's closing paragraph permits the adjustment
//! preference by describing what its four levels achieve rather than how to rank two
//! paragraphs, and §C.3's relaxation permits its own mechanism by allowing a break "even
//! though Table 2 prohibits it" without saying whether the matrix has been amended or the
//! character reclassified.
//!
//! This is [ADR 0009](../../docs/adr/0009-generated-data-and-attested-transcription.md)'s
//! attested category, and the control that earns it is the same one
//! `crate::inventory`'s direction mark carries: **the document supplies the evidence and
//! this table supplies the reading, and the derivation holds the two against each other.**
//! Every row quotes a sentence and names where it is; the derivation refuses to emit unless
//! that sentence is verbatim in that section or note, in that rendering — and now every
//! *answer* does the same, one level down, because a preset that picked an answer the
//! document does not support would publish invention as a citable claim.
//!
//! What the file therefore is *not* is a transcription in `spec/captured/`'s sense. Nothing
//! here is keyed in from a document a machine cannot read: the sentences are extracted from
//! the snapshot by `crate::inventory` and the rows carry no text of their own beyond the
//! quotation that is checked against it. Double entry is the control for a matrix that
//! exists only as PDF; the control for a reading of machine-readable prose is that the
//! prose is machine-read.
//!
//! # What the row says, and what it does not
//!
//! `permission` records *why* the alternative is permitted, which is the distinction ADR
//! 0009 exists for and the one a reader of a published policy surface most needs:
//!
//! | value | what the document does |
//! | --- | --- |
//! | `stated` | the section or note states the alternatives in so many words |
//! | `divergent` | the two renderings of one sentence do not state the same rule |
//! | `contradictory` | the document states one rule twice, in ways that are not equivalent |
//! | `silent` | the document decides nothing here, and the answers are this project's |
//!
//! Those map onto `jlreq_spec::Standing`: `stated` is an `Alternative`, `divergent` and
//! `contradictory` are `Adjudicated`, and `silent` is `Unstated`. Only `stated` is a
//! permission the specification grants; the other three are permissions this project reads
//! *out of* the specification, and the column is what stops the three being laundered into
//! the first.
//!
//! Where a question is `silent`, an answer's `statement` is not a sentence stating that
//! answer — there is none — but the sentence whose *silence* permits the question, quoted
//! once for each answer it has, because nothing in the document distinguishes them further.
//! `Choice::statement`'s doc names both readings.
//!
//! `jlreq` is the answer `Policy::JLREQ` selects, under one rule applied to every row:
//! JLReq's own preference where it states one — §C.3's "Default, general publications",
//! §D's "the method adopted by this document", §B.2's "preferred" — then this project's
//! published reading where JLReq states nothing, and failing both the reading the
//! specification states first. `jis_reading`, `book`, `magazine` and `newspaper` are
//! `jlreq`'s own answer everywhere the document records no divergence for that practice, and
//! diverge from it only where `Policy`'s own published doc comments already promise a
//! divergence — reduction Table 5, pattern 3 and hanging punctuation for `book`; Loose
//! kinsoku for `magazine`; Very loose kinsoku for `newspaper`; the JIS X 4051 reading
//! wherever the document names one for `jis_reading`. A preset is overrides layered on
//! JLReq's own answer, not a second universe of answers, and recording it that way is what
//! keeps the four from silently diverging from what `crates/jlreq-spec/src/policy.rs`
//! already tells a caller they mean.
//!
//! # Hand-rolled, on purpose
//!
//! The reading is a `const` table rather than a data file for the reason `crate::classes`'s
//! Remarks vocabulary is one: a reading that lived in the output could be edited into
//! agreement with itself, and a reading that lives in the reader is covered by the reader
//! digest every derived file states.
//!
//! See `docs/design/generation.md`, `docs/adr/0009`, `docs/adr/0012` and `docs/adr/0013`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::derive::Derivation;
use crate::generate::{Emission, Record, Table, Unit};
use crate::inventory;

/// The vendored rendering of the published document.
const SNAPSHOT: &str = "spec/snapshot/index.html";

/// The path this derivation writes, for the findings that name it.
const QUESTIONS_FILE: &str = "spec/derived/questions.tsv";

/// The policy space, which the generated `Question` constants and `QUESTIONS` come from.
pub(crate) const QUESTIONS: Derivation = Derivation {
    sources: &[SNAPSHOT],
    reader: &["xtask/src/inventory.rs", "xtask/src/policy.rs"],
    output: QUESTIONS_FILE,
    caption: concat!(
        "One row per place JLReq permits more than one answer: the `Question` constant ",
        "docs/design/api-spine.md publishes, the address that permits the alternative, every ",
        "answer with the sentence it rests on and the rule stating it, whether JLReq calls it ",
        "preferred, the answer each of the five presets selects, and the exclusions ",
        "`Policy::with` reads."
    ),
    read: read_questions,
};

/// Which rendering of the bilingual document a sentence was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rendering {
    /// `its-locale-filter-list="en"`.
    English,
    /// `its-locale-filter-list="ja"`.
    Japanese,
}

impl Rendering {
    /// The tag the emitted `locale` column carries.
    const fn tag(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Japanese => "ja",
        }
    }

    /// This rendering's position in what `inventory::prose` returns.
    const fn column(self) -> usize {
        match self {
            Self::English => 0,
            Self::Japanese => 1,
        }
    }

    /// The other one.
    const fn other(self) -> Self {
        match self {
            Self::English => Self::Japanese,
            Self::Japanese => Self::English,
        }
    }
}

/// Where a question's permission comes from.
///
/// The four values are the module documentation's table, and the difference between the
/// first and the other three is the whole of ADR 0009's split applied to prose: one is a
/// permission the specification grants and three are permissions read out of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Permission {
    /// The section or note states the alternatives in so many words.
    Stated,
    /// The two renderings of one sentence do not state the same rule.
    Divergent,
    /// The document states one rule twice, in ways that are not equivalent.
    Contradictory,
    /// The document decides nothing here, and the answers are this project's.
    Silent,
}

impl Permission {
    /// The token the emitted `permission` column carries.
    const fn token(self) -> &'static str {
        match self {
            Self::Stated => "stated",
            Self::Divergent => "divergent",
            Self::Contradictory => "contradictory",
            Self::Silent => "silent",
        }
    }
}

/// One permitted answer of one question.
///
/// The section that states an answer is recorded per answer and not inherited from the
/// question, because one of the twenty-two is not: `kinsoku.relaxation_mechanism`'s
/// `reclassify` is stated at §C.2#1, while the question itself is addressed at §C.3, the
/// section that permits choosing between the two mechanisms (`docs/adr/0013`).
#[derive(Debug)]
struct Answer {
    /// The stable name a conformance case file names this answer by.
    name: &'static str,
    /// The section or note that states this alternative, in ADR 0013's address grammar.
    address: &'static str,
    /// The sentence the alternative rests on, verbatim in the `en` rendering of `address`.
    /// Where the owning question's `permission` is `Silent`, this is the sentence whose
    /// silence permits the question rather than a sentence stating this answer by name.
    statement: &'static str,
    /// Whether JLReq calls this one "preferred". JLReq: §B.2#2, #7, #8 among these
    /// twenty-two; the wider set §B.2 states it at is #1, #2, #4, #6, #7, #8, #17.
    preferred: bool,
    /// The answers of other questions this one excludes, read from both ends by
    /// `jlreq_spec::Policy::with`. Empty for every answer but `kinsoku.level`'s
    /// `very-strict`, which is the one place among these twenty-two the specification states
    /// an exclusion: "no alternate rule explained in § C.2 Notes is applied" at Very strict.
    excludes: &'static [Exclusion],
}

/// One answer of one other question that an [`Answer`] excludes.
#[derive(Debug)]
struct Exclusion {
    /// The other question's path.
    question: &'static str,
    /// The other question's answer that cannot stand beside this one.
    answer: &'static str,
    /// The section stating the exclusion.
    address: &'static str,
}

/// One place JLReq permits more than one answer.
#[derive(Debug)]
struct Question {
    /// The `jlreq_spec::Question` constant `docs/design/api-spine.md` publishes, which the
    /// `api` gate holds this table equal to.
    constant: &'static str,
    /// The stable dotted path a conformance case file names this question by.
    path: &'static str,
    /// The section or note that permits the alternative, in ADR 0013's address grammar.
    address: &'static str,
    /// Why the alternative is permitted.
    permission: Permission,
    /// The rendering `evidence` was read from.
    locale: Rendering,
    /// The sentence the permission rests on, verbatim in that rendering of that address.
    evidence: &'static str,
    /// The permitted answers, in the order the specification states them.
    answers: &'static [Answer],
    /// The answer `Policy::JLREQ` selects, which is one of `answers`.
    jlreq: &'static str,
    /// The answer `Policy::JIS_READING` selects: the JIS X 4051 reading wherever the
    /// document records one, `jlreq`'s own answer everywhere else.
    jis_reading: &'static str,
    /// The answer `Policy::BOOK` selects.
    book: &'static str,
    /// The answer `Policy::MAGAZINE` selects.
    magazine: &'static str,
    /// The answer `Policy::NEWSPAPER` selects.
    newspaper: &'static str,
}

/// How many places JLReq permits more than one answer.
///
/// The table below is an array of exactly this many rather than a slice, so a row added
/// without this figure moving does not compile. It is the same arrangement `crate::classes`
/// uses for its per-class census: a count the reader derives from its own input proves only
/// that the reader is self-consistent, and a count written down is a claim someone made.
///
/// The figure is not the whole control. `docs/design/api-spine.md` publishes one `Question`
/// constant per row, and the `api` gate holds that list and the emitted file equal in both
/// directions — so a row without a constant, or a constant without a row, fails the build
/// whichever side it was added on.
const QUESTION_COUNT: usize = 22;

/// The fewest answers a question permits. One answer is not a question.
const LEAST_ANSWERS: usize = 2;

/// Every place JLReq permits more than one answer, in the order `docs/design/api-spine.md`
/// publishes the constants.
///
/// Each answer is a claim about the published document: that `statement` is verbatim in the
/// `en` rendering of `address`. `read_questions` refuses to emit unless every one of them
/// holds, so the reading below cannot drift from the specification in silence.
const POLICY_SPACE: [Question; QUESTION_COUNT] = [
    Question {
        constant: "KINSOKU_LEVEL",
        path: "kinsoku.level",
        address: "C.3",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "The following lists four levels of convention.",
        answers: &[
            Answer {
                name: "very-loose",
                address: "C.3",
                statement: "Very loose (Newspapers)",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "loose",
                address: "C.3",
                statement: "Loose (Magazines)",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "strict",
                address: "C.3",
                statement: "Strict (Default, general publications)",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "very-strict",
                address: "C.3",
                statement: "Very strict (General publications)",
                preferred: false,
                // §C.3's own level-4 paragraph: "no alternate rule explained in § C.2 Notes
                // is applied". Among these twenty-two, `kinsoku.grouped_numeral_before_
                // western`'s `breakable` and `kinsoku.relaxation_mechanism`'s `reclassify`
                // are the two answers that apply a §C.2 alternate rule.
                excludes: &[
                    Exclusion {
                        question: "kinsoku.grouped_numeral_before_western",
                        answer: "breakable",
                        address: "C.3",
                    },
                    Exclusion {
                        question: "kinsoku.relaxation_mechanism",
                        answer: "reclassify",
                        address: "C.3",
                    },
                ],
            },
        ],
        jlreq: "strict",
        jis_reading: "strict",
        book: "strict",
        magazine: "loose",
        newspaper: "very-loose",
    },
    Question {
        constant: "REDUCTION_TABLE",
        path: "adjustment.reduction_table",
        address: "D",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "Table 3 follows the method adopted by this document, Table 4 supplies an \
                   alternative way specified by JIS X 4051, and Table 5, taking partially \
                   different approaches from the previous two, represents yet another method \
                   which can be seen in books or other publications.",
        answers: &[
            Answer {
                name: "table-3",
                address: "D",
                statement: "Table 3 follows the method adopted by this document",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "table-4",
                address: "D",
                statement: "Table 4 supplies an alternative way specified by JIS X 4051",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "table-5",
                address: "D",
                statement: "Table 5, taking partially different approaches from the previous \
                            two, represents yet another method which can be seen in books or \
                            other publications",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "table-3",
        jis_reading: "table-4",
        book: "table-5",
        magazine: "table-3",
        newspaper: "table-3",
    },
    Question {
        constant: "LINE_END_PUNCTUATION",
        path: "spacing.line_end_punctuation",
        address: "B.2#2",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "The preferred spacing between closing brackets (cl-02) and the line end \
                   is a half em. The alternative is to set solid",
        answers: &[
            Answer {
                name: "half-em",
                address: "B.2#2",
                statement: "The preferred spacing between closing brackets (cl-02) and the \
                            line end is a half em.",
                preferred: true,
                excludes: &[],
            },
            Answer {
                name: "solid",
                address: "B.2#2",
                statement: "The alternative is to set solid (JIS X 4051 adopts solid setting \
                            method",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "half-em",
        jis_reading: "solid",
        book: "half-em",
        magazine: "half-em",
        newspaper: "half-em",
    },
    Question {
        constant: "LINE_END_FULL_STOP_COMMA",
        path: "spacing.line_end_full_stop_comma",
        address: "B.2#6",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "The preferred spacing between full stops (cl-06) or commas (cl-07) and \
                   the line end is a half em. The alternative is to set solid",
        answers: &[
            Answer {
                name: "preferred",
                address: "B.2#6",
                statement: "The preferred spacing between full stops (cl-06) or commas \
                            (cl-07) and the line end is a half em.",
                preferred: true,
                excludes: &[],
            },
            Answer {
                name: "jis",
                address: "B.2#6",
                statement: "The alternative is to set solid (JIS X 4051 specifies that the \
                            spacing after full stop (cl-06) is a half em and the spacing \
                            after comma (cl-07) is solid",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "preferred",
        jis_reading: "jis",
        book: "preferred",
        magazine: "preferred",
        newspaper: "preferred",
    },
    Question {
        constant: "LINE_HEAD_OPENING_BRACKET",
        path: "spacing.line_head_opening_bracket",
        address: "3.1.5",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "When starting a new line with opening brackets (cl-01) there are some \
                   patterns as shown in Figure 71.",
        answers: &[
            Answer {
                name: "pattern-1",
                address: "3.1.5",
                statement: "The first line indent after the line feed is set full-width (one \
                            em) and the next line after the first line break starts with no \
                            space",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "pattern-2",
                address: "3.1.5",
                statement: "The first line indent after the line feed is set one and a half \
                            em and the next line indent after the first line break is set to \
                            a half em",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "pattern-3",
                address: "3.1.5",
                statement: "The first line indent after the line feed is set at a half em \
                            and the next line after the first line break is set tentsuki",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "pattern-1",
        jis_reading: "pattern-1",
        book: "pattern-3",
        magazine: "pattern-1",
        newspaper: "pattern-1",
    },
    Question {
        constant: "RUBY_OVERHANG_KANA",
        path: "ruby.overhang_kana",
        address: "B.2#7",
        permission: Permission::Stated,
        locale: Rendering::English,
        // `stated`, not `divergent`: see the module documentation of
        // `crates/jlreq-spec/src/generated.rs` and `spec/derived/defects.tsv`'s
        // `b2-note-7-locale-class-divergence` for why. §B.2 note 7's English half states all
        // four answers in so many words, which is what is quoted below.
        evidence: "There are alternative methods, one of which is to allow ruby text to be \
                   extended up to the size of the ruby character over any character including \
                   ideographic (cl-19) as well as hiragana (cl-15) and katakana (cl-16) \
                   characters",
        answers: &[
            Answer {
                name: "kana",
                address: "B.2#7",
                statement: "the preferred approach is to allow the ruby text to be extended \
                            up to the size of the ruby character over the katakana",
                preferred: true,
                excludes: &[],
            },
            Answer {
                name: "jis",
                address: "B.2#7",
                statement: "if it is required to conform to JIS X 4051, ruby text shall not \
                            be extended over the katakana because katakana characters belong \
                            to the ideographic character class in JIS X 4051",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "any",
                address: "B.2#7",
                statement: "one of which is to allow ruby text to be extended up to the size \
                            of the ruby character over any character including ideographic \
                            (cl-19) as well as hiragana (cl-15) and katakana (cl-16) \
                            characters",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "none",
                address: "B.2#7",
                statement: "another is NOT to allow ruby text to be extended over any \
                            character from hiragana (cl-15), katakana (cl-16) and \
                            ideographic characters (cl-19)",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "kana",
        jis_reading: "jis",
        book: "kana",
        magazine: "kana",
        newspaper: "kana",
    },
    Question {
        constant: "RUBY_OVERHANG_INDENT",
        path: "ruby.overhang_indent",
        address: "B.2#8",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "The alternative approach is not to allow ruby text to be extended over \
                   the line head indent.",
        answers: &[
            Answer {
                name: "permitted",
                address: "B.2#8",
                statement: "The preferred approach is to apply the same for the full-width \
                            line head indent at the beginning of a paragraph.",
                preferred: true,
                excludes: &[],
            },
            Answer {
                name: "prohibited",
                address: "B.2#8",
                statement: "The alternative approach is not to allow ruby text to be \
                            extended over the line head indent.",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "permitted",
        jis_reading: "permitted",
        book: "permitted",
        magazine: "permitted",
        newspaper: "permitted",
    },
    Question {
        constant: "RUBY_ALIGNMENT",
        path: "ruby.alignment",
        address: "3.3.5",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "When attaching a single hiragana (cl-15) ruby character to a single base \
                   character, there are two ways of positioning the ruby character.",
        answers: &[
            Answer {
                name: "nakatsuki",
                address: "3.3.5",
                statement: "attach a ruby character so that its vertical center matches that \
                            of the base character",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "katatsuki",
                address: "3.3.5",
                statement: "attach a ruby character so that the top of its virtual body is \
                            aligned with the top of that of the base character",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "nakatsuki",
        jis_reading: "nakatsuki",
        book: "nakatsuki",
        magazine: "nakatsuki",
        newspaper: "nakatsuki",
    },
    Question {
        constant: "GROUP_RUBY_DISTRIBUTION",
        path: "ruby.group_distribution",
        address: "3.3.6",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "Another way is to first align the leading characters for both the base \
                   text and ruby text and the ends of both trailing characters, and then add \
                   the same amount of inter-character spacing between the rest of the ruby \
                   characters",
        answers: &[
            Answer {
                name: "jis",
                address: "3.3.6",
                statement: "add 1 unit of spacing between the start of the base text and the \
                            start of the ruby text, and between the end of the ruby text and \
                            the end of the base text. This will give a balanced appearance, \
                            and is the method specified in JIS X 4051",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "flush",
                address: "3.3.6",
                statement: "Another way is to first align the leading characters for both the \
                            base text and ruby text and the ends of both trailing characters, \
                            and then add the same amount of inter-character spacing between \
                            the rest of the ruby characters",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "jis",
        jis_reading: "jis",
        book: "jis",
        magazine: "jis",
        newspaper: "jis",
    },
    Question {
        constant: "JUKUGO_RUBY_LAYOUT",
        path: "ruby.jukugo_layout",
        address: "3.3.7",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "The available methods include the layout as specified in JIS X 4051",
        answers: &[
            Answer {
                name: "group",
                address: "3.3.7",
                statement: "The available methods include the layout as specified in JIS X \
                            4051",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "phonetic",
                address: "3.3.7",
                statement: "layout decided by the phonetic structure of the kanji compound \
                            word and the type of script of the adjacent characters",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "group",
        jis_reading: "group",
        book: "group",
        magazine: "group",
        newspaper: "group",
    },
    Question {
        constant: "ITERATION_MARK_AT_LINE_HEAD",
        path: "kinsoku.iteration_mark_at_line_head",
        address: "B.2#14",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "there are three ways to deal with this situation",
        answers: &[
            Answer {
                name: "prohibited",
                address: "B.2#14",
                statement: "Follow the principle by applying some sort of line adjustment.",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "permitted",
                address: "B.2#14",
                statement: "Allow IDEOGRAPHIC ITERATION MARK \"々\" to be placed either at \
                            the line head or at the head of an inline cutting note.",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "replaced",
                address: "B.2#14",
                statement: "Replace IDEOGRAPHIC ITERATION MARK \"々\" with the corresponding \
                            character.",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "prohibited",
        jis_reading: "prohibited",
        book: "prohibited",
        magazine: "prohibited",
        newspaper: "prohibited",
    },
    Question {
        constant: "HANGING_PUNCTUATION",
        path: "adjustment.hanging_punctuation",
        address: "3.8.2",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "Line adjustment by hanging punctuation is a method of avoiding line head \
                   wrap of full stops (cl-06) and commas (cl-07). This method is not \
                   formally defined in JIS X 4051, however JIS X 4051 does provide \
                   explanatory material about it.",
        answers: &[
            Answer {
                name: "none",
                address: "3.8.2",
                statement: "This method is not formally defined in JIS X 4051, however JIS X \
                            4051 does provide explanatory material about it.",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "hanging",
                address: "3.8.2",
                statement: "Line adjustment by hanging punctuation is a method of avoiding \
                            line head wrap of full stops (cl-06) and commas (cl-07).",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "none",
        jis_reading: "none",
        book: "hanging",
        magazine: "none",
        newspaper: "none",
    },
    Question {
        constant: "GROUPED_NUMERAL_BEFORE_WESTERN",
        path: "kinsoku.grouped_numeral_before_western",
        address: "C.2#10",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "There are two approaches: one is to allow a line to break between \
                   preceding grouped numerals (cl-24) and trailing Western characters \
                   (cl-27), and the other is not to.",
        answers: &[
            Answer {
                name: "breakable",
                address: "C.2#10",
                statement: "one is to allow a line to break between preceding grouped \
                            numerals (cl-24) and trailing Western characters (cl-27)",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "unbreakable",
                address: "C.2#10",
                statement: "and the other is not to",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "breakable",
        jis_reading: "breakable",
        book: "breakable",
        magazine: "breakable",
        newspaper: "breakable",
    },
    Question {
        constant: "SENTENCE_MEDIAL_DIVIDING_MARK",
        path: "spacing.sentence_medial_dividing_mark",
        address: "3.1.6",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "either add no spacing or a quarter em spacing before and after the \
                   dividing punctuation mark",
        answers: &[
            Answer {
                name: "solid",
                address: "3.1.6",
                statement: "add no spacing",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "quarter-em",
                address: "3.1.6",
                statement: "a quarter em spacing before and after the dividing punctuation \
                            mark",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "solid",
        jis_reading: "solid",
        book: "solid",
        magazine: "solid",
        newspaper: "solid",
    },
    Question {
        constant: "JAPANESE_LATIN_EXPANSION_CEILING",
        path: "adjustment.japanese_latin_expansion_ceiling",
        address: "3.8.4",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "is increased equally with proportional character size, up to half em \
                   spacing (or one third em spacing)",
        answers: &[
            Answer {
                name: "half-em",
                address: "3.8.4",
                statement: "half em spacing",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "third-em",
                address: "3.8.4",
                statement: "one third em spacing",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "rigid",
                address: "3.8.4",
                statement: "is regarded as a fixed spacing, and spacing adaptation is not \
                            applied",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "half-em",
        jis_reading: "half-em",
        book: "half-em",
        magazine: "half-em",
        newspaper: "half-em",
    },
    Question {
        constant: "EXPANSION_ORDER",
        path: "adjustment.expansion_order",
        address: "3.8.4",
        permission: Permission::Silent,
        locale: Rendering::English,
        evidence: "In JIS X 4051, the following processing order is defined.",
        answers: &[
            Answer {
                name: "jis",
                address: "3.8.4",
                statement: "In JIS X 4051, the following processing order is defined.",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "implementation",
                address: "3.8.4",
                statement: "it depends on each layout processing system whether \
                            inter-character spacing should be added equally",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "jis",
        jis_reading: "jis",
        book: "jis",
        magazine: "jis",
        newspaper: "jis",
    },
    Question {
        constant: "ADJUSTMENT_PREFERENCE",
        path: "adjustment.preference",
        address: "C.3",
        permission: Permission::Silent,
        locale: Rendering::English,
        evidence: "the very strict rule is for the best appearance at the line head, while \
                   the strict rule is best to avoid inter-character spacing adjustment",
        answers: &[
            Answer {
                name: "least-adjustment",
                address: "C.3",
                statement: "the very strict rule is for the best appearance at the line head",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "even-texture",
                address: "C.3",
                statement: "the strict rule is best to avoid inter-character spacing \
                            adjustment",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "least-adjustment",
        jis_reading: "least-adjustment",
        book: "least-adjustment",
        magazine: "least-adjustment",
        newspaper: "least-adjustment",
    },
    Question {
        constant: "REMAINDER",
        path: "adjustment.remainder",
        address: "3.8.3",
        permission: Permission::Silent,
        locale: Rendering::English,
        evidence: "The same width reduction is applied to all spaces on the target line at \
                   the same time.",
        answers: &[
            Answer {
                name: "leading",
                address: "3.8.3",
                statement: "The same width reduction is applied to all spaces on the target \
                            line at the same time.",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "trailing",
                address: "3.8.3",
                statement: "The same width reduction is applied to all spaces on the target \
                            line at the same time.",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "leading",
        jis_reading: "leading",
        book: "leading",
        magazine: "leading",
        newspaper: "leading",
    },
    Question {
        constant: "UNLISTED_CODE_POINT",
        path: "classification.unlisted_code_point",
        address: "3.9.2",
        permission: Permission::Silent,
        locale: Rendering::English,
        evidence: "Furthermore JIS X 4051 states that it is implementation-defined how to \
                   handle characters that are not explicitly mentioned, e.g. whether they \
                   should belong to either class or not.",
        answers: &[
            Answer {
                name: "by-frame",
                address: "3.9.2",
                statement: "Furthermore JIS X 4051 states that it is implementation-defined \
                            how to handle characters that are not explicitly mentioned, e.g. \
                            whether they should belong to either class or not.",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "ideographic",
                address: "3.9.2",
                statement: "Furthermore JIS X 4051 states that it is implementation-defined \
                            how to handle characters that are not explicitly mentioned, e.g. \
                            whether they should belong to either class or not.",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "by-frame",
        jis_reading: "by-frame",
        book: "by-frame",
        magazine: "by-frame",
        newspaper: "by-frame",
    },
    Question {
        constant: "AMBIGUOUS_CONTEXT",
        path: "classification.ambiguous_context",
        address: "3.9.2",
        permission: Permission::Silent,
        locale: Rendering::English,
        evidence: "In this case, English spelling is indicated using parentheses in a \
                   Japanese line of text. In this particular case, Japanese design is better.",
        answers: &[
            Answer {
                name: "lowest-class",
                address: "3.9.2",
                statement: "In this particular case, Japanese design is better.",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "highest-class",
                address: "3.9.2",
                statement: "In this case, English spelling is indicated using parentheses in \
                            a Japanese line of text.",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "lowest-class",
        jis_reading: "lowest-class",
        book: "lowest-class",
        magazine: "lowest-class",
        newspaper: "lowest-class",
    },
    Question {
        constant: "GROUPED_NUMERAL_QUALIFICATION",
        path: "classification.grouped_numeral_qualification",
        address: "3.9.2",
        permission: Permission::Silent,
        locale: Rendering::English,
        evidence: "Sequences of European numerals which are not full-width and are handled \
                   as Japanese text, the decimal point or the comma and space used as a \
                   decimal place indicator in numbers.",
        answers: &[
            Answer {
                name: "by-width",
                address: "3.9.2",
                statement: "Sequences of European numerals which are not full-width and are \
                            handled as Japanese text, the decimal point or the comma and \
                            space used as a decimal place indicator in numbers.",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "by-role",
                address: "3.9.2",
                statement: "Sequences of European numerals which are not full-width and are \
                            handled as Japanese text, the decimal point or the comma and \
                            space used as a decimal place indicator in numbers.",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "by-width",
        jis_reading: "by-width",
        book: "by-width",
        magazine: "by-width",
        newspaper: "by-width",
    },
    Question {
        constant: "RELAXATION_MECHANISM",
        path: "kinsoku.relaxation_mechanism",
        address: "C.3",
        permission: Permission::Contradictory,
        locale: Rendering::English,
        evidence: "Breaking a line is allowed before or after the following character \
                   classes even though Table 2 prohibits it.",
        answers: &[
            Answer {
                name: "reclassify",
                // Stated at §C.2, not at §C.3: the notes of §C.2 read a relaxed character as
                // reclassified rather than as a matrix cell overridden, which is the other
                // half of the contradiction this question addresses.
                address: "C.2#1",
                statement: "the character shall be treated as a member of the ideographic \
                            character (cl-19) class",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "matrix",
                address: "C.3",
                statement: "Breaking a line is allowed before or after the following \
                            character classes even though Table 2 prohibits it.",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "reclassify",
        jis_reading: "reclassify",
        book: "reclassify",
        magazine: "reclassify",
        newspaper: "reclassify",
    },
];

/// Read the policy space out of the published rendering and the reading above.
fn read_questions(sources: &[String]) -> Result<String, String> {
    let html = only(sources)?;
    let prose = inventory::prose(html)?;
    let ordinals = inventory::rule_ordinals(html)?;
    let mut violations = Vec::new();
    check_table(&mut violations);
    for question in &POLICY_SPACE {
        check_against_document(question, &prose, &mut violations);
        check_answers_against_document(question, &prose, &ordinals, &mut violations);
    }
    if !violations.is_empty() {
        return Err(violations.join("\n  "));
    }

    let mut out = String::from(
        "question\tconstant\taddress\tpermission\tlocale\tpermits\trule\tanswer_rules\t\
         statements\tpreferred\tjlreq\tjis_reading\tbook\tmagazine\tnewspaper\texcludes\t\
         evidence\n",
    );
    out.push_str(&explanation(&[
        "Which sections state two readings is a reading of prose and not a property a",
        "scanner computes, so the rows below are written in xtask/src/policy.rs and the",
        "derivation holds each of them against the document: every `statements` entry is",
        "quoted verbatim from the `en` rendering of the address at the same position in",
        "`answer_rules`, and a row whose sentence is not there emits nothing at all",
        "(docs/adr/0009).",
        "",
        "`permission` says why the alternative is permitted, which is the distinction that",
        "must not be laundered away. `stated`: the section or note states the alternatives",
        "in so many words. `divergent`: the two renderings of one sentence do not state the",
        "same rule. `contradictory`: the document states one rule twice, in ways that are",
        "not equivalent. `silent`: the document decides nothing, and the answers are this",
        "project's. Only `stated` is a permission JLReq grants; the other three are",
        "permissions read out of it.",
        "",
        "`permits` names every answer, space separated; `answer_rules` gives each one's",
        "citing rule as the ordinal `spec/derived/rules.tsv` assigns it, pipe separated in",
        "the same order; `statements` gives each one's sentence the same way. `preferred`",
        "names the one answer of `permits` JLReq calls preferred, or `none`.",
        "",
        "`jlreq` is the answer Policy::JLREQ selects: JLReq's own preference where it states",
        "one, this project's published reading where JLReq states nothing, and failing both",
        "the reading the specification states first. `jis_reading`, `book`, `magazine` and",
        "`newspaper` are the further four presets; each is `jlreq`'s own answer wherever",
        "this project found no divergence to record for that practice.",
        "",
        "`excludes` lists, for the answers that exclude another question's answer, entries",
        "of the form `owner|question|answer|rule`, semicolon separated. `rule` is the",
        "excluding rule's ordinal in `spec/derived/rules.tsv`. The relation is symmetric and",
        "is recorded once, on whichever answer states it.",
    ]));
    for question in &POLICY_SPACE {
        out.push_str(&row(&fields(question, &ordinals))?);
    }
    Ok(out)
}

/// The one source this derivation reads.
fn only(sources: &[String]) -> Result<&str, String> {
    match sources {
        [one] => Ok(one.as_str()),
        _ => Err(format!(
            "this derivation reads {SNAPSHOT} and nothing else; it was handed {count} \
             source(s)",
            count = sources.len()
        )),
    }
}

/// A comment block, written under the column line rather than above it.
///
/// `conform` takes the first line it does not skip as the header, so a comment block above
/// the column line would hand it the column line itself as a question path.
fn explanation(lines: &[&str]) -> String {
    let mut text = String::new();
    for line in lines {
        text.push('#');
        if !line.is_empty() {
            text.push(' ');
            text.push_str(line);
        }
        text.push('\n');
    }
    text
}

/// One row's fields, in the order the header states them.
///
/// Every ordinal is resolved from `ordinals` rather than trusted, so a row is built only
/// once every citing address the question and its answers name has been held against
/// `check_answers_against_document`'s pass over the same map — this function runs after that
/// pass has already refused to emit on a miss, so `resolve` here never meets one it has to
/// report.
fn fields(question: &Question, ordinals: &BTreeMap<String, u16>) -> [String; 17] {
    let permits = question
        .answers
        .iter()
        .map(|answer| answer.name)
        .collect::<Vec<&str>>()
        .join(" ");
    let answer_rules = question
        .answers
        .iter()
        .map(|answer| resolve(answer.address, ordinals).to_string())
        .collect::<Vec<String>>()
        .join("|");
    let statements = question
        .answers
        .iter()
        .map(|answer| answer.statement)
        .collect::<Vec<&str>>()
        .join("|");
    let preferred = question
        .answers
        .iter()
        .find(|answer| answer.preferred)
        .map_or("none", |answer| answer.name);
    let excludes = question
        .answers
        .iter()
        .flat_map(|answer| {
            answer.excludes.iter().map(move |exclusion| {
                format!(
                    "{owner}|{question}|{answer}|{rule}",
                    owner = answer.name,
                    question = exclusion.question,
                    answer = exclusion.answer,
                    rule = resolve(exclusion.address, ordinals)
                )
            })
        })
        .collect::<Vec<String>>()
        .join(";");
    [
        question.path.to_owned(),
        question.constant.to_owned(),
        question.address.to_owned(),
        question.permission.token().to_owned(),
        question.locale.tag().to_owned(),
        permits,
        resolve(question.address, ordinals).to_string(),
        answer_rules,
        statements,
        preferred.to_owned(),
        question.jlreq.to_owned(),
        question.jis_reading.to_owned(),
        question.book.to_owned(),
        question.magazine.to_owned(),
        question.newspaper.to_owned(),
        excludes,
        question.evidence.to_owned(),
    ]
}

/// The ordinal an address resolves to, or `u16::MAX` when it does not.
///
/// `u16::MAX` never names a real rule: `crates/jlreq-spec/src/rule.rs` asserts the inventory
/// holds no more than `u16::MAX` rows, which bounds every real ordinal below it. Reached only
/// after `check_answers_against_document` has already refused to emit over a miss, so this is
/// a total reading of the map rather than a second place a miss could surface.
fn resolve(address: &str, ordinals: &BTreeMap<String, u16>) -> u16 {
    ordinals.get(address).copied().unwrap_or(u16::MAX)
}

/// One row: the fields, tab separated.
///
/// Every field here was written by a person rather than extracted, so a tab or a newline in
/// one is a mistake that would silently become two fields or two rows. It is refused, and
/// the sentence naming the field is what a writer needs to fix it.
fn row(fields: &[String]) -> Result<String, String> {
    if let Some(broken) = fields
        .iter()
        .find(|field| field.contains('\t') || field.contains('\n'))
    {
        return Err(format!(
            "`{broken}` holds a tab or a newline, and a tab-separated field is one line"
        ));
    }
    let mut line = fields.join("\t");
    line.push('\n');
    Ok(line)
}

/// Everything about the reading that is decidable without the document.
///
/// The count, the two identifier vocabularies, and the shape of an answer set. A question
/// permitting one answer is not a question, and a preset naming an answer the question does
/// not permit is a preset the generated table could not compile against (`Policy::get` is
/// total exactly because every preset names an answer that exists).
fn check_table(violations: &mut Vec<String>) {
    let mut paths: BTreeSet<&str> = BTreeSet::new();
    let mut constants: BTreeSet<&str> = BTreeSet::new();
    for question in &POLICY_SPACE {
        let at = question.constant;
        if !is_path(question.path) {
            violations.push(format!(
                "{at}: `{path}` is not a question path; a path is dotted, and every segment \
                 is lower-case ASCII letters or underscores",
                path = question.path
            ));
        }
        if !is_constant(question.constant) {
            violations.push(format!(
                "`{at}` is not the name of a `Question` constant; a constant is upper-case \
                 ASCII letters, digits and underscores"
            ));
        }
        if !paths.insert(question.path) {
            violations.push(format!(
                "{at}: two questions carry the path `{path}`, so a case file naming it names \
                 both",
                path = question.path
            ));
        }
        if !constants.insert(question.constant) {
            violations.push(format!("`{at}` names two questions"));
        }
        check_answers(question, violations);
    }
    check_excludes(violations);
}

/// One question's answer set: at least two, each named once, and every preset naming one of
/// them.
fn check_answers(question: &Question, violations: &mut Vec<String>) {
    let at = question.constant;
    let mut named: BTreeSet<&str> = BTreeSet::new();
    let mut preferred = 0usize;
    for answer in question.answers {
        if !is_answer(answer.name) {
            violations.push(format!(
                "{at}: `{name}` is not the name of an answer; a name is lower-case ASCII \
                 letters, digits and hyphens",
                name = answer.name
            ));
        }
        if !named.insert(answer.name) {
            violations.push(format!("{at}: permits `{name}` twice", name = answer.name));
        }
        if answer.statement.trim().is_empty() {
            violations.push(format!(
                "{at}: `{name}` rests on no sentence",
                name = answer.name
            ));
        }
        if answer.preferred {
            preferred = preferred.saturating_add(1);
        }
    }
    if question.answers.len() < LEAST_ANSWERS {
        violations.push(format!(
            "{at}: permits {count} answer(s); a place with one answer is not a place JLReq \
             permits more than one",
            count = question.answers.len()
        ));
    }
    if preferred > 1 {
        violations.push(format!(
            "{at}: calls {preferred} answers preferred; JLReq states one preference per \
             question or none"
        ));
    }
    for (preset, name) in [
        ("jlreq", question.jlreq),
        ("jis_reading", question.jis_reading),
        ("book", question.book),
        ("magazine", question.magazine),
        ("newspaper", question.newspaper),
    ] {
        if !named.contains(name) {
            violations.push(format!(
                "{at}: Policy::{preset} selects `{name}`, which this question does not \
                 permit",
                preset = preset.to_uppercase()
            ));
        }
    }
}

/// Every exclusion names a question and an answer among the twenty-two, and never its own
/// question.
fn check_excludes(violations: &mut Vec<String>) {
    for question in &POLICY_SPACE {
        for answer in question.answers {
            for exclusion in answer.excludes {
                if exclusion.question == question.path {
                    violations.push(format!(
                        "{at}: `{name}` excludes an answer of its own question; only one \
                         answer of a question is ever in force at a time, so the relation is \
                         between two different questions",
                        at = question.constant,
                        name = answer.name
                    ));
                    continue;
                }
                let Some(other) = POLICY_SPACE
                    .iter()
                    .find(|candidate| candidate.path == exclusion.question)
                else {
                    violations.push(format!(
                        "{at}: `{name}` excludes `{question}`, which is not one of the \
                         twenty-two questions",
                        at = question.constant,
                        name = answer.name,
                        question = exclusion.question
                    ));
                    continue;
                };
                if !other
                    .answers
                    .iter()
                    .any(|candidate| candidate.name == exclusion.answer)
                {
                    violations.push(format!(
                        "{at}: `{name}` excludes `{question}` = `{excluded}`, which \
                         `{question}` does not permit",
                        at = question.constant,
                        name = answer.name,
                        question = exclusion.question,
                        excluded = exclusion.answer
                    ));
                }
            }
        }
    }
}

/// Hold one question's own evidence against the published document.
///
/// This is the control the module documentation describes: the address must be a section or
/// a note the document has, and the sentence must be verbatim in the rendering the row
/// names. A `divergent` row additionally has to still diverge.
fn check_against_document(
    question: &Question,
    prose: &BTreeMap<String, [String; 2]>,
    violations: &mut Vec<String>,
) {
    let at = question.constant;
    let Some(text) = prose.get(question.address) else {
        violations.push(format!(
            "{at}: §{address} is not a section or a note the published document states in \
             its own words, so nothing there permits an alternative",
            address = question.address
        ));
        return;
    };
    if question.evidence.trim().is_empty() {
        violations.push(format!(
            "{at}: quotes no sentence; a permitted alternative rests on something the \
             document says"
        ));
        return;
    }
    let Some(rendering) = text.get(question.locale.column()) else {
        return;
    };
    if !rendering.contains(question.evidence) {
        violations.push(format!(
            "{at}: the sentence it quotes is not in the `{tag}` rendering of §{address}; the \
             reading names the document and the document is what decides",
            tag = question.locale.tag(),
            address = question.address
        ));
    }
    if question.permission == Permission::Divergent {
        check_divergence(question, text, violations);
    }
}

/// Hold every answer's statement, and its citing address, against the published document.
///
/// Independent of `check_against_document`: an answer's citing address need not be the
/// question's own — `kinsoku.relaxation_mechanism`'s `reclassify` is the one place among
/// these twenty-two it is not — so each is resolved and checked on its own.
fn check_answers_against_document(
    question: &Question,
    prose: &BTreeMap<String, [String; 2]>,
    ordinals: &BTreeMap<String, u16>,
    violations: &mut Vec<String>,
) {
    let at = question.constant;
    for answer in question.answers {
        if !ordinals.contains_key(answer.address) {
            violations.push(format!(
                "{at}: `{name}` cites §{address}, which the rule inventory does not \
                 address; a citing address is one `spec/derived/rules.tsv` also holds a row \
                 for",
                name = answer.name,
                address = answer.address
            ));
        }
        let Some(text) = prose.get(answer.address) else {
            violations.push(format!(
                "{at}: `{name}` cites §{address}, which is not a section or a note the \
                 published document states in its own words",
                name = answer.name,
                address = answer.address
            ));
            continue;
        };
        let Some(rendering) = text.get(Rendering::English.column()) else {
            continue;
        };
        if !rendering.contains(answer.statement) {
            violations.push(format!(
                "{at}: `{name}`'s sentence is not in the `en` rendering of §{address}; the \
                 reading names the document and the document is what decides",
                name = answer.name,
                address = answer.address
            ));
        }
    }
    for exclusion in question.answers.iter().flat_map(|answer| answer.excludes) {
        if !ordinals.contains_key(exclusion.address) {
            violations.push(format!(
                "{at}: an exclusion cites §{address}, which the rule inventory does not \
                 address",
                address = exclusion.address
            ));
        }
    }
}

/// A `divergent` row still diverges.
///
/// A divergence between the two renderings of one sentence is visible as a difference
/// between the character classes they cite, so if a revision brought them into
/// correspondence the permission would be gone, and this is what refuses to keep publishing
/// it — the same reason `attest` requires the detected defects to equal the recorded ones.
///
/// No row carries `divergent` today. §B.2 note 7 did until the row above was read again: its
/// English half states all four answers in so many words, which makes the permission one
/// JLReq grants and the locale difference beside it a defect of the document rather than the
/// reason the alternatives exist. The value stays in the vocabulary, because that vocabulary
/// is published in the file's own header and is the thing that stops the three permissions
/// this project reads *out of* the document being laundered into the one it grants; this is
/// the proof the next row recorded as a divergence has to survive.
fn check_divergence(question: &Question, text: &[String; 2], violations: &mut Vec<String>) {
    let named = |rendering: Rendering| -> BTreeSet<&str> {
        text.get(rendering.column())
            .map(|one| classes_named(one))
            .unwrap_or_default()
    };
    let mine = named(question.locale);
    let theirs = named(question.locale.other());
    if mine == theirs {
        violations.push(format!(
            "{at}: is recorded as an English/Japanese divergence, and the two renderings of \
             §{address} now name the same character classes, so nothing there permits an \
             alternative",
            at = question.constant,
            address = question.address
        ));
    }
}

/// Every character class one rendering cites, written the way the document writes them.
fn classes_named(text: &str) -> BTreeSet<&str> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(at) = rest.find("cl-") {
        let from = rest.get(at..).unwrap_or("");
        let digits = from.get("cl-".len()..).map_or(0, |tail| {
            tail.chars().take_while(char::is_ascii_digit).count()
        });
        if digits > 0 {
            if let Some(name) = from.get(.."cl-".len().saturating_add(digits)) {
                found.insert(name);
            }
        }
        rest = from.get("cl-".len()..).unwrap_or("");
    }
    found
}

/// Whether a string is a stable dotted policy path, as `Question::path` renders one.
fn is_path(text: &str) -> bool {
    text.contains('.')
        && text.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|character| character.is_ascii_lowercase() || character == '_')
        })
}

/// Whether a string is the name of a `Question` constant.
fn is_constant(text: &str) -> bool {
    !text.is_empty()
        && text.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        })
}

/// Whether a string is the name of one permitted answer, as `Choice::name` renders one.
fn is_answer(text: &str) -> bool {
    !text.is_empty()
        && text.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

// ---------------------------------------------------------------------------------------
// Stage 2: the generation unit
// ---------------------------------------------------------------------------------------

/// The policy space, as `jlreq-spec` indexes it.
pub(crate) const POLICY_SPACE_UNIT: Unit = Unit {
    input: QUESTIONS_FILE,
    generator: &["xtask/src/policy.rs"],
    output: "crates/jlreq-spec/src/generated/policy.rs",
    summary: "The policy space: one row and one named `Question` identifier per place JLReq \
              permits more than one answer.",
    emit: emit_policy,
};

/// The columns this generator reads, in the order `read_questions` writes them.
const COLUMNS: [&str; 17] = [
    "question",
    "constant",
    "address",
    "permission",
    "locale",
    "permits",
    "rule",
    "answer_rules",
    "statements",
    "preferred",
    "jlreq",
    "jis_reading",
    "book",
    "magazine",
    "newspaper",
    "excludes",
    "evidence",
];

/// Which preset column a practice occupies, as `jlreq_spec::policy::Preset::column` fixes it.
///
/// This order is the contract between this crate and the generator: emitting the five
/// presets in any other order would compile and answer the wrong practice for every named
/// constant beside `Policy::JLREQ`.
const PRESET_COLUMNS: [&str; 5] = ["jlreq", "jis_reading", "book", "magazine", "newspaper"];

/// One row of the policy space, read out of the derived table.
#[derive(Debug)]
struct Row {
    /// The stable dotted path.
    path: String,
    /// The section or note that permits the alternative, for the doc comment on the named
    /// `Question` constant. Not `rule`: a reader citing the constant wants the specification's
    /// own spelling, and an ordinal into a table that regenerates on every revision is not it.
    address: String,
    /// The `Question` constant this row is named by.
    constant: String,
    /// Every permitted answer, in the order the specification states them.
    answers: Vec<RowAnswer>,
    /// This question's own citing rule.
    rule: u16,
    /// Which answer each of the five presets selects, indexed by [`PRESET_COLUMNS`]'s order.
    presets: [u8; 5],
    /// The `excludes` field, unresolved: one `(owner, question, answer, rule)` tuple per
    /// entry. Resolved into each `RowAnswer::excludes` by [`resolve_excludes`] once every
    /// row's own answers are known, because an entry may name a question later in the file.
    raw_excludes: Vec<(String, String, String, String)>,
}

/// One answer of one row.
#[derive(Debug)]
struct RowAnswer {
    /// The stable name.
    name: String,
    /// The sentence it rests on.
    statement: String,
    /// The rule that states it.
    rule: u16,
    /// Whether JLReq calls it preferred.
    preferred: bool,
    /// The answers of other questions it excludes, resolved to a position in `QUESTIONS` and
    /// an index into that question's own answers.
    excludes: Vec<RowExclusion>,
}

/// One resolved exclusion.
#[derive(Debug)]
struct RowExclusion {
    /// The excluded question's position in `QUESTIONS`.
    question: usize,
    /// The excluded answer's index into that question's `choices`.
    answer: u8,
    /// The rule stating the exclusion.
    rule: u16,
}

/// Emit the policy space.
fn emit_policy(table: &Table) -> Result<Emission, String> {
    if table.columns != COLUMNS {
        return Err(format!(
            "names the columns {found:?} where this generator reads {COLUMNS:?}",
            found = table.columns
        ));
    }
    let rows = read_rows(table)?;
    let mut items = String::new();
    items.push_str(&policy_rows(&rows));
    items.push_str(&policy_identifiers(&rows));
    Ok(Emission {
        entries: rows.len(),
        items,
    })
}

/// Read every row, resolving each exclusion against the full set so that a forward reference
/// — an earlier question excluding a later one's answer — resolves the same as a backward
/// one.
fn read_rows(table: &Table) -> Result<Vec<Row>, String> {
    let mut rows = Vec::new();
    for record in &table.records {
        rows.push(read_row(record)?);
    }
    if rows.len() != QUESTION_COUNT {
        return Err(format!(
            "holds {found} row(s) where this repository was written against {QUESTION_COUNT}",
            found = rows.len()
        ));
    }
    resolve_excludes(&mut rows)?;
    Ok(rows)
}

/// Read one row, refusing everything the emitted table could not state.
fn read_row(record: &Record) -> Result<Row, String> {
    let path = field(record, "question")?.to_owned();
    let address = field(record, "address")?.to_owned();
    let constant = field(record, "constant")?.to_owned();
    let permits: Vec<&str> = field(record, "permits")?.split_whitespace().collect();
    let rule: Vec<&str> = field(record, "rule")?.split('|').collect();
    let answer_rules: Vec<&str> = field(record, "answer_rules")?.split('|').collect();
    let statements: Vec<&str> = field(record, "statements")?.split('|').collect();
    let preferred = field(record, "preferred")?;

    if answer_rules.len() != permits.len() || statements.len() != permits.len() {
        return Err(at(
            record,
            &format!(
                "`{path}` names {names} answer(s), {rules} rule(s) and {statements} \
                 sentence(s); the three columns are one entry per answer",
                names = permits.len(),
                rules = answer_rules.len(),
                statements = statements.len()
            ),
        ));
    }
    let [rule] = rule.as_slice() else {
        return Err(at(
            record,
            &format!("`{path}`'s own `rule` names more than one ordinal"),
        ));
    };
    let rule = ordinal(record, rule)?;

    let mut answers = Vec::new();
    for index in 0..permits.len() {
        let name = permits[index].to_owned();
        answers.push(RowAnswer {
            preferred: preferred == name,
            name,
            statement: statements[index].to_owned(),
            rule: ordinal(record, answer_rules[index])?,
            excludes: Vec::new(),
        });
    }
    if preferred != "none" && !answers.iter().any(|answer| answer.preferred) {
        return Err(at(
            record,
            &format!("`{path}` calls `{preferred}` preferred, which it does not permit"),
        ));
    }

    let mut presets = [0u8; PRESET_COLUMNS.len()];
    for (column, name) in PRESET_COLUMNS.iter().enumerate() {
        presets[column] = preset_index(record, &path, &answers, field(record, name)?)?;
    }

    let mut raw_excludes = Vec::new();
    for entry in field(record, "excludes")?
        .split(';')
        .filter(|entry| !entry.is_empty())
    {
        let fields: Vec<&str> = entry.split('|').collect();
        let [owner, question, answer, rule] = fields.as_slice() else {
            return Err(at(
                record,
                &format!("`{entry}` is not an `owner|question|answer|rule` exclusion"),
            ));
        };
        raw_excludes.push((
            (*owner).to_owned(),
            (*question).to_owned(),
            (*answer).to_owned(),
            (*rule).to_owned(),
        ));
    }

    Ok(Row {
        path,
        address,
        constant,
        answers,
        rule,
        presets,
        raw_excludes,
    })
}

/// The index of the answer named `name` permits, or a rejection naming the answer set.
fn preset_index(
    record: &Record,
    path: &str,
    answers: &[RowAnswer],
    name: &str,
) -> Result<u8, String> {
    answers
        .iter()
        .position(|answer| answer.name == name)
        .and_then(|index| u8::try_from(index).ok())
        .ok_or_else(|| {
            at(
                record,
                &format!("`{path}` selects `{name}`, which it does not permit as a preset"),
            )
        })
}

/// Parse one `|`-joined ordinal field.
fn ordinal(record: &Record, text: &str) -> Result<u16, String> {
    text.parse::<u16>()
        .map_err(|_| at(record, &format!("`{text}` is not a rule ordinal")))
}

/// Resolve every row's [`Row::raw_excludes`] against the full row set, folding each entry
/// onto the [`RowAnswer`] that owns it.
///
/// A separate pass from [`read_row`] because an entry may name a question the file states
/// later — `kinsoku.level`'s `very-strict` excludes `kinsoku.relaxation_mechanism`'s
/// `reclassify`, and the two are twenty rows apart — so resolution needs every row's answer
/// names known before any one entry can be resolved.
fn resolve_excludes(rows: &mut [Row]) -> Result<(), String> {
    let mut resolved: Vec<Vec<(usize, RowExclusion)>> = Vec::new();
    for record_index in 0..rows.len() {
        let path = rows[record_index].path.clone();
        let mut per_row = Vec::new();
        for (owner, question, answer, rule) in &rows[record_index].raw_excludes {
            let owner_index = rows[record_index]
                .answers
                .iter()
                .position(|candidate| candidate.name == *owner)
                .ok_or_else(|| {
                    format!("{path}: excludes from `{owner}`, which it does not permit")
                })?;
            let question_index = rows
                .iter()
                .position(|candidate| candidate.path == *question)
                .ok_or_else(|| format!("{path}: excludes `{question}`, which is not a question"))?;
            let answer_index = rows[question_index]
                .answers
                .iter()
                .position(|candidate| candidate.name == *answer)
                .and_then(|index| u8::try_from(index).ok())
                .ok_or_else(|| {
                    format!("{path}: excludes `{question}` = `{answer}`, which it does not permit")
                })?;
            let rule = rule
                .parse::<u16>()
                .map_err(|_| format!("{path}: `{rule}` is not a rule ordinal"))?;
            per_row.push((
                owner_index,
                RowExclusion {
                    question: question_index,
                    answer: answer_index,
                    rule,
                },
            ));
        }
        resolved.push(per_row);
    }
    for (record_index, entries) in resolved.into_iter().enumerate() {
        for (owner_index, exclusion) in entries {
            rows[record_index].answers[owner_index]
                .excludes
                .push(exclusion);
        }
    }
    Ok(())
}

/// One field of a record, found by column name.
fn field<'a>(record: &'a Record, name: &str) -> Result<&'a str, String> {
    let index = COLUMNS
        .iter()
        .position(|column| *column == name)
        .unwrap_or(usize::MAX);
    record
        .fields
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| at(record, &format!("has no `{name}` field")))
}

/// A rejection, with the line it belongs to.
fn at(record: &Record, reason: &str) -> String {
    format!("line {line}: {reason}", line = record.line)
}

/// The rows of the emitted policy space.
fn policy_rows(rows: &[Row]) -> String {
    let written: Vec<String> = rows
        .iter()
        .map(|row| {
            let choices: Vec<String> = row.answers.iter().map(choice_of).collect();
            format!(
                "    QuestionRecord {{\n\
                 \x20       path: {path},\n\
                 \x20       rule: RuleId({rule}),\n\
                 \x20       choices: &[\n{choices}\x20       ],\n\
                 \x20       presets: {presets:?},\n\
                 \x20   }},\n",
                path = literal(&row.path),
                rule = row.rule,
                choices = choices.concat(),
                presets = row.presets,
            )
        })
        .collect();
    format!("{HEAD}{body}];\n\n", body = written.concat())
}

/// How many spaces a `ChoiceRecord` sits at inside a `QuestionRecord`'s `choices`, and every
/// deeper level this function's output reaches, each four more than its parent — the nesting
/// `rustfmt` produces for a struct literal that is itself an array element.
const CHOICE_INDENT: usize = 12;
/// One level deeper: a `ChoiceRecord`'s own fields.
const CHOICE_FIELD_INDENT: usize = CHOICE_INDENT + 4;
/// One level deeper again: an `Exclusion` inside a non-empty `excludes` array.
const EXCLUSION_INDENT: usize = CHOICE_FIELD_INDENT + 4;
/// The deepest level this function reaches: an `Exclusion`'s own fields.
const EXCLUSION_FIELD_INDENT: usize = EXCLUSION_INDENT + 4;

/// One `ChoiceRecord`, indented to sit inside a `QuestionRecord`'s `choices`.
///
/// Built by concatenation and a computed indent rather than a single `\`-continued string
/// literal: that continuation strips every leading space of the line that follows, so the
/// fixed indent of a nested struct literal cannot be written as raw spaces in the template
/// the way the surrounding rows are — the count above is the deliberate replacement, checked
/// against `rustfmt`'s own opinion by `generate --check` on every run.
fn choice_of(answer: &RowAnswer) -> String {
    let excludes = if answer.excludes.is_empty() {
        "&[]".to_owned()
    } else {
        let mut body = String::new();
        for exclusion in &answer.excludes {
            // `write!` into the `String` that already exists, rather than a `format!` that
            // allocates a second one just to be appended: the loop above owns `body` for
            // exactly this reason. The result is discarded with `let _ =` rather than
            // `unwrap`/`expect` — both denied workspace-wide by `Cargo.toml`'s
            // `[workspace.lints]` — because `fmt::Write` for `String` never fails.
            let _ = write!(
                body,
                "{at}Exclusion {{\n\
                 {field}question: Question({question}),\n\
                 {field}choice: {choice},\n\
                 {field}rule: RuleId({rule}),\n\
                 {at}}},\n",
                at = " ".repeat(EXCLUSION_INDENT),
                field = " ".repeat(EXCLUSION_FIELD_INDENT),
                question = exclusion.question,
                choice = exclusion.answer,
                rule = exclusion.rule,
            );
        }
        format!(
            "&[\n{body}{close}]",
            close = " ".repeat(CHOICE_FIELD_INDENT)
        )
    };
    format!(
        "{at}ChoiceRecord {{\n\
         {field}name: {name},\n\
         {field}statement: {statement},\n\
         {field}rule: RuleId({rule}),\n\
         {field}preferred: {preferred},\n\
         {field}excludes: {excludes},\n\
         {at}}},\n",
        at = " ".repeat(CHOICE_INDENT),
        field = " ".repeat(CHOICE_FIELD_INDENT),
        name = literal(&answer.name),
        statement = literal(&answer.statement),
        rule = answer.rule,
        preferred = answer.preferred,
    )
}

/// What the emitted table says about itself, above its first row.
const HEAD: &str = "use crate::policy::{ChoiceRecord, Exclusion, Question, QuestionRecord};\n\
                    use crate::rule::RuleId;\n\
                    \n\
                    /// Every place JLReq permits more than one answer, in the specification's\n\
                    /// own reading order.\n\
                    ///\n\
                    /// JLReq: \u{a7}B.2, \u{a7}C.2, \u{a7}C.3, \u{a7}D, \u{a7}E.2\n\
                    pub(crate) const QUESTIONS: &[QuestionRecord] = &[\n";

/// The named identifier of every question.
fn policy_identifiers(rows: &[Row]) -> String {
    let written: Vec<String> = rows
        .iter()
        .enumerate()
        .map(|(ordinal, row)| {
            format!(
                "{gap}    /// The place at `{path}`.\n\
                 \x20   ///\n\
                 \x20   /// JLReq: \u{a7}{address}\n\
                 {declaration}",
                gap = if ordinal == 0 { "" } else { "\n" },
                path = row.path,
                address = row.address,
                declaration = declaration(&row.constant, ordinal),
            )
        })
        .collect();
    format!("impl Question {{\n{body}}}\n", body = written.concat())
}

/// One named identifier, wrapped where `rustfmt` would wrap it.
const MAX_WIDTH: usize = 100;

/// One named `Question` constant, wrapped the way `rustfmt` would leave it.
fn declaration(name: &str, ordinal: usize) -> String {
    let one_line = format!("    pub const {name}: Self = Self({ordinal});\n");
    if one_line.trim_end().chars().count() <= MAX_WIDTH {
        return one_line;
    }
    format!("    pub const {name}: Self =\n        Self({ordinal});\n")
}

/// One Rust string literal holding `text`.
fn literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len().saturating_add(2));
    out.push('"');
    for character in text.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        Answer, POLICY_SPACE, Permission, QUESTION_COUNT, QUESTIONS, Question, Rendering,
        check_against_document, check_answers, check_answers_against_document, check_table,
        classes_named, is_answer, is_constant, is_path, row,
    };

    /// A document holding one section, in both renderings.
    fn document(address: &str, english: &str, japanese: &str) -> BTreeMap<String, [String; 2]> {
        let mut prose = BTreeMap::new();
        prose.insert(
            address.to_owned(),
            [english.to_owned(), japanese.to_owned()],
        );
        prose
    }

    /// A question of the shape §C.3 has, over a fixture document.
    const FIXTURE: Question = Question {
        constant: "KINSOKU_LEVEL",
        path: "kinsoku.level",
        address: "C.3",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "four levels of convention",
        answers: &[
            Answer {
                name: "strict",
                address: "C.3",
                statement: "four levels of convention",
                preferred: false,
                excludes: &[],
            },
            Answer {
                name: "very-strict",
                address: "C.3",
                statement: "four levels of convention",
                preferred: false,
                excludes: &[],
            },
        ],
        jlreq: "strict",
        jis_reading: "strict",
        book: "strict",
        magazine: "strict",
        newspaper: "strict",
    };

    #[test]
    fn the_committed_reading_is_sound_without_the_document() {
        let mut violations = Vec::new();
        check_table(&mut violations);
        assert!(
            violations.is_empty(),
            "the policy space is malformed: {violations:?}"
        );
    }

    #[test]
    fn every_question_names_a_distinct_constant_and_path() {
        let constants: BTreeSet<&str> = POLICY_SPACE.iter().map(|one| one.constant).collect();
        let paths: BTreeSet<&str> = POLICY_SPACE.iter().map(|one| one.path).collect();
        assert_eq!(constants.len(), QUESTION_COUNT);
        assert_eq!(paths.len(), QUESTION_COUNT);
    }

    #[test]
    fn the_derivation_reads_the_snapshot_and_writes_the_policy_space() {
        assert_eq!(QUESTIONS.output, "spec/derived/questions.tsv");
        assert_eq!(QUESTIONS.sources, &["spec/snapshot/index.html"]);
        assert!(
            QUESTIONS.reader.contains(&"xtask/src/policy.rs"),
            "the reading lives in this module, so this module's digest is what the emitted \
             file states it was read by"
        );
        assert!(
            QUESTIONS.reader.contains(&"xtask/src/inventory.rs"),
            "the sentences are extracted by the inventory's scanner, so its digest decides \
             these bytes too"
        );
    }

    #[test]
    fn a_sentence_the_document_does_not_state_emits_nothing() {
        let prose = document("C.3", "The following lists four levels of convention.", "");
        let mut violations = Vec::new();
        check_against_document(&FIXTURE, &prose, &mut violations);
        assert!(violations.is_empty(), "{violations:?}");

        let absent = Question {
            evidence: "The following lists five levels of convention.",
            ..FIXTURE
        };
        let mut violations = Vec::new();
        check_against_document(&absent, &prose, &mut violations);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(violations[0].contains("is not in the `en` rendering"));
    }

    #[test]
    fn a_question_addressing_a_section_the_document_does_not_have_emits_nothing() {
        let prose = document("C.4", "", "");
        let mut violations = Vec::new();
        check_against_document(&FIXTURE, &prose, &mut violations);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(violations[0].contains("not a section or a note"));
    }

    #[test]
    fn an_answers_sentence_absent_from_its_own_address_emits_nothing() {
        let prose = document("C.3", "four levels of convention", "");
        let ordinals = BTreeMap::from([("C.3".to_owned(), 0u16)]);
        let mut violations = Vec::new();
        check_answers_against_document(&FIXTURE, &prose, &ordinals, &mut violations);
        assert!(violations.is_empty(), "{violations:?}");

        let wrong = Question {
            answers: &[Answer {
                name: "strict",
                address: "C.3",
                statement: "five levels of convention",
                preferred: false,
                excludes: &[],
            }],
            ..FIXTURE
        };
        let mut violations = Vec::new();
        check_answers_against_document(&wrong, &prose, &ordinals, &mut violations);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(violations[0].contains("is not in the `en` rendering"));
    }

    #[test]
    fn an_answer_citing_an_address_the_rule_inventory_does_not_hold_is_refused() {
        let prose = document("C.3", "four levels of convention", "");
        let ordinals: BTreeMap<String, u16> = BTreeMap::new();
        let mut violations = Vec::new();
        check_answers_against_document(&FIXTURE, &prose, &ordinals, &mut violations);
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("the rule inventory does not address")),
            "{violations:?}"
        );
    }

    #[test]
    fn a_preset_naming_an_answer_the_question_does_not_permit_is_refused() {
        let wrong = Question {
            jlreq: "loose",
            ..FIXTURE
        };
        let mut violations = Vec::new();
        check_answers(&wrong, &mut violations);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(violations[0].contains("which this question does not permit"));
    }

    #[test]
    fn a_place_with_one_answer_is_not_a_question() {
        let single = Question {
            answers: &[Answer {
                name: "strict",
                address: "C.3",
                statement: "four levels of convention",
                preferred: false,
                excludes: &[],
            }],
            ..FIXTURE
        };
        let mut violations = Vec::new();
        check_answers(&single, &mut violations);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(violations[0].contains("is not a place JLReq permits more than one"));
    }

    #[test]
    fn two_answers_of_one_question_calling_themselves_preferred_is_refused() {
        let both = Question {
            answers: &[
                Answer {
                    name: "strict",
                    address: "C.3",
                    statement: "four levels of convention",
                    preferred: true,
                    excludes: &[],
                },
                Answer {
                    name: "very-strict",
                    address: "C.3",
                    statement: "four levels of convention",
                    preferred: true,
                    excludes: &[],
                },
            ],
            ..FIXTURE
        };
        let mut violations = Vec::new();
        check_answers(&both, &mut violations);
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("calls 2 answers preferred")),
            "{violations:?}"
        );
    }

    #[test]
    fn the_class_tokens_of_a_rendering_are_read_whole() {
        let found = classes_named("katakana (cl-16), cl-10 and 小書きの仮名（cl-11）, cl-16 again");
        assert_eq!(
            found.into_iter().collect::<Vec<&str>>(),
            vec!["cl-10", "cl-11", "cl-16"]
        );
        assert!(classes_named("cl- and clause").is_empty());
    }

    #[test]
    fn the_two_identifier_vocabularies_are_separate() {
        assert!(is_path("spacing.line_end_punctuation"));
        assert!(!is_path("line_end_punctuation"), "a path is dotted");
        assert!(!is_path("spacing.lineEnd"));
        assert!(!is_path("spacing."));
        assert!(is_constant("LINE_END_PUNCTUATION"));
        assert!(!is_constant("line_end_punctuation"));
        assert!(is_answer("very-strict"));
        assert!(is_answer("table-3"));
        assert!(!is_answer("Very-Strict"));
        assert!(!is_answer(""));
    }

    #[test]
    fn a_field_holding_a_tab_is_refused() {
        assert!(row(&["one".to_owned(), "two".to_owned()]).is_ok());
        assert!(row(&["one\ttwo".to_owned()]).is_err());
        assert!(row(&["one\ntwo".to_owned()]).is_err());
    }
}
