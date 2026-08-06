// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Classification: what class one occurrence is, and what the answer rests on.
//!
//! There is no function from a code point to a class and there cannot be one. Appendix A
//! enumerates 1133 keys and names 473 of them under more than one class, reaching degree
//! four; five classes enumerate nothing at all. So a class is a property of an occurrence
//! the caller describes — a cluster of the caller's own text, with the frame (字幅) their
//! advance covers and the role their document gives it (`docs/adr/0008`).
//!
//! # How an answer is reached
//!
//! Appendix A names the candidates, and the caller's facts remove the ones their document
//! rules out. Five steps, in this order:
//!
//! 1. the key's listings, reached literally and then through the compatibility folding,
//!    together with the ideograph predicate §A.19 defers to the character database for;
//! 2. the policy's reclassifications, which §C.2 notes 1 through 3 state as "shall be
//!    treated as a member of" another class and which therefore apply before the table is
//!    read for anything else;
//! 3. the frame as the Remarks column states it, for the 834 rows that state one;
//! 4. the frame as the prose states it, for the rows that state none: §3.1.2 gives five
//!    classes a half-width advance, so a third-em, quarter-em or proportional one is not
//!    theirs, and §3.2.4 and §3.2.6 separate cl-19 from cl-27;
//! 5. the role, which six code points need and no others.
//!
//! A step that would leave no candidate leaves the set alone. A declared frame no listing
//! permits is a caller declaring something about their own document that Appendix A does
//! not record — a diagnostic, which `jlreq::diagnose` reports — and never a reason to
//! answer with no class at all.
//!
//! # What the caller is not asked for, and what that costs
//!
//! The construct axis has no parameter here, because a construct is a run over a stream
//! rather than a property of one item (`docs/adr/0015`). Nine classes are membership *in* a
//! construct — the five that enumerate nothing, and the four that enumerate what may appear
//! inside a grouped numeral (連数字), a unit symbol, or a warichu (割注) bracket — so
//! wherever one of those survives, [`Classified::Several`] names [`AxisSet::CONSTRUCT`] and
//! says the axis was never supplied. That is the honest answer rather than a quiet
//! preference for the class that happens to sort first; [`resolve`] is where a caller who
//! must have one asks for it, and it says in its provenance that it is a tie-break.
//!
//! What that tie-break may not do is answer *with* one of those nine. Four of them — cl-24,
//! cl-25, cl-28 and cl-29 — enumerate ordinary Western and punctuation keys and are numbered
//! below cl-27, so a rule that simply took the lowest-numbered survivor answered "a
//! character inside a unit symbol" for every proportional Latin letter in a Japanese
//! document. [`resolve`] passes over them, and reaches one only when nothing else survived.

use jlreq_spec::{Answer, Choice, Policy, Provenance, RuleId, Standing};
use jlreq_unit::{Frame, Item, ItemIndex, Role};

use crate::class::{Class, ClassSet};
use crate::generated::appendix_a::{
    FRAME_FULL_EM, FRAME_HALF_EM, FRAME_PROPORTIONAL, FRAME_QUARTER_EM, FRAME_THIRD_EM,
    FRAMES_UNSTATED, Listing, REMARKS, ROLE_DECIMAL_POINT, ROLE_DIGIT_GROUP_SEPARATOR,
    ROLE_UNSTATED,
};
use crate::member::{
    Member, asserted_frame, folded, is_ideograph, listings, members, only_code_point,
};
use crate::text::{Annotation, AnnotationIndex, Text};

/// §3.9.2, the section that groups characters into classes and hands the membership to
/// Appendix A. Every classification rests on it.
const GROUPING: RuleId = RuleId::GROUPING_OF_CHARACTERS_AND_SYMBOLS_DEPENDING_ON_THEIR_POSITIONING;

/// §3.2.4, which puts full-width and fixed-width Western characters in cl-19.
const FULL_WIDTH_WESTERN: RuleId =
    RuleId::METHOD_FOR_SETTING_FULL_WIDTH_LATIN_LETTERS_AND_EUROPEAN_NUMERALS;

/// §3.2.6, which puts proportional Western characters in cl-27.
const PROPORTIONAL_WESTERN: RuleId =
    RuleId::HANDLING_OF_WESTERN_TEXT_IN_JAPANESE_TEXT_USING_PROPORTIONAL_WESTERN_FONTS;

/// §3.1.2, which states the character advance of five classes as half-width.
const HALF_WIDTH_ADVANCE: RuleId =
    RuleId::POSITIONING_OF_PUNCTUATION_MARKS_COMMAS_PERIODS_AND_BRACKETS;

/// Every frame the caller can declare, so a narrowing can be tried against each of them.
///
/// [`Frame`] is `#[non_exhaustive]`, so this is the vocabulary as it stands rather than the
/// type's cardinality: a frame added later would not be tried here, and the consequence is
/// that [`AxisSet::FRAME`] would go unreported for a key only that frame separates. The
/// list is checked against the generated frame vocabulary by a test.
const FRAMES: [Frame; 5] = [
    Frame::FullEm,
    Frame::HalfEm,
    Frame::ThirdEm,
    Frame::QuarterEm,
    Frame::Proportional,
];

/// Every role the caller can declare, for the same reason [`FRAMES`] is written out.
const ROLES: [Role; 6] = [
    Role::DecimalPoint,
    Role::DigitGroupSeparator,
    Role::UnitSymbol,
    Role::QuantitySymbol,
    Role::SentenceTerminator,
    Role::SentenceMedial,
];

/// The answer to "what class is this occurrence".
///
/// JLReq: §3.9.2, §A
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Classified {
    /// One class survives. [`Answer::why`] names the rule that decided it.
    One(Answer<Class>),
    /// Appendix A names several and the supplied facts do not separate them. The axes
    /// that would are named, as a set rather than singly: §3.9.2's own irreducible
    /// example needs frame *and* role, and `U+0031` needs frame *and* construct.
    Several {
        /// The classes still standing.
        candidates: ClassSet,
        /// The caller-supplied axes that would separate them.
        needs: AxisSet,
    },
    /// Appendix A names several and nothing can separate them. §3.9.2 concedes the case
    /// — "エディター（editor）は……" — and states a preference rather than a rule, so
    /// `Question::AMBIGUOUS_CONTEXT` decides it and [`resolve`] applies that choice.
    Irreducible {
        /// The classes still standing.
        candidates: ClassSet,
        /// That §3.9.2 states a preference here rather than a rule.
        why: Provenance,
    },
    /// The member is in no Appendix A table — most of Unicode. §3.9.2 records that
    /// JIS X 4051 leaves this implementation-defined and that JLReq inherits it, so the
    /// answer is a published reading marked [`Standing::Unstated`].
    Unlisted,
    /// The ordinal names no item of this stream, so there is no occurrence to classify.
    ///
    /// Distinct from [`Classified::Unlisted`], which is a fact about Appendix A: "the
    /// specification lists no class for the member at this ordinal" and "there is no member
    /// at this ordinal" are two different answers, and a caller who could not tell them
    /// apart would read a class for an occurrence that is not there. `Text::items` is what
    /// an ordinal is bounded against.
    NoSuchItem,
}

