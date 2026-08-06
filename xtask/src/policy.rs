// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The policy space: one derivation of the `derive` gate.
//!
//! `crate::derive` is stage 1 of the specification data pipeline and this module is one
//! entry in its registry, reading the ReSpec-rendered snapshot vendored at
//! `spec/snapshot/index.html`:
//!
//! - `spec/derived/questions.tsv` — every place JLReq permits more than one answer, with
//!   the address that permits it, the answers, the answer `Policy::JLREQ` selects, and the
//!   sentence the permission rests on.
//!
//! Everything else about a derived file — that it is not to be edited, which sources it was
//! read from and their digests, byte identity on a second run — is the frame's, so this
//! module is the reading and nothing else.
//!
//! # Why the membership is written here and not scanned
//!
//! A question is "a section that states two readings", which is a reading of prose rather
//! than a property a scanner computes, and a heuristic would publish permitted alternatives
//! the specification does not permit. Three of the twenty-one rows below show why no scan
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
//! that sentence is verbatim in that section or note, in that rendering. So a row is a
//! claim about the published document that fails the build when it stops being true, and a
//! revision that resolves a question fails loudly rather than leaving a permitted
//! alternative nothing supports.
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
//! `jlreq` is the answer `Policy::JLREQ` selects, under one rule applied to every row:
//! JLReq's own preference where it states one — §C.3's "Default, general publications",
//! §D's "the method adopted by this document", §B.2's "preferred" — then this project's
//! published reading in `docs/decisions/` where JLReq states nothing, and failing both the
//! reading the specification states first. Nothing is left to the emitter to invent.
//!
//! Four things this file deliberately does not carry, each because it belongs to the stage
//! that emits `crates/jlreq-spec/src/generated/policy.rs`. The sentence *each answer* rests
//! on, which is `Choice::statement`. Whether JLReq calls an answer "preferred", which is
//! `Choice::is_preferred` and which §B.2 states for seven of its notes. The four preset
//! columns beside `jlreq`, which are `Policy::JIS_READING`, `BOOK`, `MAGAZINE` and
//! `NEWSPAPER`. And the exclusions — §C.3's strictest level excluding every §C.2 alternate
//! rule is the one `Policy::with` was designed around — which are a property of an answer
//! rather than of a question and so have no cell in a table with one row per question. Each
//! arrives as further columns, which every reader of a derived table already tolerates
//! because the column is found by name.
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

use crate::derive::Derivation;
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
        "docs/design/api-spine.md publishes, the address that permits the alternative, ",
        "the permitted answers, the answer Policy::JLREQ selects, and the sentence the ",
        "permission rests on, quoted from the rendering the `locale` column names."
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
    permits: &'static [&'static str],
    /// The answer `Policy::JLREQ` selects, which is one of `permits`.
    jlreq: &'static str,
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
const QUESTION_COUNT: usize = 21;

/// The fewest answers a question permits. One answer is not a question.
const LEAST_ANSWERS: usize = 2;

