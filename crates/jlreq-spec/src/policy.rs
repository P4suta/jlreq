// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The policy space: every place JLReq permits more than one answer.
//!
//! Zero Cargo features. The feature-matrix cost is quadratic in features per core crate
//! and there are roughly forty of these places; as features that is uncomputable, and it
//! would also be wrong, because a policy is a property of a document rather than of a
//! build — one process may set a quotation to JIS conventions inside a book set to JLReq
//! conventions.
//!
//! A [`Policy`] is total by construction: there is no unset question, so no default hides
//! in an evaluator. It is opaque, so the question set grows without breaking a caller
//! (see `docs/adr/0012`). And it is validated at construction rather than at use: §C.3
//! defines its strictest level as applying no §C.2 alternate rule, so a policy naming both
//! is not something to check for later but a value that is never built (see
//! `docs/adr/0010`).
//!
//! The questions, their permitted answers and the presets are generated from the
//! specification (see `docs/design/generation.md`). What is written here is the vocabulary
//! and the validation: which combinations are contradictory is data, and that no
//! contradictory combination can be built is code.

use crate::rule::{LARGEST_ORDINAL, RuleId};

// A question is addressed by a `u16` ordinal into the generated policy space, and
// `Policy`'s representation is one byte per question in that same order.
const _: () = assert!(QUESTIONS.len() <= LARGEST_ORDINAL);

// The generated policy space answers every question with an answer that question permits,
// and every exclusion names a question and an answer that exist. A generated file that
// fails this does not compile, which is what makes `Policy::get` total and `Policy::with`
// exhaustive rather than best-effort.
const _: () = assert!(
    is_sound(QUESTIONS),
    "the generated policy space names an answer that does not exist"
);

/// The largest number of answers one question may permit, because [`Choice`] holds the
/// answer's index in a byte.
const LARGEST_CHOICE_COUNT: usize = 255;

/// A place where JLReq permits more than one answer. Generated.
///
/// JLReq: §B.2, §C.2, §C.3, §D, §E.2
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Question(pub(crate) u16);

impl Question {
    // The named constants — `KINSOKU_LEVEL` (§C.3), `REDUCTION_TABLE` (§D),
    // `RUBY_ALIGNMENT` (§3.3.5), `ADJUSTMENT_PREFERENCE` (§C.3's silence), and one per
    // permitted alternative — are emitted beside `QUESTIONS` rather than written here,
    // because each is the section that permits the alternative and a hand-written one is a
    // transcription of it (see `docs/adr/0009`). They arrive with the policy space, in a
    // second `impl Question` block.

    /// Every question.
    ///
    /// JLReq: §B.2, §C.2, §C.3, §D, §E.2
    pub const ALL: &'static [Self] = &identifiers();

    /// `ALL.len()`. Generated, and the width of [`Policy`]'s representation.
    ///
    /// JLReq: n/a (representation)
    pub const COUNT: usize = QUESTIONS.len();

    /// The permitted answers.
    ///
    /// JLReq: §B.2, §C.2, §C.3, §D, §E.2
    #[must_use]
    pub const fn permits(self) -> &'static [Choice] {
        let (_, from_here) = CHOICES.split_at(first_choice_of(QUESTIONS, self.ordinal()));
        let (permitted, _) = from_here.split_at(QUESTIONS[self.ordinal()].choices.len());
        permitted
    }

    /// The section that permits the alternative.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn rule(self) -> RuleId {
        QUESTIONS[self.ordinal()].rule
    }

    /// The stable dotted path used in a conformance case file.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn path(self) -> &'static str {
        QUESTIONS[self.ordinal()].path
    }

    /// This question's position in the generated policy space.
    const fn ordinal(self) -> usize {
        self.0 as usize
    }
}

/// One permitted answer.
///
/// A `Choice` carries the [`Question`] it answers, and [`Policy::with`] reads the
/// question out of it, so setting a question to a choice belonging to a different
/// question is not an expression that can be written.
///
/// JLReq: §B.2, §C.2, §C.3, §D, §E.2
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Choice {
    /// The question this answers.
    pub(crate) question: Question,
    /// Which of that question's permitted answers this is.
    pub(crate) index: u8,
}