impl Classified {
    /// Frozen projection (ADR-0012): whether the supplied facts decided this. `true` for
    /// every variant but [`Classified::One`], and a new variant recording a further
    /// reason the facts did not decide keeps the answer `true`.
    ///
    /// JLReq: §3.9.2
    #[must_use]
    pub const fn is_ambiguous(self) -> bool {
        !matches!(self, Self::One(_))
    }
}

/// Which caller-supplied axes would separate the surviving candidates.
///
/// Exactly three, and there is no fourth. §3.7.4's in-line and independent-line settings
/// look like a fourth and are not: they change the *spacing* between cl-17 or cl-18 and
/// its neighbors, never which class a member is in, so they belong to
/// `jlreq_unit::ConstructKind::MathFormula` and to an override predicate, not here.
///
/// JLReq: §3.2.4, §3.2.6, §3.9.2
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct AxisSet(u8);

impl AxisSet {
    /// No axis at all: nothing the caller could state would narrow the answer further.
    ///
    /// JLReq: §3.9.2
    pub const EMPTY: Self = Self(0);

    /// The frame (字幅) the caller's advance covers. JLReq: §3.2.4, §3.2.6, §A Remarks
    pub const FRAME: Self = Self(0b001);

    /// The syntactic job the document gives the occurrence. JLReq: §3.1.3, §A.24
    pub const ROLE: Self = Self(0b010);

    /// Which ruby, tate-chu-yoko (縦中横), warichu (割注), grouped numeral (連数字) or
    /// unit symbol the occurrence belongs to. JLReq: §A.20–§A.25, §A.28–§A.30
    pub const CONSTRUCT: Self = Self(0b100);

    /// This set together with another, so an occurrence needing two axes names both.
    ///
    /// JLReq: §3.9.2
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether every axis of `axis` is in this set. [`AxisSet::EMPTY`] is in every set,
    /// which is what asking for nothing means.
    ///
    /// JLReq: §3.9.2
    #[must_use]
    pub const fn contains(self, axis: Self) -> bool {
        self.0 & axis.0 == axis.0
    }

    /// Whether no axis is in this set. JLReq: §3.9.2
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// What a relaxation or a reclassification applies to.
///
/// §C.3's own heading says "the following character classes (or characters)", and its
/// levels differ precisely on this: Very loose relaxes cl-05, cl-09 and cl-13 as whole
/// classes, while Loose relaxes `・`, `々` and `%` as single members of those same
/// classes. A subject typed as a class cannot tell the two levels apart.
///
/// The same granularity governs reclassification. §C.2 note 1 moves `々` alone into
/// cl-19, not cl-09's other five members, and §C.2's percent alternative moves `%` alone
/// out of cl-13's thirty-two. Both mechanisms therefore key on this type.
///
/// JLReq: §C.3, §C.2#1–#3, §A.9, §A.13
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Subject {
    /// Every member of one class.
    Class(Class),
    /// One member, whatever class it is read under.
    Member(Member),
    /// One adjacency of two members, which is a boundary and never one occurrence.
    Pair(Member, Member),
}

impl Subject {
    /// Frozen projection (ADR-0012): whether this subject names an adjacency rather than
    /// something one occurrence can be.
    ///
    /// The distinction every consumer turns on: a boundary is `jlreq-spacing`'s to evaluate
    /// and never this crate's to classify, so a subject added later that names a wider
    /// adjacency still answers `true` and a caller's branch keeps meaning what it meant.
    ///
    /// JLReq: §C.2#1–#3, §C.3
    #[must_use]
    pub const fn is_boundary(self) -> bool {
        matches!(self, Self::Pair(_, _))
    }
}

/// A policy-driven change of class, applied before any table lookup.
///
/// §C.2 notes 1 through 3 do not merely permit a break: they say the character "shall be
/// treated as a member of" another class, and §C.2 note 1 adds the dereference
/// instruction "see the cells for ideographic characters (cl-19)". So a relaxed `々`
/// answers as cl-19 against all six matrices, not only at the line head, and the change
/// has to happen here rather than in the line breaker.
///
/// §C.3 states the same three relaxations as overrides of Table 2 alone, and the two
/// readings are not equivalent. Both are expressible and the choice is
/// `Question::RELAXATION_MECHANISM`; this project's reading is recorded, because §C.3
/// defines its strictest level by reference to the §C.2 notes, which implies the level
/// selector drives them.
///
/// JLReq: §C.2#1–#3, §B.2#14–#16, §E.2#1–#3, §C.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Reclassification {
    /// What the change applies to.
    subject: Subject,
    /// The class the subject is then treated as a member of.
    to: Class,
    /// The answer that puts the change in force.
    when: Choice,
    /// The section that states it.
    rule: RuleId,
}

impl Reclassification {
    /// What this change applies to. JLReq: §C.2#1–#3
    #[must_use]
    pub const fn subject(self) -> Subject {
        self.subject
    }

    /// The class the subject is treated as a member of. JLReq: §C.2#1–#3
    #[must_use]
    pub const fn to(self) -> Class {
        self.to
    }

    /// The answer that puts this change in force. JLReq: §C.2#1–#3, §C.3
    #[must_use]
    pub const fn when(self) -> Choice {
        self.when
    }

    /// The section that states it. JLReq: §C.2#1–#3
    #[must_use]
    pub const fn rule(self) -> RuleId {
        self.rule
    }

    /// Whether this change applies to one occurrence of `member` read under `candidates`.
    ///
    /// A [`Subject::Pair`] never does: it names an adjacency, which is a boundary that
    /// `jlreq-spacing` evaluates and not an occurrence that this crate classifies.
    fn applies_to(self, member: Member, candidates: ClassSet) -> bool {
        match self.subject {
            Subject::Class(class) => candidates.contains(class),
            Subject::Member(subject) => subject == member,
            Subject::Pair(_, _) => false,
        }
    }
}

/// Every reclassification the specification states.
///
/// # Empty, and why
///
/// The three §C.2 notes that state one — note 1 moving `々` into cl-19, and the two
/// alternatives beside it — are appendix notes, and an appendix note becomes data when the
/// policy space and the note table are generated (`docs/design/generation.md`). Neither
/// exists yet: a question is a section that states two readings, which is a reading of
/// prose rather than a property a scanner computes, so `spec/derived/questions.tsv` is
/// written with its choices read by a person.
///
/// A reclassification invented here would publish a permitted alternative the specification
/// does not permit, which is what ADR 0009 exists to prevent, so the table is empty and the
/// mechanism above it is written and total. The moment the rows arrive, [`classify`] reads
/// them.
const RECLASSIFICATIONS: &[Reclassification] = &[];