/// Every place JLReq permits more than one answer, in the order `docs/design/api-spine.md`
/// publishes the constants.
///
/// Each row is a claim about the published document: that `evidence` is verbatim in the
/// `locale` rendering of `address`. `read_questions` refuses to emit unless every one of
/// them holds, so the reading below cannot drift from the specification in silence.
const POLICY_SPACE: [Question; QUESTION_COUNT] = [
    Question {
        constant: "KINSOKU_LEVEL",
        path: "kinsoku.level",
        address: "C.3",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "The following lists four levels of convention.",
        permits: &["very-loose", "loose", "strict", "very-strict"],
        jlreq: "strict",
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
        permits: &["table-3", "table-4", "table-5"],
        jlreq: "table-3",
    },
    Question {
        constant: "LINE_END_PUNCTUATION",
        path: "spacing.line_end_punctuation",
        address: "B.2#2",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "The preferred spacing between closing brackets (cl-02) and the line end \
                   is a half em. The alternative is to set solid",
        permits: &["half-em", "solid"],
        jlreq: "half-em",
    },
    Question {
        constant: "LINE_HEAD_OPENING_BRACKET",
        path: "spacing.line_head_opening_bracket",
        address: "3.1.5",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "When starting a new line with opening brackets (cl-01) there are some \
                   patterns as shown in Figure 71.",
        permits: &["pattern-1", "pattern-2", "pattern-3"],
        jlreq: "pattern-1",
    },
    Question {
        constant: "RUBY_OVERHANG_KANA",
        path: "ruby.overhang_kana",
        address: "B.2#7",
        // `stated`, not `divergent`, and the difference is the one this column exists to
        // protect. §B.2 note 7's English half states all four answers in so many words — the
        // preferred overhang over the katakana, the JIS X 4051 prohibition, and the two
        // alternatives in the sentence quoted below — so this is a permission JLReq grants
        // and the rule reaches `jlreq-spec` as `Standing::Alternative`. Recording it as
        // `divergent` mapped it to `Standing::Adjudicated`, which is what a rule the document
        // states two incompatible things about carries, and would have published this one as
        // an adjudication of ours.
        //
        // The divergence at that address is real and is a different fact: the Japanese half
        // links cl-10 and cl-11 where the English half does not, which is about the class set
        // the *preferred* answer covers rather than about why the alternatives are permitted.
        // It is recorded once, as `b2-note-7-locale-class-divergence` in
        // spec/derived/defects.tsv, whose detector measures it over §B.2's seventeen notes and
        // fails if it disappears — so the control this row used to carry is not lost, it is
        // where the fact is (ADR 0019).
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "There are alternative methods, one of which is to allow ruby text to be \
                   extended up to the size of the ruby character over any character including \
                   ideographic (cl-19) as well as hiragana (cl-15) and katakana (cl-16) \
                   characters",
        permits: &["kana", "jis", "any", "none"],
        jlreq: "kana",
    },
    Question {
        constant: "RUBY_OVERHANG_INDENT",
        path: "ruby.overhang_indent",
        address: "B.2#8",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "The alternative approach is not to allow ruby text to be extended over \
                   the line head indent.",
        permits: &["permitted", "prohibited"],
        jlreq: "permitted",
    },
    Question {
        constant: "RUBY_ALIGNMENT",
        path: "ruby.alignment",
        address: "3.3.5",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "When attaching a single hiragana (cl-15) ruby character to a single base \
                   character, there are two ways of positioning the ruby character.",
        permits: &["nakatsuki", "katatsuki"],
        jlreq: "nakatsuki",
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
        permits: &["jis", "flush"],
        jlreq: "jis",
    },
    Question {
        constant: "JUKUGO_RUBY_LAYOUT",
        path: "ruby.jukugo_layout",
        address: "3.3.7",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "The available methods include the layout as specified in JIS X 4051",
        permits: &["group", "phonetic"],
        jlreq: "group",
    },
    Question {
        constant: "ITERATION_MARK_AT_LINE_HEAD",
        path: "kinsoku.iteration_mark_at_line_head",
        address: "B.2#14",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "there are three ways to deal with this situation",
        permits: &["prohibited", "permitted", "replaced"],
        jlreq: "prohibited",
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
        permits: &["none", "hanging"],
        jlreq: "none",
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
        permits: &["breakable", "unbreakable"],
        jlreq: "breakable",
    },
    Question {
        constant: "SENTENCE_MEDIAL_DIVIDING_MARK",
        path: "spacing.sentence_medial_dividing_mark",
        address: "3.1.6",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "either add no spacing or a quarter em spacing before and after the \
                   dividing punctuation mark",
        permits: &["solid", "quarter-em"],
        jlreq: "solid",
    },
    Question {
        constant: "JAPANESE_LATIN_EXPANSION_CEILING",
        path: "adjustment.japanese_latin_expansion_ceiling",
        address: "3.8.4",
        permission: Permission::Stated,
        locale: Rendering::English,
        evidence: "is increased equally with proportional character size, up to half em \
                   spacing (or one third em spacing)",
        permits: &["half-em", "third-em", "rigid"],
        jlreq: "half-em",
    },
    Question {
        constant: "EXPANSION_ORDER",
        path: "adjustment.expansion_order",
        address: "3.8.4",
        permission: Permission::Silent,
        locale: Rendering::English,
        evidence: "In JIS X 4051, the following processing order is defined.",
        permits: &["jis", "implementation"],
        jlreq: "jis",
    },
    Question {
        constant: "ADJUSTMENT_PREFERENCE",
        path: "adjustment.preference",
        address: "C.3",
        permission: Permission::Silent,
        locale: Rendering::English,
        evidence: "the very strict rule is for the best appearance at the line head, while \
                   the strict rule is best to avoid inter-character spacing adjustment",
        permits: &["least-adjustment", "even-texture"],
        jlreq: "least-adjustment",
    },
    Question {
        constant: "REMAINDER",
        path: "adjustment.remainder",
        address: "3.8.3",
        permission: Permission::Silent,
        locale: Rendering::English,
        evidence: "The same width reduction is applied to all spaces on the target line at \
                   the same time.",
        permits: &["leading", "trailing"],
        jlreq: "leading",
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
        permits: &["by-frame", "ideographic"],
        jlreq: "by-frame",
    },
    Question {
        constant: "AMBIGUOUS_CONTEXT",
        path: "classification.ambiguous_context",
        address: "3.9.2",
        permission: Permission::Silent,
        locale: Rendering::English,
        evidence: "In this case, English spelling is indicated using parentheses in a \
                   Japanese line of text. In this particular case, Japanese design is better.",
        permits: &["lowest-class", "highest-class"],
        jlreq: "lowest-class",
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
        permits: &["by-width", "by-role"],
        jlreq: "by-width",
    },
    Question {
        constant: "RELAXATION_MECHANISM",
        path: "kinsoku.relaxation_mechanism",
        address: "C.3",
        permission: Permission::Contradictory,
        locale: Rendering::English,
        evidence: "Breaking a line is allowed before or after the following character \
                   classes even though Table 2 prohibits it.",
        permits: &["reclassify", "matrix"],
        jlreq: "reclassify",
    },
];