impl Choice {
    /// The question this answers.
    ///
    /// JLReq: n/a (representation)
    #[must_use]
    pub const fn question(self) -> Question {
        self.question
    }

    /// The section that states this alternative.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn rule(self) -> RuleId {
        self.record().rule
    }

    /// e.g. "JIS X 4051: ruby shall not extend over katakana".
    ///
    /// JLReq: §B.2, §C.2, §C.3, §D, §E.2
    #[must_use]
    pub const fn statement(self) -> &'static str {
        self.record().statement
    }

    /// Whether JLReq calls this one "preferred". JLReq: §B.2#1, #2, #4, #6, #7, #8, #17
    #[must_use]
    pub const fn is_preferred(self) -> bool {
        self.record().preferred
    }

    /// The stable name used in a conformance case file.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.record().name
    }

    /// This answer's row of the generated policy space.
    const fn record(self) -> &'static ChoiceRecord {
        record_of(QUESTIONS, self.question.ordinal(), self.index)
    }
}

/// The permitted alternative in force at every question.
///
/// Total by construction: there is no unset question, so no default hides in an
/// evaluator. Opaque, so adding a question is not a breaking change.
///
/// JLReq: §B.2, §C.2, §C.3, §D, §E.2
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Policy {
    /// One answer per question, in question order.
    choices: [u8; Question::COUNT],
}

impl Policy {
    /// JLReq's own preference wherever it states one: Strict kinsoku (禁則) (§C.3 level 3,
    /// which JLReq labels "Default, general publications") and reduction Table 3
    /// ("the method adopted by this document").
    ///
    /// This is a citable factual claim, checked field by field against the quotations by a
    /// conformance case. It is not a default: there is no `impl Default for Policy`, so a
    /// caller names a preset and the choice appears in their source where a reviewer sees
    /// it.
    ///
    /// JLReq: §C.3, §D
    pub const JLREQ: Self = Self::preset(Preset::Jlreq);

    /// The JIS X 4051 reading wherever JLReq records a divergence. This is JLReq's
    /// account of JIS, not JIS conformance — note that §B.2 note 5's divergence is a
    /// different class lattice (cl-07 as a subset of cl-02), not a spacing choice.
    ///
    /// JLReq: §B.2#5, §3.1.9
    pub const JIS_READING: Self = Self::preset(Preset::JisReading);

    /// Book practice: reduction Table 5, §3.1.5 pattern 3, hanging punctuation
    /// (ぶら下げ, burasage). JLReq: §D, §3.1.5, §2.5.1
    pub const BOOK: Self = Self::preset(Preset::Book);

    /// Magazine practice: Loose kinsoku (禁則). JLReq: §C.3 level 2
    pub const MAGAZINE: Self = Self::preset(Preset::Magazine);

    /// Newspaper practice: Very loose kinsoku (禁則). JLReq: §C.3 level 1
    pub const NEWSPAPER: Self = Self::preset(Preset::Newspaper);

    /// Set one question. Returns `Err` when the result would be a combination JLReq makes
    /// contradictory — a Very strict level alongside a §C.2 alternate rule, which §C.3
    /// defines Very strict as excluding.
    ///
    /// This is a `Result` rather than a separate `validate`, and the reason is
    /// ADR-0010's: a contradictory policy has no representation, so no entry point
    /// has to check for one and none can forget to. There is deliberately no way to build
    /// a policy that `compose` would have to reject.
    ///
    /// JLReq: §C.3
    pub const fn with(self, choice: Choice) -> Result<Self, PolicyConflict> {
        if let Some(conflict) = conflict(&self.choices, QUESTIONS, choice) {
            return Err(conflict);
        }
        let mut choices = self.choices;
        choices[choice.question.ordinal()] = choice.index;
        Ok(Self { choices })
    }