/// Resolve the class of one item.
///
/// Total over the items of a [`Text`], because [`Text::new`] has already refused every
/// stream whose items are not one Appendix A key each (ADR-0018). There is no
/// "misaligned" answer, because there is no misaligned input.
///
/// An ordinal naming no item answers [`Classified::NoSuchItem`], which is not
/// [`Classified::Unlisted`]: the second is a fact about Appendix A and the first is a fact
/// about the ordinal.
///
/// JLReq: §3.9.2, §A, §3.2.4, §3.2.6, §C.2#1–#3
#[must_use]
pub fn classify(text: Text<'_>, index: ItemIndex, policy: Policy) -> Classified {
    examine(
        text.cluster(index),
        text.items().get(index.get() as usize),
        policy,
    )
}

/// The annotation twin. One implementation, two ordinal types: ruby text is classified by
/// the same tables and the same axes, and §3.3 gives it boundaries of its own.
///
/// JLReq: §3.9.2, §A, §3.3.1
#[must_use]
pub fn classify_annotation(
    annotation: Annotation<'_>,
    index: AnnotationIndex,
    policy: Policy,
) -> Classified {
    examine(
        annotation.cluster(index),
        annotation.items().get(index.get() as usize),
        policy,
    )
}

/// The total variant, for callers that must have an answer.
///
/// Defined as [`classify`] followed by exactly one further step, so there is one
/// classification implementation and not two: an unlisted member is answered by this
/// project's reading of the silence §3.9.2 records, and a residual ambiguity by its reading
/// of the preference §3.9.2 states. Every answer either branch produces carries
/// [`Standing::Unstated`], which is how a caller tells a class this project read from a
/// class the specification decided.
///
/// `None` exactly when `index` names no item. An occurrence that is not there has no class,
/// and answering one would put a class a caller cannot distinguish from a real one into a
/// loop over `0..=items.len()`. [`Text::items`] is what an ordinal is bounded against, and
/// [`Text::size_of`] and [`Text::cluster`] state their own boundary at their own accessors.
///
/// # The two questions, and where they are today
///
/// The policy space is generated from `spec/derived/questions.tsv`, which is not written:
/// a question is a section stating two readings, which is a reading of prose rather than a
/// property a scanner computes. So `Question::UNLISTED_CODE_POINT` and
/// `Question::AMBIGUOUS_CONTEXT` do not exist yet, `policy` cannot change either outcome,
/// and what applies is this project's published reading, recorded in
/// `docs/decisions/unlisted-code-point.md` and `docs/decisions/ambiguous-context.md`. Both
/// files state what a caller who disagrees would answer instead, and both readings gain a
/// conformance case, so the disagreement is publishable before the mechanism exists.
///
/// JLReq: §3.9.2
#[must_use]
pub fn resolve(text: Text<'_>, index: ItemIndex, policy: Policy) -> Option<Answer<Class>> {
    let item = text.items().get(index.get() as usize);
    match classify(text, index, policy) {
        Classified::One(answer) => Some(answer),
        Classified::Several { candidates, .. } | Classified::Irreducible { candidates, .. } => {
            Some(ambiguous_context(candidates))
        },
        Classified::Unlisted => Some(unlisted_code_point(
            item.map_or(Frame::Unstated, |item| item.frame()),
        )),
        Classified::NoSuchItem => None,
    }
}

/// Every class Appendix A names one key under, before any of the caller's facts apply.
///
/// Crate-visible because `Text::new` reads it: an item whose key is named under one of
/// §3.1.2's five classes must declare a frame, and that is a question about the table and
/// not about the answer (ADR-0018).
pub(crate) fn listed_classes(member: Member) -> ClassSet {
    rows(member).fold(ClassSet::EMPTY, |set, row| set.with(row.class))
}

/// Every class Appendix A names one key under *as written*, with neither the compatibility
/// folding nor the ideograph predicate.
///
/// Crate-visible because `Text::new` reads it for ADR 0018's Western-ligature exception,
/// which is about the keys of the cluster the shaper produced. The folded reading is the
/// right one for "what class is this occurrence" and the wrong one for "is this key one
/// §A.27 lists": `U+FF21` folds onto `U+0041`, and §3.2.4 puts full-width Latin in cl-19.
pub(crate) fn literally_listed_classes(member: Member) -> ClassSet {
    listings(member)
        .iter()
        .filter_map(|listing| Class::from_number(listing.class))
        .fold(ClassSet::EMPTY, ClassSet::with)
}

/// One candidate: the class, and the qualifications the Remarks cell states for it.
#[derive(Debug, Clone, Copy)]
struct Row {
    /// The class Appendix A lists the key under.
    class: Class,
    /// The frames the Remarks cell permits, as a mask; zero when it states none.
    frames: u8,
    /// The role the Remarks cell names, or [`ROLE_UNSTATED`].
    role: u8,
}

/// Every candidate for one key.
///
/// The literal key first and the folded one only when the literal is listed nowhere, which
/// is what keeps `U+3000` cl-14 rather than folding it onto the Western word space; then
/// the ideograph predicate, which is where the 101 996 kanji §A.19 does not enumerate come
/// from.
fn rows(member: Member) -> impl Iterator<Item = Row> {
    let literal = listings(member);
    let table = if literal.is_empty() {
        folded(member).map_or(&[][..], listings)
    } else {
        literal
    };
    let named_ideographic = table
        .iter()
        .any(|listing| listing.class == Class::Ideographic.number());
    let ideographic = !named_ideographic && only_code_point(member).is_some_and(is_ideograph);
    table
        .iter()
        .filter_map(row_of)
        .chain(ideographic.then_some(Row {
            class: Class::Ideographic,
            frames: FRAMES_UNSTATED,
            role: ROLE_UNSTATED,
        }))
}

/// One listing read as a candidate, or `None` for a row the generated table could not have
/// written — which the compile-time assertions over that table already rule out.
fn row_of(listing: &'static Listing) -> Option<Row> {
    let remark = REMARKS.get(listing.remark as usize)?;
    Some(Row {
        class: Class::from_number(listing.class)?,
        frames: remark.frames,
        role: remark.role,
    })
}

/// The bit the generated Remarks vocabulary gives one declared frame.
const fn frame_bit(frame: Frame) -> u8 {
    match frame {
        Frame::FullEm => FRAME_FULL_EM,
        Frame::HalfEm => FRAME_HALF_EM,
        Frame::ThirdEm => FRAME_THIRD_EM,
        Frame::QuarterEm => FRAME_QUARTER_EM,
        Frame::Proportional => FRAME_PROPORTIONAL,
        // `Frame` is `#[non_exhaustive]`; `Unstated` and anything added later state no
        // frame, and a mask of zero is what "narrows nothing" is written as below.
        _ => FRAMES_UNSTATED,
    }
}