/// Read the policy space out of the published rendering and the reading above.
fn read_questions(sources: &[String]) -> Result<String, String> {
    let prose = inventory::prose(only(sources)?)?;
    let mut violations = Vec::new();
    check_table(&mut violations);
    for question in &POLICY_SPACE {
        check_against_document(question, &prose, &mut violations);
    }
    if !violations.is_empty() {
        return Err(violations.join("\n  "));
    }

    let mut out =
        String::from("question\tconstant\taddress\tpermission\tlocale\tpermits\tjlreq\tevidence\n");
    out.push_str(&explanation(&[
        "Which sections state two readings is a reading of prose and not a property a",
        "scanner computes, so the rows below are written in xtask/src/policy.rs and the",
        "derivation holds each of them against the document: `evidence` is quoted verbatim",
        "from the `locale` rendering of `address`, and a row whose sentence is not there",
        "emits nothing at all (docs/adr/0009).",
        "",
        "`permission` says why the alternative is permitted, which is the distinction that",
        "must not be laundered away. `stated`: the section or note states the alternatives",
        "in so many words. `divergent`: the two renderings of one sentence do not state the",
        "same rule. `contradictory`: the document states one rule twice, in ways that are",
        "not equivalent. `silent`: the document decides nothing, and the answers are this",
        "project's, published in docs/decisions/. Only `stated` is a permission JLReq",
        "grants; the other three are permissions read out of it.",
        "",
        "`jlreq` is the answer Policy::JLREQ selects: JLReq's own preference where it states",
        "one, this project's published reading where JLReq states nothing, and failing both",
        "the reading the specification states first.",
        "",
        "One row per question. The sentence each answer rests on, whether JLReq calls an",
        "answer preferred, the four further preset columns and the exclusions between",
        "answers are properties of an answer rather than of a question, and arrive as",
        "further columns with the stage that emits the policy space.",
    ]));
    for question in &POLICY_SPACE {
        out.push_str(&row(&[
            question.path,
            question.constant,
            question.address,
            question.permission.token(),
            question.locale.tag(),
            &question.permits.join(" "),
            question.jlreq,
            question.evidence,
        ])?);
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

/// One row: the fields, tab separated.
///
/// Every field here was written by a person rather than extracted, so a tab or a newline in
/// one is a mistake that would silently become two fields or two rows. It is refused, and
/// the sentence naming the field is what a writer needs to fix it.
fn row(fields: &[&str]) -> Result<String, String> {
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
/// permitting one answer is not a question, and a `jlreq` naming an answer the question does
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
}

/// One question's answer set: at least two, each named once, and one of them JLReq's.
fn check_answers(question: &Question, violations: &mut Vec<String>) {
    let at = question.constant;
    let mut named: BTreeSet<&str> = BTreeSet::new();
    for answer in question.permits {
        if !is_answer(answer) {
            violations.push(format!(
                "{at}: `{answer}` is not the name of an answer; a name is lower-case ASCII \
                 letters, digits and hyphens"
            ));
        }
        if !named.insert(answer) {
            violations.push(format!("{at}: permits `{answer}` twice"));
        }
    }
    if question.permits.len() < LEAST_ANSWERS {
        violations.push(format!(
            "{at}: permits {count} answer(s); a place with one answer is not a place JLReq \
             permits more than one",
            count = question.permits.len()
        ));
    }
    if !named.contains(question.jlreq) {
        violations.push(format!(
            "{at}: Policy::JLREQ selects `{jlreq}`, which this question does not permit",
            jlreq = question.jlreq
        ));
    }
}

/// Hold one row against the published document.
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

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        POLICY_SPACE, Permission, QUESTION_COUNT, QUESTIONS, Question, Rendering,
        check_against_document, check_answers, check_table, classes_named, is_answer, is_constant,
        is_path, row,
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
        permits: &["strict", "very-strict"],
        jlreq: "strict",
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
    fn a_divergence_that_stopped_diverging_emits_nothing() {
        let diverging = Question {
            permission: Permission::Divergent,
            locale: Rendering::Japanese,
            evidence: "cl-10",
            ..FIXTURE
        };
        let apart = document("C.3", "cl-16", "cl-16 cl-10");
        let mut violations = Vec::new();
        check_against_document(&diverging, &apart, &mut violations);
        assert!(violations.is_empty(), "{violations:?}");

        let together = document("C.3", "cl-16 cl-10", "cl-16 cl-10");
        let mut violations = Vec::new();
        check_against_document(&diverging, &together, &mut violations);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(violations[0].contains("name the same character classes"));
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
            permits: &["strict"],
            ..FIXTURE
        };
        let mut violations = Vec::new();
        check_answers(&single, &mut violations);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(violations[0].contains("is not a place JLReq permits more than one"));
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
        assert!(row(&["one", "two"]).is_ok());
        assert!(row(&["one\ttwo"]).is_err());
        assert!(row(&["one\ntwo"]).is_err());
    }
}