    /// The answer in force at one question.
    ///
    /// JLReq: §B.2, §C.2, §C.3, §D, §E.2
    #[must_use]
    pub const fn get(self, question: Question) -> Choice {
        Choice {
            question,
            index: self.choices[question.ordinal()],
        }
    }

    /// Every question, its answer, and the section that permits it. No other
    /// implementation of Japanese layout can report this.
    ///
    /// JLReq: §B.2, §C.2, §C.3, §D, §E.2
    pub fn explain(self) -> impl Iterator<Item = (Question, Choice)> {
        Question::ALL
            .iter()
            .copied()
            .map(move |question| (question, self.get(question)))
    }

    /// Every question this policy answers differently from `base`.
    ///
    /// JLReq: §B.2, §C.2, §C.3, §D, §E.2
    pub fn diff(self, base: Self) -> impl Iterator<Item = (Question, Choice)> {
        self.explain()
            .filter(move |(question, choice)| base.get(*question) != *choice)
    }

    // `remainder` — the one function that derives `jlreq_unit::RemainderRule` from a
    // policy, so that `distribute`'s parameter is a transport and not a second carrier
    // (ADR-0019) — arrives with the policy space, beside the named `Question` constants.
    // It reads `Question::REMAINDER`, and until `spec/derived/questions.tsv` is emitted
    // there is no such question, no answer in force at it, and therefore no rule to
    // return; a function returning one anyway would be publishing a typographic decision
    // nobody made. `crates/jlreq-spec/Cargo.toml` gains its `jlreq-unit` dependency in the
    // same commit, which is why the crate graph permits an edge the manifest does not yet
    // declare (ADR-0020).

    /// One named practice, read out of the generated table's preset columns.
    const fn preset(which: Preset) -> Self {
        let column = which.column();
        let mut choices = [0u8; Question::COUNT];
        let mut index = 0;
        while index < QUESTIONS.len() {
            choices[index] = QUESTIONS[index].presets[column];
            index = index.saturating_add(1);
        }
        Self { choices }
    }
}

/// Two choices JLReq makes mutually exclusive — for example [`Question`]'s kinsoku
/// (禁則) level set to Very strict, which §C.3 defines as applying no §C.2 alternate rule,
/// alongside a §C.2 alternate. Returned by [`Policy::with`], so the contradictory policy
/// is never built rather than built and checked.
///
/// The questions are in the order the conflict was met: the one already answered, then the
/// one being set.
///
/// JLReq: §C.3
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PolicyConflict {
    /// The two questions whose answers cannot stand together.
    pub questions: [Question; 2],
    /// The rule that excludes the combination.
    pub rule: RuleId,
}

/// The named practices [`Policy`]'s constants publish, in the order the generated table
/// carries their columns.
///
/// This order is the contract between this crate and the generator: `presets[n]` is the
/// answer the practice at column `n` gives. It is not public, because a preset is a
/// constant a caller names rather than a value a caller computes with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Preset {
    /// JLReq's own preference wherever it states one.
    Jlreq,
    /// The JIS X 4051 reading wherever JLReq records a divergence.
    JisReading,
    /// Book practice.
    Book,
    /// Magazine practice.
    Magazine,
    /// Newspaper practice.
    Newspaper,
}

impl Preset {
    /// How many preset columns every question carries.
    const COUNT: usize = 5;

    /// This practice's column in a question's preset row.
    const fn column(self) -> usize {
        match self {
            Self::Jlreq => 0,
            Self::JisReading => 1,
            Self::Book => 2,
            Self::Magazine => 3,
            Self::Newspaper => 4,
        }
    }
}