/// The classes one declared role selects, or every class when it selects none.
///
/// Each of the five is the class whose Appendix A section is about that job: §A.24 is the
/// characters permitted inside a grouped numeral (連数字), which is where a decimal point
/// and a digit-grouping separator occur; §A.25 is the characters inside a unit symbol;
/// §C.2 note 11 and §E.2 note 10 treat a quantity symbol as a Western character; §3.1.1
/// gives the sentence-final full stop and comma their own two classes; and §3.1.6 is about
/// the dividing punctuation marks, which is what a sentence-medial one is.
///
/// A role added to the vocabulary later selects nothing until it is written here, which
/// widens an answer rather than narrowing it wrongly.
///
/// JLReq: §A.24, §A.25, §C.2#11, §3.1.1, §3.1.6
fn selected_by(role: Role) -> ClassSet {
    match role {
        Role::DecimalPoint | Role::DigitGroupSeparator => ClassSet::of(Class::InGroupedNumeral),
        Role::UnitSymbol => ClassSet::of(Class::InUnitSymbol),
        Role::QuantitySymbol => ClassSet::of(Class::Western),
        Role::SentenceTerminator => ClassSet::of(Class::FullStop).with(Class::Comma),
        Role::SentenceMedial => ClassSet::of(Class::DividingPunctuation),
        _ => ClassSet::ALL,
    }
}

/// The role a declared one is written as in the Remarks vocabulary.
const fn remark_role(role: Role) -> u8 {
    match role {
        Role::DecimalPoint => ROLE_DECIMAL_POINT,
        Role::DigitGroupSeparator => ROLE_DIGIT_GROUP_SEPARATOR,
        _ => ROLE_UNSTATED,
    }
}

/// The candidates left once the caller's frame and role have removed what their document
/// rules out.
///
/// A narrowing that would leave nothing leaves the set alone: a declared frame no listing
/// permits is a caller stating something about their document that Appendix A does not
/// record, which `jlreq::diagnose` reports and which is never a reason to answer with no
/// class at all.
fn narrow(member: Member, frame: Frame, role: Role) -> ClassSet {
    let all = listed_classes(member);
    let by_remarks = rows(member)
        .filter(|row| permits_frame(row.frames, frame) && permits_role(row.role, role))
        .fold(ClassSet::EMPTY, |set, row| set.with(row.class));
    let kept = keep(all, by_remarks);
    let by_advance = keep(kept, advance_rule(kept, frame));
    let by_rule = keep(by_advance, western_rule(by_advance, frame));
    keep(by_rule, intersect(by_rule, selected_by(role)))
}

/// `narrowed` when it removed something without removing everything, and `set` otherwise.
fn keep(set: ClassSet, narrowed: ClassSet) -> ClassSet {
    if narrowed.is_empty() { set } else { narrowed }
}

/// The classes in both sets.
fn intersect(set: ClassSet, other: ClassSet) -> ClassSet {
    set.classes()
        .filter(|class| other.contains(*class))
        .fold(ClassSet::EMPTY, ClassSet::with)
}

/// Whether a Remarks cell permits a declared frame.
///
/// A cell that states no frame restricts none, which is why the empty mask is kept rather
/// than read as "the ideographic frame": §A.19's 465 rows and six of §A.25's state nothing
/// in the Remarks column and are not thereby full-width only.
const fn permits_frame(frames: u8, frame: Frame) -> bool {
    let declared = frame_bit(frame);
    frames == FRAMES_UNSTATED || declared == FRAMES_UNSTATED || frames & declared != 0
}

/// Whether a Remarks cell permits a declared role.
///
/// The column names a role for four cells — the decimal point and the digit-grouping
/// separator — so a cell naming one of them is refused only by a caller who declared the
/// other.
const fn permits_role(named: u8, role: Role) -> bool {
    let declared = remark_role(role);
    named == ROLE_UNSTATED || declared == ROLE_UNSTATED || named == declared
}

/// §3.1.2, which states the character advance of five classes and thereby says which frames
/// they are set on.
///
/// Commas (cl-07), full stops (cl-06), opening brackets (cl-01), closing brackets (cl-02)
/// and middle dots (cl-05): §3.1.2 states their advance as half-width, and Table 1's amount
/// is what "makes them appear as if they were intrinsically full-width". ADR 0017 reads that
/// as the two declarations `Frame::FullEm` and `Frame::HalfEm` — the same geometry reached
/// from opposite directions, the conditional space trimmed out of the advance or added to
/// it. A third-em, quarter-em or proportional advance is neither, so a caller declaring one
/// has stated something about their own document that rules those five classes out.
///
/// Without this the frame narrowed nothing at all on the commonest ambiguity in mixed text:
/// `U+0028` is named by cl-01, cl-25, cl-27 and cl-28 and its cl-01 Remarks cell is empty,
/// so a declared proportional frame left cl-01 standing and §3.2.6 — which puts Western text
/// set with a proportional font in cl-27 — could not fire, because §3.2.6 is reached below
/// only where cl-19 is also a candidate. The Remarks column supplies the frame for 834 rows
/// and this section supplies it for the five classes whose rows state nothing.
///
/// The empty set means "this rule narrows nothing here", which [`keep`] reads: a member
/// whose only candidates are those five keeps them, because a declared frame Appendix A does
/// not record is a diagnostic and never a reason to answer with no class at all.
///
/// JLReq: §3.1.2
fn advance_rule(candidates: ClassSet, frame: Frame) -> ClassSet {
    let stated = candidates
        .classes()
        .filter(|class| class.advance_is_stated_half_width())
        .fold(ClassSet::EMPTY, ClassSet::with);
    if stated.is_empty() {
        return ClassSet::EMPTY;
    }
    match frame {
        Frame::ThirdEm | Frame::QuarterEm | Frame::Proportional => {
            stated.classes().fold(candidates, ClassSet::without)
        },
        // `Frame::FullEm` and `Frame::HalfEm` are the two readings ADR 0017 makes one
        // geometry; `Frame::Unstated` has no representation here at all, because
        // `Text::new` refuses a stream that leaves the frame unstated on exactly these five
        // classes. `Frame` is `#[non_exhaustive]`, and a frame added later states nothing
        // about §3.1.2 until it is written here, which widens an answer rather than
        // narrowing it wrongly.
        _ => ClassSet::EMPTY,
    }
}

/// §3.2.4 and §3.2.6, which are the rule that separates cl-19 from cl-27.
///
/// The Remarks column marks every one of §A.27's 778 rows proportional and none of §A.19's
/// 465 rows anything at all, so the column alone rules out cl-27 on the ideographic frame
/// and rules out nothing on the proportional one. §3.2.6 is what supplies the other half:
/// Western characters set with a proportional font are cl-27, so cl-19 does not survive a
/// declared proportional frame where cl-27 is also a candidate.
///
/// The empty set means "this rule narrows nothing here", which [`keep`] reads.
///
/// JLReq: §3.2.4, §3.2.6
fn western_rule(candidates: ClassSet, frame: Frame) -> ClassSet {
    if !(candidates.contains(Class::Ideographic) && candidates.contains(Class::Western)) {
        return ClassSet::EMPTY;
    }
    match frame {
        Frame::Proportional => candidates.without(Class::Ideographic),
        Frame::FullEm => candidates.without(Class::Western),
        _ => ClassSet::EMPTY,
    }
}

/// Which axes would separate the candidates that survived.
fn needed_axes(member: Member, frame: Frame, role: Role, survivors: ClassSet) -> AxisSet {
    let mut needs = AxisSet::EMPTY;
    if frame == Frame::Unstated
        && FRAMES
            .into_iter()
            .any(|candidate| narrow(member, candidate, role) != survivors)
    {
        needs = needs.with(AxisSet::FRAME);
    }
    if role == Role::Unstated
        && ROLES
            .into_iter()
            .any(|candidate| narrow(member, frame, candidate) != survivors)
    {
        needs = needs.with(AxisSet::ROLE);
    }
    if survivors.classes().any(Class::is_construct_membership) {
        needs = needs.with(AxisSet::CONSTRUCT);
    }
    needs
}

/// Classify one occurrence: the shared implementation both entry points call.
fn examine(cluster: &str, item: Option<&Item>, policy: Policy) -> Classified {
    let Some(item) = item else {
        return Classified::NoSuchItem;
    };
    let mut keys = members(cluster);
    let Some((_, member)) = keys.next() else {
        return Classified::Unlisted;
    };
    if keys.next().is_some() {
        // Several keys in one item. `Text::new` accepts that only for a Western ligature on
        // the proportional frame, which §3.2.6 puts in cl-27 whole; there is no amount and
        // no break inside one for a per-key answer to have been about (ADR-0018).
        //
        // The chain opens at §3.9.2 like every other answer's. Opening it at §3.2.6 made the
        // same class decided by the same sentence report two different chains depending on
        // whether the caller's shaper produced a ligature, so a conformance case keyed on
        // provenance would pass or fail on a property of the caller's font stack.
        let why = Provenance::of(GROUPING, Standing::Normative);
        return Classified::One(Answer::new(
            Class::Western,
            why.then(PROPORTIONAL_WESTERN, Standing::Normative)
                .unwrap_or(why),
        ));
    }

    let frame = effective_frame(item.frame(), member);
    let role = item.role();
    let survivors = narrow(member, frame, role);
    let change = reclassification(member, survivors, policy);
    if let Some(change) = change {
        let why = Provenance::of(GROUPING, Standing::Normative)
            .then(change.rule(), Standing::Alternative);
        return match why {
            Some(why) => Classified::One(Answer::new(change.to(), why)),
            None => Classified::Irreducible {
                candidates: ClassSet::of(change.to()),
                why: Provenance::of(change.rule(), Standing::Alternative),
            },
        };
    }
    if survivors.is_empty() {
        return Classified::Unlisted;
    }
    if let Some(class) = survivors.only() {
        return Classified::One(Answer::new(class, decided(member, frame, class)));
    }
    let needs = needed_axes(member, frame, role, survivors);
    if needs.is_empty() {
        Classified::Irreducible {
            candidates: survivors,
            why: Provenance::of(GROUPING, Standing::Unstated),
        }
    } else {
        Classified::Several {
            candidates: survivors,
            needs,
        }
    }
}

/// The frame in force at one occurrence: the caller's where they declared one, and the one
/// the code point asserts about itself where they did not.
///
/// Appendix A's preamble lists `U+0028` where real Japanese text carries `U+FF08`, so a
/// caller who wrote the compatibility form has thereby stated that the frame is full-width
/// whether or not they also said so (§A preamble). A declared frame always wins: ADR 0002
/// makes the caller authoritative, and a contradiction between the two is
/// `jlreq::diagnose`'s to report rather than this function's to resolve.
///
/// JLReq: §A preamble, §3.2.4, §3.2.6
fn effective_frame(declared: Frame, member: Member) -> Frame {
    if declared == Frame::Unstated {
        asserted_frame(member).unwrap_or(Frame::Unstated)
    } else {
        declared
    }
}

/// The provenance of a decided class: §3.9.2, and the rules the caller's frame let fire.
///
/// The chain always opens at §3.9.2, which is the section that groups characters into
/// classes and hands the membership to Appendix A. Every answer this crate produces rests on
/// it, so a conformance case keyed on provenance compares like with like whatever the
/// caller's shaper produced.
///
/// Which Appendix A section listed the key is not a step of the chain: it is a property of
/// the answered class, published as [`Class::enumeration`], and the rule inventory's scope
/// deliberately holds Appendix A as data rather than as rules (`xtask/src/inventory.rs`).
fn decided(member: Member, frame: Frame, class: Class) -> Provenance {
    let listed = listed_classes(member);
    let mut why = Provenance::of(GROUPING, Standing::Normative);
    for rule in separating_rules(listed, frame, class).into_iter().flatten() {
        why = why.then(rule, Standing::Normative).unwrap_or(why);
    }
    why
}

/// The rules the declared frame fired over one key, in the order [`narrow`] applies them.
fn separating_rules(listed: ClassSet, frame: Frame, class: Class) -> [Option<RuleId>; 2] {
    let advance = (!class.advance_is_stated_half_width()
        && listed.classes().any(Class::advance_is_stated_half_width)
        && matches!(
            frame,
            Frame::ThirdEm | Frame::QuarterEm | Frame::Proportional
        ))
    .then_some(HALF_WIDTH_ADVANCE);
    let western = if listed.contains(Class::Ideographic) && listed.contains(Class::Western) {
        match (frame, class) {
            (Frame::FullEm, Class::Ideographic) => Some(FULL_WIDTH_WESTERN),
            (Frame::Proportional, Class::Western) => Some(PROPORTIONAL_WESTERN),
            _ => None,
        }
    } else {
        None
    };
    [advance, western]
}

/// The reclassification in force at one occurrence, if the policy puts one there.
fn reclassification(
    member: Member,
    candidates: ClassSet,
    policy: Policy,
) -> Option<Reclassification> {
    RECLASSIFICATIONS.iter().copied().find(|change| {
        change.applies_to(member, candidates)
            && policy.get(change.when().question()) == change.when()
    })
}

/// This project's published reading of §3.9.2's conceded ambiguity: the lowest-numbered
/// surviving class an occurrence can be in without belonging to a construct.
///
/// §3.9.2 concedes the case with the example "エディター（editor）は……" and says the
/// Japanese design of the brackets "is better", which is a preference rather than a rule.
/// Appendix A numbers the Japanese classes before the Western ones — cl-01 through cl-26
/// ahead of cl-27 — so taking the lowest-numbered survivor is that preference made
/// mechanical, and it is the reading recorded in `docs/decisions/ambiguous-context.md`.
///
/// # Why the construct classes are passed over
///
/// Nine classes are membership *in* a construct, and four of them — cl-24, cl-25, cl-28 and
/// cl-29 — enumerate ordinary Western and punctuation keys and are numbered below cl-27.
/// [`classify`] takes no construct axis, so no occurrence it is given is inside a grouped
/// numeral (連数字), a unit symbol or a warichu (割注) as far as this crate can know; taking
/// the lowest-numbered survivor over all of them therefore answered "a character inside a
/// unit symbol" for every proportional Latin letter in a Japanese document, which is a
/// statement about the caller's text that nothing in the caller's text supports. The reading
/// is the lowest-numbered survivor *the supplied facts can reach*, and the construct classes
/// are reached only when nothing else survives — where they are still the honest answer,
/// because Appendix A named them and named nothing else.
///
/// [`Classified::Several`] continues to report them, with [`AxisSet::CONSTRUCT`] naming the
/// axis that would settle it. This narrowing is `resolve`'s and not `classify`'s: the
/// candidates are the specification's, and only the tie-break is ours.
///
/// Marked [`Standing::Unstated`], because the specification decides nothing here.
///
/// JLReq: §3.9.2, §A.20–§A.25, §A.28–§A.30
fn ambiguous_context(candidates: ClassSet) -> Answer<Class> {
    let reachable = candidates
        .classes()
        .find(|class| !class.is_construct_membership());
    match reachable.or_else(|| candidates.classes().next()) {
        Some(class) => Answer::new(class, Provenance::of(GROUPING, Standing::Unstated)),
        // Unreachable over an answer this crate produces: `examine` reports `Unlisted`
        // rather than an empty candidate set, so the two ambiguous variants always name at
        // least two classes. Stated rather than assumed, and it answers the way an unlisted
        // key answers, because a key no class names is exactly what that is.
        None => unlisted_code_point(Frame::Unstated),
    }
}