/// Every question, in the specification's own reading order.
///
/// # Empty, and why
///
/// The policy space is empty because `spec/derived/questions.tsv` does not exist yet. It
/// is produced by stage 1 of the pipeline, from the same snapshot the rule inventory comes
/// from: a question is an appendix note or a section that states two readings, and its
/// answers are those readings with the sentence stating each (see
/// `docs/design/generation.md`).
///
/// Everything reading this table therefore answers over an empty policy space:
/// [`Question::ALL`] is empty, [`Question::COUNT`] is zero, and the five presets are the
/// same empty answer. That is the honest state of a table nobody has read the
/// specification for, and not a placeholder — a question invented here would publish a
/// permitted alternative the specification does not permit, and a preset filled in here
/// would publish a factual claim about §C.3 that nobody checked (ADR-0009).
///
/// Three things arrive with the policy space. The rows below, each carrying its answers
/// and its preset columns in the order [`Preset::column`] fixes. The exclusions each
/// answer states — §C.3's strictest level excluding every §C.2 alternate rule is the one
/// this design was written around — which is what [`Policy::with`] reads. And one named
/// constant per row, `KINSOKU_LEVEL` for §C.3 and so on.
pub(crate) const QUESTIONS: &[QuestionRecord] = &[];

/// One row of the policy space.
#[derive(Clone, Copy, Debug)]
pub(crate) struct QuestionRecord {
    /// The stable dotted path a conformance case file names this question by.
    pub(crate) path: &'static str,
    /// The section that permits the alternative.
    pub(crate) rule: RuleId,
    /// The permitted answers, in the order the specification states them.
    pub(crate) choices: &'static [ChoiceRecord],
    /// What each named practice answers here, indexed by `Preset::column`.
    pub(crate) presets: [u8; Preset::COUNT],
}

/// One permitted answer of one question.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ChoiceRecord {
    /// The stable name a conformance case file names this answer by.
    pub(crate) name: &'static str,
    /// The sentence that states this alternative.
    pub(crate) statement: &'static str,
    /// The section that states it.
    pub(crate) rule: RuleId,
    /// Whether JLReq calls this one "preferred".
    pub(crate) preferred: bool,
    /// The answers of other questions this one cannot stand beside.
    pub(crate) excludes: &'static [Exclusion],
}

/// One answer of one other question that a choice excludes.
///
/// The relation is symmetric and is recorded once. [`Policy::with`] reads it from both
/// ends, so a generator that wrote each pair twice and a generator that wrote it once
/// produce the same behavior, and the second cannot record half a contradiction.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Exclusion {
    /// The other question.
    pub(crate) question: Question,
    /// The other question's answer that cannot stand beside this one.
    pub(crate) choice: u8,
    /// The rule that excludes the combination.
    pub(crate) rule: RuleId,
}

/// One identifier per question, in the policy space's own order.
const fn identifiers() -> [Question; Question::COUNT] {
    let mut all = [Question(0); Question::COUNT];
    let mut index = 0;
    let mut ordinal = 0u16;
    while index < QUESTIONS.len() {
        all[index] = Question(ordinal);
        index = index.saturating_add(1);
        ordinal = ordinal.saturating_add(1);
    }
    all
}

/// Every permitted answer of every question, in question order.
///
/// Derived rather than emitted: the generated table states each question's answers once,
/// and this is that statement counted, so a choice cannot end up in the wrong question's
/// list (ADR-0019).
const CHOICES: &[Choice] = &choices::<CHOICE_COUNT>(QUESTIONS);

/// How many answers the whole policy space permits.
const CHOICE_COUNT: usize = total_choices(QUESTIONS);

/// Count the answers of every question.
const fn total_choices(table: &[QuestionRecord]) -> usize {
    let mut total = 0usize;
    let mut index = 0;
    while index < table.len() {
        total = total.saturating_add(table[index].choices.len());
        index = index.saturating_add(1);
    }
    total
}

/// Build the flat answer list of a policy space.
///
/// `N` is that space's [`total_choices`], which is what makes the list exactly as long as
/// the answers it holds; the table is a parameter so that the flattening and the offsets
/// into it are exercised over a policy space that has questions in it.
const fn choices<const N: usize>(table: &[QuestionRecord]) -> [Choice; N] {
    let mut all = [Choice {
        question: Question(0),
        index: 0,
    }; N];
    let mut position = 0usize;
    let mut question = 0;
    let mut ordinal = 0u16;
    while question < table.len() {
        let mut index = 0u8;
        while (index as usize) < table[question].choices.len() {
            all[position] = Choice {
                question: Question(ordinal),
                index,
            };
            position = position.saturating_add(1);
            index = index.saturating_add(1);
        }
        question = question.saturating_add(1);
        ordinal = ordinal.saturating_add(1);
    }
    all
}