/// This project's published reading of the silence §3.9.2 records: cl-27 on a proportional
/// frame and cl-19 otherwise.
///
/// §3.9.2 states that JIS X 4051 leaves it implementation-defined whether a character it
/// does not list belongs to a class, and JLReq inherits that. The reading extends the one
/// distinction JLReq does draw over unenumerated Western text: §3.2.6 puts a proportional
/// occurrence in cl-27 and §3.2.4 puts a full-width or fixed-width one with the ideographic
/// characters. It is recorded in `docs/decisions/unlisted-code-point.md`.
///
/// Marked [`Standing::Unstated`], because the specification decides nothing here.
///
/// JLReq: §3.9.2, §3.2.4, §3.2.6
fn unlisted_code_point(frame: Frame) -> Answer<Class> {
    let class = if frame == Frame::Proportional {
        Class::Western
    } else {
        Class::Ideographic
    };
    Answer::new(class, Provenance::of(GROUPING, Standing::Unstated))
}

#[cfg(test)]
mod tests {
    use jlreq_spec::{Policy, Standing};
    use jlreq_unit::{
        Advance, ByteOffset, Frame, InlineExtent, Item, ItemIndex, Role, Scale, ScaleId,
    };

    use super::{
        AxisSet, Classified, FRAMES, RECLASSIFICATIONS, ROLES, classify, frame_bit, resolve,
    };
    use crate::class::{Class, ClassSet};
    use crate::generated::appendix_a::FRAMES_UNSTATED;
    use crate::member::Member;
    use crate::text::Text;

    /// A one-em square size, which every stream below declares first.
    fn base() -> Scale {
        Scale::square(Advance::new(1000).unwrap()).expect("a positive em")
    }

    /// One item at the head of a stream, with the frame and role a caller declared.
    fn occurrence(frame: Frame, role: Role) -> ([Item; 1], [Scale; 1]) {
        (
            [Item::new(
                ByteOffset::new(0),
                InlineExtent::new(1000).unwrap(),
                ScaleId::BASE,
            )
            .with_frame(frame)
            .with_role(role)],
            [base()],
        )
    }

    /// What `classify` answers for one occurrence of `text`.
    fn answer(text: &str, frame: Frame, role: Role) -> Classified {
        let (items, scales) = occurrence(frame, role);
        let stream = Text::new(text, &items, &scales).expect("one well-formed occurrence");
        classify(stream, ItemIndex::new(0), Policy::JLREQ)
    }

    /// The one class `classify` decided, or `None` when it did not decide one.
    fn decided(text: &str, frame: Frame, role: Role) -> Option<Class> {
        match answer(text, frame, role) {
            Classified::One(one) => Some(one.value()),
            _ => None,
        }
    }

    /// What `resolve` answers for one occurrence of `text`.
    fn resolved(text: &str, frame: Frame, role: Role) -> (Class, Standing) {
        let (items, scales) = occurrence(frame, role);
        let stream = Text::new(text, &items, &scales).expect("one well-formed occurrence");
        let answer = resolve(stream, ItemIndex::new(0), Policy::JLREQ)
            .expect("the stream has an item at ordinal zero");
        (answer.value(), answer.why().standing())
    }

    #[test]
    fn a_kanji_the_appendix_does_not_list_is_still_ideographic() {
        assert_eq!(
            decided("漢", Frame::FullEm, Role::Unstated),
            Some(Class::Ideographic),
            "§A.19 lists 465 rows and Unified_Ideograph covers 101 996 code points, so a \
             lookup that read only the table would answer Unlisted for almost every kanji"
        );
    }

    #[test]
    fn a_hiragana_is_hiragana_whatever_the_caller_states() {
        for frame in FRAMES {
            assert_eq!(
                decided("あ", frame, Role::Unstated),
                Some(Class::Hiragana),
                "§A.15 names あ and nothing else does, so no axis has anything to separate"
            );
        }
    }

    #[test]
    fn the_answer_of_a_single_class_key_cites_the_section_that_groups_characters() {
        let Classified::One(one) = answer("あ", Frame::FullEm, Role::Unstated) else {
            panic!("§A.15 names あ under one class");
        };
        assert_eq!(one.why().standing(), Standing::Normative);
        assert!(
            one.why().rules().eq([super::GROUPING]),
            "the table lookup rests on §3.9.2, and nothing else fired"
        );
    }

    #[test]
    fn a_full_width_numeral_is_ideographic_and_a_proportional_one_is_western() {
        assert_eq!(
            decided("1", Frame::FullEm, Role::Unstated),
            Some(Class::Ideographic),
            "§3.2.4 sets full-width and fixed-width European numerals as quasi-ideographs"
        );
        assert_eq!(
            answer("1", Frame::Proportional, Role::Unstated),
            Classified::Several {
                candidates: ClassSet::of(Class::InUnitSymbol).with(Class::Western),
                needs: AxisSet::ROLE.with(AxisSet::CONSTRUCT),
            },
            "§3.2.6 rules out cl-19, and §A.25 permits a numeral inside a unit symbol, \
             which the role would settle and which no other fact of one item can"
        );
        assert_eq!(
            decided("1", Frame::Proportional, Role::QuantitySymbol),
            Some(Class::Western),
            "and §C.2 note 11's quantity symbol is what settles it"
        );
    }

    #[test]
    fn the_rule_that_separated_cl_19_from_cl_27_is_named_in_the_provenance() {
        let Classified::One(one) = answer("1", Frame::FullEm, Role::Unstated) else {
            panic!("the ideographic frame decides a numeral");
        };
        assert!(
            one.why()
                .rules()
                .eq([super::GROUPING, super::FULL_WIDTH_WESTERN]),
            "the table named the candidates and §3.2.4 chose between them, which is exactly \
             the two-step chain `Provenance` is bounded at"
        );
    }