/// Where one question's answers start in the flat answer list.
const fn first_choice_of(table: &[QuestionRecord], question: usize) -> usize {
    let mut start = 0usize;
    let mut index = 0;
    while index < question {
        start = start.saturating_add(table[index].choices.len());
        index = index.saturating_add(1);
    }
    start
}

/// One answer's row, in a policy space.
///
/// The row outlives the table it was reached through, because a question's answers are
/// themselves a `'static` slice: a policy space is generated data all the way down.
const fn record_of(table: &[QuestionRecord], question: usize, choice: u8) -> &'static ChoiceRecord {
    let permitted = table[question].choices;
    &permitted[choice as usize]
}

/// The conflict setting `incoming` would create with an answer already in force, if any.
///
/// `current` is one answer per question, indexed as `table` is. Both directions of the
/// exclusion relation are read, which is what lets the generated data record each
/// exclusion once.
const fn conflict(
    current: &[u8],
    table: &[QuestionRecord],
    incoming: Choice,
) -> Option<PolicyConflict> {
    if let Some(conflict) = incoming_excludes_an_answer(current, table, incoming) {
        return Some(conflict);
    }
    an_answer_excludes_incoming(current, table, incoming)
}

/// The forward direction: the answer being set excludes one already in force.
const fn incoming_excludes_an_answer(
    current: &[u8],
    table: &[QuestionRecord],
    incoming: Choice,
) -> Option<PolicyConflict> {
    let excludes = record_of(table, incoming.question.ordinal(), incoming.index).excludes;
    let mut index = 0;
    while index < excludes.len() {
        let exclusion = excludes[index];
        let other = exclusion.question.ordinal();
        if other < current.len() && current[other] == exclusion.choice {
            return Some(PolicyConflict {
                questions: [exclusion.question, incoming.question],
                rule: exclusion.rule,
            });
        }
        index = index.saturating_add(1);
    }
    None
}

/// The reverse direction: an answer already in force excludes the one being set.
const fn an_answer_excludes_incoming(
    current: &[u8],
    table: &[QuestionRecord],
    incoming: Choice,
) -> Option<PolicyConflict> {
    let asked = incoming.question.ordinal();
    let mut index = 0;
    let mut ordinal = 0u16;
    while index < table.len() && index < current.len() {
        if index != asked {
            let excludes = record_of(table, index, current[index]).excludes;
            if let Some(rule) = excluding_rule(excludes, incoming) {
                return Some(PolicyConflict {
                    questions: [Question(ordinal), incoming.question],
                    rule,
                });
            }
        }
        index = index.saturating_add(1);
        ordinal = ordinal.saturating_add(1);
    }
    None
}

/// The rule by which one of `excludes` forbids `incoming`, if one does.
const fn excluding_rule(excludes: &[Exclusion], incoming: Choice) -> Option<RuleId> {
    let mut index = 0;
    while index < excludes.len() {
        let exclusion = excludes[index];
        if exclusion.question.0 == incoming.question.0 && exclusion.choice == incoming.index {
            return Some(exclusion.rule);
        }
        index = index.saturating_add(1);
    }
    None
}

/// Whether a policy space is structurally sound: every question answerable, every preset
/// naming an answer that question permits, every exclusion naming a question and an answer
/// that exist.
///
/// This is what makes [`Policy::get`] total. It is checked over the generated table at
/// compile time rather than asserted in prose, because every one of these is a mistake an
/// emitter can make and none of them has a sensible runtime behavior.
const fn is_sound(table: &[QuestionRecord]) -> bool {
    every_question_is_answerable(table)
        && every_preset_names_an_answer(table)
        && every_exclusion_resolves(table)
}

/// Whether every question permits at least one answer and at most as many as a byte holds.
const fn every_question_is_answerable(table: &[QuestionRecord]) -> bool {
    let mut index = 0;
    while index < table.len() {
        let permitted = table[index].choices.len();
        if permitted == 0 || permitted > LARGEST_CHOICE_COUNT {
            return false;
        }
        index = index.saturating_add(1);
    }
    true
}

/// Whether every preset column names an answer its question permits.
const fn every_preset_names_an_answer(table: &[QuestionRecord]) -> bool {
    let mut index = 0;
    while index < table.len() {
        let permitted = table[index].choices.len();
        let mut column = 0;
        while column < Preset::COUNT {
            if table[index].presets[column] as usize >= permitted {
                return false;
            }
            column = column.saturating_add(1);
        }
        index = index.saturating_add(1);
    }
    true
}

/// Whether every exclusion names another question and an answer that question permits.
const fn every_exclusion_resolves(table: &[QuestionRecord]) -> bool {
    let mut index = 0;
    while index < table.len() {
        let mut choice = 0;
        while choice < table[index].choices.len() {
            if !exclusions_resolve(table, index, table[index].choices[choice].excludes) {
                return false;
            }
            choice = choice.saturating_add(1);
        }
        index = index.saturating_add(1);
    }
    true
}