    #[test]
    fn an_unstated_frame_on_a_numeral_reports_the_frame_as_the_separating_axis() {
        let Classified::Several { needs, .. } = answer("1", Frame::Unstated, Role::Unstated) else {
            panic!("four classes name U+0031");
        };
        assert!(
            needs.contains(AxisSet::FRAME) && needs.contains(AxisSet::CONSTRUCT),
            "the frame separates cl-19 from cl-27 and the construct separates cl-24 from \
             both, which is the pair §3.9.2's own example turns on"
        );
        assert!(!needs.is_empty());
    }

    #[test]
    fn the_role_narrows_a_numeral_to_the_class_whose_section_is_about_that_job() {
        assert_eq!(
            decided("1", Frame::HalfEm, Role::UnitSymbol),
            Some(Class::InUnitSymbol),
            "§A.25 is the characters permitted inside a unit symbol"
        );
        assert_eq!(
            decided("1", Frame::HalfEm, Role::DigitGroupSeparator),
            Some(Class::InGroupedNumeral),
            "§A.24 is the characters permitted inside a grouped numeral (連数字)"
        );
    }

    #[test]
    fn a_frame_no_listing_permits_is_a_diagnostic_and_not_an_empty_answer() {
        assert_eq!(
            decided("\u{02E5}", Frame::FullEm, Role::Unstated),
            Some(Class::Western),
            "§A.27 marks the tone bar proportional and the caller declared the ideographic \
             frame; the contradiction is `jlreq::diagnose`'s to report, and narrowing to \
             nothing would answer with no class at all"
        );
    }

    #[test]
    fn a_code_point_no_table_names_is_unlisted() {
        assert_eq!(
            answer("\u{1F600}", Frame::FullEm, Role::Unstated),
            Classified::Unlisted,
            "most of Unicode; §3.9.2 records that JIS X 4051 leaves this \
             implementation-defined"
        );
    }

    #[test]
    fn an_unlisted_member_resolves_to_this_projects_published_reading() {
        assert_eq!(
            resolved("\u{1F600}", Frame::FullEm, Role::Unstated),
            (Class::Ideographic, Standing::Unstated),
            "the standing is what tells a caller this is not the specification's answer"
        );
        assert_eq!(
            resolved("\u{1F600}", Frame::Proportional, Role::Unstated),
            (Class::Western, Standing::Unstated),
            "the reading extends the one distinction §3.2.4 and §3.2.6 do draw"
        );
    }

    #[test]
    fn plain_proportional_western_text_is_western_and_not_a_character_inside_a_unit_symbol() {
        // §A names U+0041 under cl-19, cl-25 and cl-27; §3.2.6 removes cl-19 on the
        // proportional frame and cl-25 is membership in a construct the caller never
        // declared. Taking the lowest-numbered survivor over all three answered "a character
        // inside a unit symbol" — 25 < 27 — for every Latin letter in a Japanese document.
        for text in ["A", "e", "1"] {
            assert_eq!(
                resolved(text, Frame::Proportional, Role::Unstated),
                (Class::Western, Standing::Unstated),
                "`{text}` set with a proportional font is §3.2.6's own subject"
            );
        }
    }

    #[test]
    fn a_proportional_bracket_is_western_because_section_3_1_2_states_cl_01s_advance() {
        // U+0028's cl-01 Remarks cell is empty, so the Remarks column narrows nothing and
        // §3.2.6 cannot fire — it separates cl-19 from cl-27 and U+0028 has no cl-19 row.
        // §3.1.2 is what states that an opening bracket's advance is half-width, so a
        // declared proportional advance is one it does not state.
        assert_eq!(
            resolved("(", Frame::Proportional, Role::Unstated),
            (Class::Western, Standing::Unstated),
            "§3.2.6 puts Western text set with a proportional font in cl-27, and the caller \
             declared the frame that says so"
        );
        assert_eq!(
            resolved("（", Frame::FullEm, Role::Unstated),
            (Class::OpeningBracket, Standing::Unstated),
            "and §3.9.2's own example — 「エディター（editor）は」 — is the full-width \
             bracket, which is unchanged"
        );
    }

    #[test]
    fn the_advance_rule_names_the_section_that_stated_the_advance() {
        let Classified::One(one) = answer("\u{003A}", Frame::Proportional, Role::Unstated) else {
            panic!("§A names U+003A under cl-05 and cl-27, and the frame separates them");
        };
        assert_eq!(one.value(), Class::Western);
        assert!(
            one.why()
                .rules()
                .eq([super::GROUPING, super::HALF_WIDTH_ADVANCE]),
            "the table named the candidates and §3.1.2 ruled out the one whose advance it \
             states"
        );
    }

    #[test]
    fn the_advance_rule_leaves_a_class_section_3_1_2_does_not_name_alone() {
        assert_eq!(
            resolved("%", Frame::Proportional, Role::Unstated),
            (Class::PostfixedAbbreviation, Standing::Unstated),
            "§A.13's rows are marked proportionally-spaced, so a proportional percent sign \
             is a postfixed abbreviation and cl-27 is the wrong answer; §3.1.2 states the \
             advance of five classes and cl-13 is not one of them"
        );
        assert_eq!(
            resolved("\u{0020}", Frame::QuarterEm, Role::Unstated),
            (Class::WesternWordSpace, Standing::Unstated),
            "§A names U+0020 under cl-24, cl-25 and cl-26; the first two are membership in a \
             construct, so the Western word space is what a caller who declared none has"
        );
    }

    #[test]
    fn an_ambiguity_resolves_to_the_lowest_numbered_survivor() {
        assert_eq!(
            resolved("（", Frame::FullEm, Role::Unstated),
            (Class::OpeningBracket, Standing::Unstated),
            "§A names U+0028 under cl-01, cl-25 and cl-28; §3.9.2 says the Japanese design \
             is better, and Appendix A numbers the Japanese classes first"
        );
    }

    #[test]
    fn a_decided_class_resolves_to_itself_with_the_specifications_own_standing() {
        assert_eq!(
            resolved("あ", Frame::FullEm, Role::Unstated),
            (Class::Hiragana, Standing::Normative),
            "resolve is classify plus at most one step, and it took none here"
        );
    }

    #[test]
    fn an_ordinal_naming_no_item_is_not_an_unlisted_key() {
        let (items, scales) = occurrence(Frame::FullEm, Role::Unstated);
        let stream = Text::new("あ", &items, &scales).expect("one occurrence");
        assert_eq!(
            classify(stream, ItemIndex::new(9), Policy::JLREQ),
            Classified::NoSuchItem,
            "`there is no member at this ordinal` and `the specification lists no class for \
             the member at this ordinal` are two different answers"
        );
        assert_eq!(
            resolve(stream, ItemIndex::new(9), Policy::JLREQ),
            None,
            "an occurrence that is not there has no class, and answering one would put a \
             fabricated ideograph into a loop over `0..=items.len()` that no caller could \
             tell from a real one"
        );
    }