/// Whether one answer's exclusions resolve.
const fn exclusions_resolve(
    table: &[QuestionRecord],
    question: usize,
    excludes: &[Exclusion],
) -> bool {
    let mut index = 0;
    while index < excludes.len() {
        let exclusion = excludes[index];
        let other = exclusion.question.ordinal();
        // A question cannot exclude its own answers: only one of them is ever in force.
        if other == question || other >= table.len() {
            return false;
        }
        if exclusion.choice as usize >= table[other].choices.len() {
            return false;
        }
        index = index.saturating_add(1);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{
        Choice, ChoiceRecord, Exclusion, Policy, Preset, Question, QuestionRecord, choices,
        conflict, every_exclusion_resolves, every_preset_names_an_answer,
        every_question_is_answerable, first_choice_of, is_sound, total_choices,
    };
    use crate::rule::RuleId;

    /// Stands for §C.3, which states the exclusion. The inventory is generated and empty,
    /// so a fixture rule is an ordinal and nothing here reads the inventory.
    const C3: RuleId = RuleId(0);
    /// Stands for §C.2, which states the alternate rule.
    const C2: RuleId = RuleId(1);

    /// The strictness level.
    const LEVEL: Question = Question(0);
    /// Whether a §C.2 alternate rule applies.
    const ALTERNATE: Question = Question(1);

    /// §C.3's strictest level, which the section defines as applying no alternate rule.
    const VERY_STRICT: u8 = 0;
    /// §C.3's default level.
    const STRICT: u8 = 1;
    /// The alternate rule is not applied.
    const PLAIN: u8 = 0;
    /// The alternate rule is applied.
    const RELAXED: u8 = 1;

    /// Two questions of the shape §C.3 and §C.2 have, with the one exclusion §C.3 states.
    ///
    /// The exclusion is recorded on the level's Very strict answer only, so a test that
    /// sets the two in either order proves that the relation is read from both ends.
    const FIXTURE: &[QuestionRecord] = &[
        QuestionRecord {
            path: "kinsoku.level",
            rule: C3,
            choices: &[
                ChoiceRecord {
                    name: "very-strict",
                    statement: "The very strict rule is for the best appearance at the line head.",
                    rule: C3,
                    preferred: false,
                    excludes: &[Exclusion {
                        question: ALTERNATE,
                        choice: RELAXED,
                        rule: C3,
                    }],
                },
                ChoiceRecord {
                    name: "strict",
                    statement: "Default, general publications.",
                    rule: C3,
                    preferred: true,
                    excludes: &[],
                },
            ],
            presets: [STRICT; Preset::COUNT],
        },
        QuestionRecord {
            path: "kinsoku.alternate_rule",
            rule: C2,
            choices: &[
                ChoiceRecord {
                    name: "plain",
                    statement: "The alternate rule is not applied.",
                    rule: C2,
                    preferred: true,
                    excludes: &[],
                },
                ChoiceRecord {
                    name: "relaxed",
                    statement: "A repetition mark at the line head is treated as cl-19.",
                    rule: C2,
                    preferred: false,
                    excludes: &[],
                },
            ],
            presets: [PLAIN; Preset::COUNT],
        },
    ];

    /// Answer the fixture's two questions.
    const fn answers(level: u8, alternate: u8) -> [u8; 2] {
        [level, alternate]
    }

    /// One answer of the fixture's policy space.
    const fn answer(question: Question, index: u8) -> Choice {
        Choice { question, index }
    }

    #[test]
    fn a_permitted_combination_is_accepted() {
        assert_eq!(
            conflict(&answers(STRICT, PLAIN), FIXTURE, answer(ALTERNATE, RELAXED)),
            None,
            "§C.2's alternate rule stands beside every level but the strictest"
        );
    }

    #[test]
    fn an_alternate_rule_beside_the_strictest_level_is_refused() {
        let refused = conflict(
            &answers(VERY_STRICT, PLAIN),
            FIXTURE,
            answer(ALTERNATE, RELAXED),
        )
        .expect("§C.3 defines Very strict as applying no §C.2 alternate rule");
        assert_eq!(refused.questions, [LEVEL, ALTERNATE]);
        assert_eq!(refused.rule, C3);
    }

    #[test]
    fn the_strictest_level_beside_an_alternate_rule_is_refused() {
        let refused = conflict(
            &answers(STRICT, RELAXED),
            FIXTURE,
            answer(LEVEL, VERY_STRICT),
        )
        .expect("the same contradiction, reached from the other side");
        assert_eq!(refused.questions, [ALTERNATE, LEVEL]);
        assert_eq!(
            refused.rule, C3,
            "one recorded exclusion, read from both ends"
        );
    }

    #[test]
    fn answering_a_question_again_is_not_a_conflict_with_itself() {
        assert_eq!(
            conflict(
                &answers(VERY_STRICT, PLAIN),
                FIXTURE,
                answer(LEVEL, VERY_STRICT)
            ),
            None,
            "one answer of a question is in force at a time"
        );
        assert_eq!(
            conflict(&answers(VERY_STRICT, PLAIN), FIXTURE, answer(LEVEL, STRICT)),
            None,
            "relaxing the level is what makes the alternate rule available"
        );
    }

    #[test]
    fn the_order_the_two_are_set_in_does_not_decide_whether_they_conflict() {
        // Both orders end at the same contradiction, which is the property that makes
        // `with` returning a result sufficient: there is no sequence of calls that
        // arrives at a contradictory policy.
        let one_way = conflict(
            &answers(VERY_STRICT, PLAIN),
            FIXTURE,
            answer(ALTERNATE, RELAXED),
        );
        let other_way = conflict(
            &answers(STRICT, RELAXED),
            FIXTURE,
            answer(LEVEL, VERY_STRICT),
        );
        assert!(one_way.is_some() && other_way.is_some());
        assert_eq!(
            one_way.map(|refused| refused.rule),
            other_way.map(|refused| refused.rule)
        );
    }

    #[test]
    fn a_sound_policy_space_is_sound() {
        assert!(is_sound(FIXTURE));
    }

    #[test]
    fn the_flat_answer_list_is_every_questions_answers_in_order() {
        // What `Question::permits` slices, and where each question's slice starts. The
        // list is derived rather than emitted, so an off-by-one here would hand one
        // question another's answers with nothing to contradict it.
        assert_eq!(total_choices(FIXTURE), 4);
        assert_eq!(
            choices::<4>(FIXTURE),
            [
                answer(LEVEL, VERY_STRICT),
                answer(LEVEL, STRICT),
                answer(ALTERNATE, PLAIN),
                answer(ALTERNATE, RELAXED),
            ]
        );
        assert_eq!(first_choice_of(FIXTURE, 0), 0);
        assert_eq!(first_choice_of(FIXTURE, 1), 2);
        assert_eq!(first_choice_of(FIXTURE, FIXTURE.len()), 4);
    }

    #[test]
    fn a_question_with_no_answer_is_not_sound() {
        const UNANSWERABLE: &[QuestionRecord] = &[QuestionRecord {
            path: "kinsoku.level",
            rule: C3,
            choices: &[],
            presets: [0; Preset::COUNT],
        }];
        assert!(!every_question_is_answerable(UNANSWERABLE));
        assert!(!is_sound(UNANSWERABLE));
    }

    #[test]
    fn a_preset_naming_an_answer_that_does_not_exist_is_not_sound() {
        const OUT_OF_RANGE: &[QuestionRecord] = &[QuestionRecord {
            path: "kinsoku.level",
            rule: C3,
            choices: &[ChoiceRecord {
                name: "strict",
                statement: "Default, general publications.",
                rule: C3,
                preferred: true,
                excludes: &[],
            }],
            presets: [1; Preset::COUNT],
        }];
        assert!(!every_preset_names_an_answer(OUT_OF_RANGE));
        assert!(!is_sound(OUT_OF_RANGE));
    }

    #[test]
    fn an_exclusion_naming_a_question_that_does_not_exist_is_not_sound() {
        const DANGLING: &[QuestionRecord] = &[QuestionRecord {
            path: "kinsoku.level",
            rule: C3,
            choices: &[ChoiceRecord {
                name: "very-strict",
                statement: "The very strict rule is for the best appearance at the line head.",
                rule: C3,
                preferred: false,
                excludes: &[Exclusion {
                    question: ALTERNATE,
                    choice: RELAXED,
                    rule: C3,
                }],
            }],
            presets: [0; Preset::COUNT],
        }];
        assert!(!every_exclusion_resolves(DANGLING));
        assert!(!is_sound(DANGLING));
    }

    #[test]
    fn an_exclusion_of_a_questions_own_answer_is_not_sound() {
        const SELF_EXCLUDING: &[QuestionRecord] = &[QuestionRecord {
            path: "kinsoku.level",
            rule: C3,
            choices: &[
                ChoiceRecord {
                    name: "very-strict",
                    statement: "The very strict rule is for the best appearance at the line head.",
                    rule: C3,
                    preferred: false,
                    excludes: &[Exclusion {
                        question: LEVEL,
                        choice: STRICT,
                        rule: C3,
                    }],
                },
                ChoiceRecord {
                    name: "strict",
                    statement: "Default, general publications.",
                    rule: C3,
                    preferred: true,
                    excludes: &[],
                },
            ],
            presets: [0; Preset::COUNT],
        }];
        assert!(!every_exclusion_resolves(SELF_EXCLUDING));
    }

    #[test]
    fn every_preset_column_is_named_once() {
        let columns = [
            Preset::Jlreq.column(),
            Preset::JisReading.column(),
            Preset::Book.column(),
            Preset::Magazine.column(),
            Preset::Newspaper.column(),
        ];
        for (position, column) in columns.iter().enumerate() {
            assert_eq!(
                *column, position,
                "the columns are the generator's contract"
            );
        }
        assert_eq!(columns.len(), Preset::COUNT);
    }

    #[test]
    fn the_policy_space_is_empty_until_it_is_generated() {
        assert!(
            Question::ALL.is_empty(),
            "spec/derived/questions.tsv has not been emitted yet"
        );
        assert_eq!(Question::COUNT, 0);
        assert_eq!(Policy::JLREQ.explain().count(), 0);
        assert_eq!(
            Policy::JLREQ.diff(Policy::BOOK).count(),
            0,
            "two presets differ at a question, and there is no question yet"
        );
        assert_eq!(
            Policy::JLREQ,
            Policy::NEWSPAPER,
            "every preset answers the same empty set of questions; they diverge when the \
             policy space is generated"
        );
    }
}