    #[test]
    fn a_western_ligature_is_cl_27_whole() {
        let items = [Item::new(
            ByteOffset::new(0),
            InlineExtent::new(1000).unwrap(),
            ScaleId::BASE,
        )
        .with_frame(Frame::Proportional)];
        let scales = [base()];
        let stream = Text::new("ffi", &items, &scales).expect("a Western ligature");
        let Classified::One(one) = classify(stream, ItemIndex::new(0), Policy::JLREQ) else {
            panic!("§3.2.6 puts a proportional Western cluster in cl-27 whole");
        };
        assert_eq!(one.value(), Class::Western);
        assert!(
            one.why()
                .rules()
                .eq([super::GROUPING, super::PROPORTIONAL_WESTERN]),
            "the chain opens at §3.9.2 like every other answer's, so a case keyed on \
             provenance does not pass or fail on whether the caller's shaper made a ligature"
        );
    }

    #[test]
    fn the_folded_key_answers_where_the_literal_one_is_listed_nowhere() {
        assert_eq!(
            decided("）", Frame::HalfEm, Role::Unstated),
            None,
            "U+FF09 folds onto U+0029, which §A names under cl-02, cl-25, cl-27 and cl-29"
        );
        assert!(
            matches!(
                answer("）", Frame::HalfEm, Role::Unstated),
                Classified::Several { candidates, .. } if candidates.contains(Class::ClosingBracket)
            ),
            "a library that did not fold would give wrong classes on ordinary text, silently"
        );
    }

    #[test]
    fn the_ideographic_space_is_never_folded_onto_the_western_word_space() {
        assert_eq!(
            decided("\u{3000}", Frame::FullEm, Role::Unstated),
            Some(Class::IdeographicSpace),
            "U+3000 has Decomposition_Type=Wide onto U+0020, so a lookup that folded before \
             trying the literal key would answer cl-26 for the ideographic space"
        );
    }

    #[test]
    fn an_answer_is_ambiguous_exactly_when_the_facts_did_not_decide_it() {
        assert!(!answer("あ", Frame::FullEm, Role::Unstated).is_ambiguous());
        assert!(answer("1", Frame::Unstated, Role::Unstated).is_ambiguous());
        assert!(answer("\u{1F600}", Frame::FullEm, Role::Unstated).is_ambiguous());
    }

    #[test]
    fn every_frame_of_the_vocabulary_has_a_bit_and_the_unstated_one_has_none() {
        for frame in FRAMES {
            assert_ne!(
                frame_bit(frame),
                FRAMES_UNSTATED,
                "{frame:?} is a frame the Remarks column can state"
            );
        }
        assert_eq!(frame_bit(Frame::Unstated), FRAMES_UNSTATED);
    }

    #[test]
    fn every_role_of_the_vocabulary_selects_something_or_everything() {
        for role in ROLES {
            let selected = super::selected_by(role);
            assert!(
                !selected.is_empty(),
                "{role:?} selects at least one class, so a narrowing by it is never empty \
                 for a reason nobody wrote down"
            );
        }
        assert_eq!(
            super::selected_by(Role::Unstated),
            ClassSet::ALL,
            "an unstated role rules out nothing"
        );
    }

    #[test]
    fn no_reclassification_is_in_force_until_the_policy_space_is_generated() {
        assert!(
            RECLASSIFICATIONS.is_empty(),
            "§C.2's three notes become data with the note table and the policy space; one \
             invented here would publish an alternative the specification does not permit"
        );
    }

    #[test]
    fn a_subject_names_a_class_a_member_or_an_adjacency() {
        // §C.3's levels differ precisely on this: Very loose relaxes cl-05, cl-09 and cl-13
        // as whole classes where Loose relaxes `・`, `々` and `%` as single members of the
        // same classes, so a subject typed as a class cannot tell the two levels apart.
        let whole = super::Subject::Class(Class::IterationMark);
        let one = super::Subject::Member(Member::single('々'));
        let adjacency = super::Subject::Pair(Member::single('々'), Member::single('々'));
        assert_ne!(whole, one);
        assert_ne!(one, adjacency);
        assert_ne!(whole, adjacency);
        assert!(adjacency.is_boundary());
        assert!(!whole.is_boundary() && !one.is_boundary());
    }

    #[test]
    fn a_folded_key_is_not_a_key_appendix_a_lists_under_cl_27() {
        assert!(
            super::literally_listed_classes(Member::single('A')).contains(Class::Western),
            "§A.27 lists U+0041"
        );
        assert!(
            !super::literally_listed_classes(Member::single('\u{FF21}')).contains(Class::Western),
            "and not U+FF21, which §3.2.4 puts with the ideographs; ADR 0018's Western \
             ligature is about the keys the shaper produced"
        );
        assert!(
            super::listed_classes(Member::single('\u{FF21}')).contains(Class::Western),
            "the folded reading is the right one for `what class is this occurrence`, which \
             is why the two are two functions"
        );
    }

    #[test]
    fn a_sentence_of_ordinary_japanese_classifies_end_to_end() {
        // 「日本語の組版。」 — an opening bracket, four ideographs and two kana, a
        // sentence-final full stop, and a closing bracket. Every occurrence is one Appendix
        // A key, and the two brackets and the full stop declare the frame §3.1.2 requires.
        let text = "「日本語の組版。」";
        let scales = [base()];
        let items: [Item; 9] = core::array::from_fn(|position| {
            let start = u32::try_from(position.saturating_mul(3)).expect("nine short items");
            let item = Item::new(
                ByteOffset::new(start),
                InlineExtent::new(1000).unwrap(),
                ScaleId::BASE,
            );
            match position {
                0 | 7 | 8 => item.with_frame(Frame::FullEm),
                _ => item,
            }
        });
        let stream = Text::new(text, &items, &scales).expect("one item per key");

        let classes: [Class; 9] = core::array::from_fn(|position| {
            let index = ItemIndex::new(u32::try_from(position).expect("nine short items"));
            resolve(stream, index, Policy::JLREQ)
                .expect("nine items, nine ordinals")
                .value()
        });
        assert_eq!(
            classes,
            [
                Class::OpeningBracket,
                Class::Ideographic,
                Class::Ideographic,
                Class::Ideographic,
                Class::Hiragana,
                Class::Ideographic,
                Class::Ideographic,
                Class::FullStop,
                Class::ClosingBracket,
            ],
            "the five kanji are listed nowhere in §A.19's table and come from the \
             ideograph predicate, which is where the 101 996 code points the appendix \
             deliberately does not enumerate are"
        );

        let standings: [Standing; 9] = core::array::from_fn(|position| {
            let index = ItemIndex::new(u32::try_from(position).expect("nine short items"));
            resolve(stream, index, Policy::JLREQ)
                .expect("nine items, nine ordinals")
                .why()
                .standing()
        });
        assert_eq!(
            standings,
            [Standing::Normative; 9],
            "and every one of the nine is the specification's own answer: §A names the two \
             corner brackets under cl-01 and cl-02 and nothing else, unlike the ASCII \
             parenthesis, which §A.25 and §A.28 name as well"
        );
    }
}
