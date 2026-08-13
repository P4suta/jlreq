// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The evaluator: one function and one small set of policy-conditional overrides.
//!
//! The evaluator holds no specification knowledge of its own beyond the shape of the six
//! tables and the handful of notes wired in below: every amount, every breakability and
//! every placement is read from the generated matrices through [`crate::raw`]'s cell
//! shapes, which are themselves a real accessor over `spec/captured/`'s transcription
//! rather than a second copy of it (see `src/lib.rs`).
//!
//! # Coverage, stated rather than assumed
//!
//! [`boundary`] is total over every adjacency between two of the twenty-eight classes
//! Tables 1 through 6 carry, over the line-head row and over the line-end column. For
//! cl-17 (math symbols) and cl-18 (math operators) — which §3.7.4 governs directly rather
//! than through a table cell, and which are absent from every one of the six matrices for
//! that reason (`crate::generated`'s module doc) — the six tables hold no cell, so
//! [`boundary`] answers with no conditional space, an unrestricted [`crate::Breakable`]
//! and a permitted [`crate::Placement`] for such an adjacency: an honest statement that no
//! table constrains it, not a guess at the quarter-em and solid settings §3.7.4 states in
//! prose. Implementing §3.7.4 itself is future work, named here rather than silently
//! absent.
//!
//! Of the forty-seven appendix notes, this evaluator wires in: the two that ADR-0014
//! itself turns on (§B.2#3, §B.2#5, read directly out of Table 1's multi-term cells,
//! needing no override at all); the three that split a two-term cell's single captured
//! reduction stage back into its two referents (§D.2#1, #2, #3, in this module's
//! `special_reduction`); the same-run delegation to a placement procedure (§B.2#9, #10,
//! #11); the five policy questions whose answer changes a Table 1 amount, synthesizes a term
//! at a coordinate Table 1 states none at all for, or changes a ruby-overhang permission
//! directly (§B.2#2 `spacing.line_end_punctuation`, §B.2#6 `spacing.line_end_full_stop_comma`,
//! §B.2#7 `ruby.overhang_kana`, §3.1.6's third Note `spacing.sentence_medial_dividing_mark` in
//! `sentence_medial_dividing_mark_spaces`, scoped to the coordinates
//! `docs/decisions/sentence-medial-dividing-mark.md` states rather than every coordinate a
//! sentence-medial cl-04 occurrence can touch — §3.1.6's own *first* Note, the sentence-final
//! "one em" this project reads as an inserted cl-14 character rather than an inter-character
//! spacing amount, is still wired nowhere in this crate, a caller-level composition decision
//! and not this evaluator's own to make; and §3.1.5 Figure 71 pattern 2, identified by §B.2#17's
//! own cross-reference as that note's "conditional half em spacing" alternative,
//! `spacing.line_head_opening_bracket` in `line_head_opening_bracket_space`, scoped to the
//! wrapped-line-head half of the pattern rather than the paragraph first-line indent Figure 71
//! also states, which is `jlreq_line::Paragraph`'s own declared amount and not a Table 1
//! coordinate at all — `docs/decisions/line-head-opening-bracket.md`'s own reading states the
//! referent, the reduction and the citation this synthesis carries); the three breakability
//! notes whose Table 2 cell
//! carries an empty `levels` bitmask because the note's own condition, not a kinsoku
//! strictness level, decides the boundary (`note_governed_refusal`): §C.2#5's five ordered
//! member pairs of cl-08 against itself, §C.2#10's `kinsoku.grouped_numeral_before_western` at
//! cl-24 against cl-27, and §C.2#11's quantity-symbol role or European-numeral key at cl-27
//! against cl-13 — the second half of that last one adjudicated rather than read from a
//! declared role (`docs/decisions/european-numeral-by-code-point.md`); and the two expansion
//! notes whose Table 6 cell carries a real quarter-em ceiling that only applies under the
//! citing note's own stated condition, mirrored one table over from `note_governed_refusal`
//! by `note_governed_expansion`: §E.2#10 reuses §C.2#11's own quantity-symbol role or
//! European-numeral key verbatim — the identical exception in the identical words over the
//! identical ordered class pair — and §E.2#4 reads its own notion of a "kind" of inseparable
//! character (cl-08) from `docs/decisions/inseparable-character-kind.md` rather than from
//! §C.2#5's own closed five-pair enumeration, a different, order-specific fact about the same
//! class pair. Every other note is either already the shape Table 1, Table 2 or Appendix D's
//! legend states outright (so the generated cell alone answers it), belongs to kinsoku
//! relaxation or line breaking (`jlreq-line`, M1-b), or is not yet wired in and is named as
//! such at its own site rather than silently answered.

use jlreq_class::{Class, ClassSet, Member, Text, members, resolve};
use jlreq_spec::{Answer, Policy, Provenance, Question, RuleId, Standing};
use jlreq_unit::{
    Construct, ConstructKind, Direction, FormulaSetting, InlineEdge, ItemIndex, Role, RubyOverhang,
    Runs,
};

use crate::axis::{After, Before};
use crate::boundary::{Boundary, Breakable, Delegation, Placement};
use crate::raw::{self, RawHang, RawRangedCell, RawSpacingCell};
use crate::space::{
    ConditionalSpace, Expansion, ExpansionStage, Reduction, ReductionStage, Referent,
};

/// The predicate forms an override may take. Closed, and derived from the notes rather than
/// assumed — but nothing in `xtask` reads this enum (checked directly: no generator emits a
/// row against it, and no gate walks `spec/derived/rules.tsv` requiring some named form to
/// cover every note), so the claim that this set is complete is asserted by this doc comment
/// and by this module's own accounting of what it wires in, not checked by a build failure.
/// `xtask/src/direction.rs`'s own `direction` gate reads [`Predicate::InDirection`] alone,
/// for the narrower claim ADR-0011 makes: that no *other* form ever names a [`Direction`],
/// not that every note has a form here at all.
///
/// Not every form is load-bearing yet — [`Predicate::SameMember`], [`Predicate::InFormula`]
/// and [`Predicate::Relaxes`] name the shape a later note takes and are not yet matched by
/// [`boundary`], which is stated in this module's own doc rather than left for a reader to
/// discover by grep. [`Predicate::SameMember`] stays unmatched on purpose rather than moving
/// with [`Predicate::MemberPair`]: §C.2 note 5 forbids three same-member pairs and two
/// different-member ones and permits every other cl-08 pair including a member against
/// itself the note never lists (〵 against 〵), so a general "these two items are the same
/// member" test would answer wrongly at that coordinate; `boundary` reads the note's own
/// five ordered pairs directly instead, which is exactly [`Predicate::MemberPair`]'s shape.
///
/// JLReq: §B.2, §C.2, §C.3, §3.7.4
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Predicate {
    /// A general "these two items are the same member" test — not what §C.2 note 5 actually
    /// states: three of its five named pairs are identical-member and two are not, and it
    /// permits every other identical cl-08 pair the five do not name (〵 against itself,
    /// among others). Named for the shape a caller might reach for; see
    /// [`Predicate::MemberPair`] for the form the note's own closed enumeration takes.
    SameMember,
    /// §C.2 note 5's five ordered adjacencies; §C.3's ellipsis pair.
    MemberPair(Member, Member),
    /// §B.2#9–#11, §C.2#6–#8, §C.2#13.
    SameRun(ConstructKind),
    /// §B.2#9–#11, §C.2#6–#8, §C.2#13.
    DifferentRun(ConstructKind),
    /// §B.2#1, §B.2#7: the *other* item is in a construct.
    IsInConstruct(Referent, ConstructKind),
    /// §B.2#7's neighbor test.
    HasClass(Referent, ClassSet),
    /// §B.2#12, §C.2#11.
    HasRole(Referent, Role),
    /// §B.2#2, #4, #6, #13.
    AtEdge(InlineEdge),
    /// §3.1.3, §3.2.5, §3.3.5 — the three direction-conditional rules, and no others.
    /// This is the only form in which generated data may name a direction, and the
    /// `direction` gate checks that (ADR-0011).
    InDirection(Direction),
    /// §3.7.4 states four spacings for cl-17 and cl-18 against cl-21, cl-24 and cl-27: two
    /// for a formula in running text and two for one set on a line of its own.
    InFormula(FormulaSetting),
    /// A policy question is set a particular way; also how §E.1's cross-table coupling is
    /// expressed — adopting reduction Table 5 makes a Table 6 quarter em rigid.
    PolicyIs(Question, jlreq_spec::Choice),
    /// §C.3's level relaxations, whose subject is a class, a member, or an ordered pair.
    Relaxes(jlreq_class::Subject),
}

/// Everything an override's predicate can ask about an adjacency.
///
/// Constructed from a [`Text`], the [`Runs`] overlaying it, and the two item ordinals a
/// boundary sits between (or the line edge in their place), never by hand, so it cannot
/// disagree with the text it came from and there is exactly one carrier of run identity.
///
/// JLReq: §B, §C, §D, §E
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Adjacency<'r> {
    text: Text<'r>,
    runs: Runs<'r>,
    before: Option<ItemIndex>,
    after: Option<ItemIndex>,
    direction: Direction,
}

impl<'r> Adjacency<'r> {
    /// The boundary between item `before` and the item immediately after it, or `None` when
    /// `before` names no item of `text`, or names `text`'s last item.
    ///
    /// The last item's trailing boundary is [`Adjacency::at_line_end`], not this
    /// constructor, because there is no item after it for this shape to name: the bound is
    /// checked here rather than left to a caller's own loop discipline, so a caller who
    /// walks every adjacent pair with `between` alone and forgets to special-case the final
    /// iteration gets `None` — a value that forces a decision — rather than an
    /// out-of-range `after` silently resolving to no class and folding into the same
    /// unconstrained answer [`boundary`] gives cl-17/cl-18, indistinguishable from that
    /// legitimate case (`class_of`'s own doc states the invariant this bound now holds by
    /// construction).
    ///
    /// JLReq: §B, §C, §D, §E
    #[must_use]
    pub const fn between(
        text: Text<'r>,
        runs: Runs<'r>,
        before: ItemIndex,
        direction: Direction,
    ) -> Option<Self> {
        let len = text.items().len();
        if (before.get() as usize).saturating_add(1) >= len {
            return None;
        }
        Some(Self {
            text,
            runs,
            before: Some(before),
            after: Some(ItemIndex::new(before.get().saturating_add(1))),
            direction,
        })
    }

    /// The boundary before the first item of a line (or of an inline cutting note).
    ///
    /// JLReq: §B.1
    #[must_use]
    pub const fn at_line_head(
        text: Text<'r>,
        runs: Runs<'r>,
        first: ItemIndex,
        direction: Direction,
    ) -> Self {
        Self {
            text,
            runs,
            before: None,
            after: Some(first),
            direction,
        }
    }

    /// The boundary after the last item of a line (or of an inline cutting note).
    ///
    /// JLReq: §B.1
    #[must_use]
    pub const fn at_line_end(
        text: Text<'r>,
        runs: Runs<'r>,
        last: ItemIndex,
        direction: Direction,
    ) -> Self {
        Self {
            text,
            runs,
            before: Some(last),
            after: None,
            direction,
        }
    }

    /// The writing direction this adjacency is read under.
    const fn direction(self) -> Direction {
        self.direction
    }

    /// The preceding position: a class, or the line head.
    fn before_position(self) -> Option<Before> {
        match self.before {
            None => Some(Before::LineHead),
            Some(index) => class_of(self.text, index).map(Before::Class),
        }
    }

    /// The trailing position: a class, or the line end.
    fn after_position(self) -> Option<After> {
        match self.after {
            None => Some(After::LineEnd),
            Some(index) => class_of(self.text, index).map(After::Class),
        }
    }

    /// The construct the preceding item belongs to, if any.
    fn before_construct(self) -> Option<Construct> {
        self.before.and_then(|index| self.runs.of(index))
    }

    /// The construct the trailing item belongs to, if any.
    fn after_construct(self) -> Option<Construct> {
        self.after.and_then(|index| self.runs.of(index))
    }

    /// The preceding item's declared role, if there is a preceding item.
    fn before_role(self) -> Option<Role> {
        self.before
            .and_then(|index| self.text.items().get(index.get() as usize))
            .map(|item| item.role())
    }

    /// The trailing item's declared role, if there is a trailing item.
    fn after_role(self) -> Option<Role> {
        self.after
            .and_then(|index| self.text.items().get(index.get() as usize))
            .map(|item| item.role())
    }

    /// The preceding item's own Appendix A key, if there is a preceding item.
    fn before_member(self) -> Option<Member> {
        self.before.and_then(|index| member_of(self.text, index))
    }

    /// The trailing item's own Appendix A key, if there is a trailing item.
    fn after_member(self) -> Option<Member> {
        self.after.and_then(|index| member_of(self.text, index))
    }
}

/// Resolve one item's class, ignoring what the specification cannot decide over one
/// occurrence: [`resolve`] already folds ambiguity into this project's published reading
/// (`jlreq_class`'s own doc), so the only `None` this function passes through is "no item
/// at this ordinal", which `Adjacency`'s own constructors never produce for a valid text.
fn class_of(text: Text<'_>, index: ItemIndex) -> Option<Class> {
    resolve(text, index, Policy::JLREQ).map(jlreq_spec::Answer::value)
}

/// Resolve one item's own Appendix A key, for the notes that turn on member identity rather
/// than on class alone (§C.2 note 5's five ordered adjacencies, §C.2 note 11's European
/// numeral). [`Text::cluster`] answers the empty string past the end of `text`'s items, and
/// [`members`] yields nothing for an empty string, so the two `None`s already coincide
/// without a separate bounds check the way [`class_of`]'s own doc states for `resolve`. Takes
/// only the first `(span, Member)` the scan yields: a Western ligature is `Text`'s one
/// exception to "one item is one key" (`Text`'s own doc), so this answers that item's first
/// key rather than refusing it outright — harmless for both callers of this function, since
/// neither §C.2 note 5's cl-08 pairs nor §C.2 note 11's ten-digit set can be a ligature's own
/// first key.
fn member_of(text: Text<'_>, index: ItemIndex) -> Option<Member> {
    members(text.cluster(index))
        .next()
        .map(|(_, member)| member)
}

/// The three coordinates where one Table 1 cell carries two terms whose captured
/// reduction-table cell cannot state two stages at once (§D.2#1, #2, #3): each entry is one
/// `(before class number, after class number, is-the-trailing-term, reduction table)` and
/// the [`Reduction`] its own citing note states for that one term.
///
/// The general path — one captured `RawRangedCell` applied to a cell's one term — is
/// correct for every other cell of Tables 1, 3, 4 and 5; these three exist because §D.2#1
/// through #3 are the only notes where the two terms of one Table 1 cell reduce at
/// different priority stages, which one cell of one color cannot encode for both at once.
fn special_reduction(
    before: u8,
    after: u8,
    trailing: bool,
    table: &str,
) -> Option<(Reduction, RuleId)> {
    let zero_at = |stage: u8| Reduction::Range {
        floor: jlreq_unit::Em::ZERO,
        stage: ReductionStage::new(stage),
    };
    match (before, after, trailing, table) {
        // cl-05 x cl-05, both quarter-em terms (§D.2#1, §B.2#3).
        (5, 5, _, "table-3") => Some((zero_at(4), RuleId::D_2_NOTE_1)),
        (5, 5, _, "table-4") => Some((zero_at(2), RuleId::D_2_NOTE_1)),
        (5, 5, _, "table-5") => Some((Reduction::Rigid, RuleId::D_2_NOTE_1)),
        // cl-06 x cl-05, full stop's half em (rigid throughout) and the middle dot's
        // quarter em (§D.2#2, §B.2#5).
        (6, 5, false, _) | (6, 5, true, "table-5") => Some((Reduction::Rigid, RuleId::D_2_NOTE_2)),
        (6, 5, true, "table-3") => Some((zero_at(4), RuleId::D_2_NOTE_2)),
        (6, 5, true, "table-4") => Some((zero_at(2), RuleId::D_2_NOTE_2)),
        // cl-07 x cl-05, comma's half em and the middle dot's quarter em, each reducing on
        // its own schedule (§D.2#3, §B.2#5). The comma's own component is rigid in Table 4
        // (§D.2#3's Japanese text and its own priority sentence give cl-05 a Table 4 stage
        // and give cl-07 none there at all) for the same reason the middle dot's own
        // component is rigid in Table 5 (D.2#3 states only the comma's reduction there) —
        // two different notes-of-silence landing on the same `Rigid` answer, not one rule.
        (7, 5, false, "table-3") => Some((zero_at(5), RuleId::D_2_NOTE_3)),
        (7, 5, false, "table-4") | (7, 5, true, "table-5") => {
            Some((Reduction::Rigid, RuleId::D_2_NOTE_3))
        },
        (7, 5, true, "table-3") => Some((zero_at(4), RuleId::D_2_NOTE_3)),
        (7, 5, true, "table-4") => Some((zero_at(2), RuleId::D_2_NOTE_3)),
        (7, 5, false, "table-5") => Some((
            Reduction::Range {
                floor: jlreq_unit::Em::QUARTER,
                stage: ReductionStage::new(3),
            },
            RuleId::D_2_NOTE_3,
        )),
        _ => None,
    }
}

/// The reduction table `Question::REDUCTION_TABLE` selects, both as the generated cells and
/// as the name [`special_reduction`] matches on.
fn reduction_table(policy: Policy) -> (&'static [RawRangedCell], &'static str) {
    let name = policy.get(Question::REDUCTION_TABLE).name();
    match name {
        "table-4" => (crate::generated::table4::CELLS, name),
        "table-5" => (crate::generated::table5::CELLS, name),
        _ => (crate::generated::table3::CELLS, "table-3"),
    }
}

/// The cell of a ranged table (Tables 3 through 6) at one coordinate, if the transcription
/// holds one.
fn ranged_cell(cells: &'static [RawRangedCell], before: u8, after: u8) -> Option<RawRangedCell> {
    cells
        .iter()
        .copied()
        .find(|cell| cell.before == before && cell.after == after)
}

/// A captured ranged cell's `Reduction`.
fn cell_reduction(cell: RawRangedCell) -> Reduction {
    match cell.limit {
        None => Reduction::Rigid,
        Some(floor) if cell.two_valued => Reduction::Discrete {
            floor,
            stage: ReductionStage::new(cell.stage),
        },
        Some(floor) => Reduction::Range {
            floor,
            stage: ReductionStage::new(cell.stage),
        },
    }
}

/// A captured ranged cell's `Expansion`.
///
/// The cell's own `rule` is read separately, by [`expansion_rule_of`]: this function states
/// only the amount, because a caller of `cell_expansion` alone (there is exactly one,
/// [`expansion_of`]) already has the same cell in hand to read the citation off, and
/// threading it through a second time here would be one fact carried by two return paths.
fn cell_expansion(cell: RawRangedCell) -> Expansion {
    if cell.residual {
        return Expansion::Residual;
    }
    match cell.limit {
        None => Expansion::None,
        Some(ceiling) => Expansion::Range {
            ceiling,
            stage: ExpansionStage::new(cell.stage),
        },
    }
}

/// §B.2#2 and §B.2#6: the preferred half em at the line end after a closing bracket
/// (cl-02), a full stop (cl-06) or a comma (cl-07), or the JIS X 4051 alternative each
/// note states. The two notes are not the same shape: §B.2#2's alternative withdraws
/// cl-02's space outright, while §B.2#6's leaves the full stop at its preferred half em
/// and withdraws only the comma's — so this reads `Question::LINE_END_FULL_STOP_COMMA` for
/// the second pair rather than reusing `Question::LINE_END_PUNCTUATION`'s binary answer
/// for a class its own statement never names (`docs/design/api-spine.md`'s own two Question
/// docs). `None` when the boundary is not one of the three this pair of notes governs.
fn line_end_punctuation_override(before: u8, after: u8, policy: Policy) -> Option<bool> {
    if after != raw::LINE_EDGE {
        return None;
    }
    if before == Class::ClosingBracket.number() {
        return Some(policy.get(Question::LINE_END_PUNCTUATION).name() == "solid");
    }
    if before == Class::Comma.number() {
        return Some(policy.get(Question::LINE_END_FULL_STOP_COMMA).name() == "jis");
    }
    if before == Class::FullStop.number() {
        // §B.2#6's JIS X 4051 alternative keeps the full stop at its preferred half em —
        // the note withdraws only the comma's space, never the full stop's — so this is
        // always "not withdrawn", stated explicitly rather than left to fall through as
        // `None` (which reads as "this note does not govern cl-06" and is exactly the
        // silent-omission shape this function existed to fix for cl-06 and cl-07 alike).
        return Some(false);
    }
    None
}

/// §B.2#7's own withdrawal-qualifying classes for `Question::RUBY_OVERHANG_KANA`'s "jis"
/// and "none" answers, or `None` for "kana" (the preferred reading) and "any", which
/// withdraw nothing.
///
/// `jis`'s own stated text in `spec/derived/questions.tsv` names katakana (cl-16) alone;
/// the prolonged sound mark (cl-10) and small kana (cl-11) are this project's published
/// reading of "kana" more broadly. `none`'s own stated text additionally names hiragana
/// (cl-15) explicitly ("NOT to allow ruby text to be extended over any character from
/// hiragana (cl-15), katakana (cl-16) and ideographic characters (cl-19)"), so "none"
/// withdraws over one more class than "jis" does — the two are not the same set, and
/// widening "jis" to match "none" would grant JIS X 4051 a withdrawal its own statement
/// never claims. cl-19 is not included below because no Table 1 cell currently grants a
/// hang permission with cl-19 as the qualifying neighbor (`crate::generated`'s module
/// doc), so there is nothing here for it to withdraw yet.
fn ruby_overhang_withdrawal_classes(policy: Policy) -> Option<ClassSet> {
    let jis_kana = ClassSet::of(Class::Katakana)
        .with(Class::ProlongedSoundMark)
        .with(Class::SmallKana);
    match policy.get(Question::RUBY_OVERHANG_KANA).name() {
        "jis" => Some(jis_kana),
        "none" => Some(jis_kana.with(Class::Hiragana)),
        _ => None,
    }
}

/// §B.2#7: whether the JIS X 4051 or "none" reading in force withdraws the ruby overhang
/// Table 1 captured for this boundary's qualifying neighbor. `false` when the question does
/// not touch this boundary at all.
fn ruby_overhang_kana_withdrawn(neighbor: Option<Class>, hang: RawHang, policy: Policy) -> bool {
    if hang == RawHang::None {
        return false;
    }
    let Some(withdrawal) = ruby_overhang_withdrawal_classes(policy) else {
        return false;
    };
    neighbor.is_some_and(|class| withdrawal.contains(class))
}

/// Whether two adjacent items sit in the same construct run of `kind`, and if so the note
/// that delegates their placement (§B.2#9, #10, #11).
fn delegation_of(before: Option<Construct>, after: Option<Construct>) -> Option<Delegation> {
    let (before, after) = (before?, after?);
    if before != after {
        return None;
    }
    // Every other construct kind — including `Ornamented`, whose §B.2#9 delegates to
    // §3.7.1, which no crate places yet — falls to the wildcard on the same terms:
    // nothing here delegates a placement procedure for it, whether because no note names
    // one or because the note's target is not yet reachable.
    let rule = match before.kind() {
        ConstructKind::NonJukugoRuby => RuleId::B_2_NOTE_10,
        ConstructKind::JukugoRuby => RuleId::B_2_NOTE_11,
        _ => return None,
    };
    Some(Delegation { rule })
}

/// Everything about one boundary, in one call.
///
/// JLReq: §B, §C, §D, §E
#[must_use]
pub fn boundary(a: Adjacency<'_>, policy: Policy) -> Boundary {
    let (Some(before_position), Some(after_position)) = (a.before_position(), a.after_position())
    else {
        return empty_boundary();
    };
    let before_raw = before_position.raw();
    let after_raw = after_position.raw();

    let Some(cell) = spacing_cell(before_raw, after_raw) else {
        return empty_boundary();
    };

    let placement = if cell.prohibited {
        Answer::new(
            Placement::Forbidden { rule: cell.rule },
            Provenance::of(cell.rule, Standing::Normative),
        )
    } else {
        Answer::new(
            Placement::Permitted,
            Provenance::of(cell.rule, Standing::Normative),
        )
    };

    let breakable = evaluate_breakable(before_raw, after_raw, cell.rule, policy, a);

    let spaces = if cell.prohibited {
        [None, None]
    } else {
        spaces_of(
            before_raw,
            after_raw,
            cell,
            policy,
            a.before_role(),
            a.after_role(),
            a.direction(),
        )
    };

    // Read once per boundary rather than once per Table 1 term (ADR-0021 amends ADR-0014 on
    // this exact point): Table 6 has one cell per class pair, so it is a fact about the
    // *coordinate*, not about either referent's own contribution. Read unconditionally, not
    // gated on `cell.prohibited` the way `spaces_of` is — `xtask attest`'s own
    // `prohibition-agrees-across-tables` invariant already requires Table 6 to read `×`
    // wherever Table 1 does (with the same B.2#13/D.2#4 exemptions that invariant states),
    // and a `×` coordinate's Table 6 cell has no captured amount for `expansion_of` to read
    // regardless (`cell_expansion`'s own doc), so the two never disagree in practice; gating
    // this on `cell.prohibited` would only duplicate a fact the generated data already
    // states, not add one.
    //
    // Two things `spaces_of` withdraws a *term* for, decided rather than assumed, because
    // each is a real question this function has to answer and not a hypothetical:
    //
    // `line_end_punctuation_override`'s withdrawal (§B.2#2, §B.2#6) never reaches this
    // question at all: it only ever fires when `after == raw::LINE_EDGE`, and Table 6 names
    // no line-edge coordinate in the first place (§E.1's own words: "there are no cells
    // involving line head or line end" — `crate::generated::table6` carries no row whose
    // `before` or `after` is the line-edge sentinel). `expansion_of` reads `after_raw`
    // unchanged, so it answers `Expansion::None` at every line-end boundary structurally,
    // with no special case needed to make that true.
    //
    // `vertical_decimal_solid`'s withdrawal (§3.1.3) is not structurally excluded the same
    // way, and is a real interaction this function decides: cl-05 (katakana middle dot)
    // against cl-19 (ideograph) or cl-24 (grouped numeral), declared `Role::DecimalPoint` in
    // vertical writing, withdraws that boundary's own sole Table 1 term (`spaces_of`'s own
    // `continue`) while Table 6 still states a real opportunity there (`residual`, at both
    // coordinates — `crate::generated::table6`'s own `(5, 19)` and `(5, 24)` cells). The
    // reading here is that the withdrawal does not follow the term off the boundary: §3.1.3
    // withdraws a *space*, a stated amount at a stated priority; §E's own cell asks whether
    // the *position* may be opened up, which a withdrawn space leaves at zero rather than
    // answering "not a place" — the same distinction §B.2#17's own entry in
    // `docs/conformance-deferrals.toml` draws between a preferred reading that is genuinely
    // zero and a coordinate the evaluator has nothing to say about. Concretely, this reading
    // is what makes [`crate::ladder::Site::new`]'s own `base = 0` choice for a term-free
    // site the right one rather than an arbitrary default: a withdrawn term leaves nothing
    // *placed* to expand from, and nothing placed is exactly what a solid boundary's own
    // realized amount already is everywhere else this evaluator answers `Expansion::Range`
    // with no term beside it (cl-19 against cl-19, among others). The alternative reading —
    // that §3.1.3 also closes the position to expansion — has no textual support: the
    // section's own two list items name a space to omit, not a place to remove from §E's
    // table, and nothing in Appendix E or its own notes qualifies this coordinate at all.
    //
    // Two coordinates narrow Table 6's own captured ceiling further still, by the citing
    // note's own stated condition rather than by anything a class-pair-keyed cell can encode
    // alone: §E.2 note 10 at cl-27 against cl-13 (the same quantity-symbol-role-or-European-
    // numeral exception §C.2 note 11 already states over the identical ordered pair) and §E.2
    // note 4 at cl-08 against cl-08 (this project's own reading of "kind",
    // `docs/decisions/inseparable-character-kind.md`). `expansion_of` checks
    // `note_governed_expansion` first for exactly those two coordinates, mirroring
    // `evaluate_breakable`'s own `note_governed_refusal` read above for Table 2, before ever
    // falling through to the unconditioned cell read every other coordinate answers.
    let expansion = expansion_of(before_raw, after_raw, a);

    // The citation for the amount just read, independent of whether that amount was a real
    // ceiling or the note's own denial of one (`expansion_rule_of`'s own doc). Read alongside
    // `expansion` rather than folded into it: `Expansion` is a kind, not a record (ADR-0010),
    // and `Expansion::None` carrying a citation would make one variant of a closed enum
    // structurally different from its siblings for no reason a caller matching on the enum
    // could see. `Boundary::expansion_rule`'s own doc states why the `Option` here is the
    // whole point rather than an implementation detail: `None` is "no row", `Some` is "a row
    // spoke here", and the two are indistinguishable through `expansion` alone.
    let expansion_rule = expansion_rule_of(before_raw, after_raw, a);

    // §B.2#7 qualifies a neighbor on *either* side of the boundary: the note's own examples
    // put the ruby complex on both sides of the class in Table 1's rows and columns. Which
    // classes qualify is itself policy-dependent (`ruby_overhang_withdrawal_classes`'s own
    // doc) — "none" qualifies hiragana where "jis" does not — so this reads the same
    // withdrawal set `ruby_overhang_of` will check downstream, rather than a second,
    // independently-drifting copy of it.
    let qualifying_neighbor = ruby_overhang_withdrawal_classes(policy).and_then(|withdrawal| {
        [before_position.class(), after_position.class()]
            .into_iter()
            .flatten()
            .find(|class| withdrawal.contains(*class))
    });
    let ruby_overhang = ruby_overhang_of(cell, qualifying_neighbor, policy);

    let delegation = delegation_of(a.before_construct(), a.after_construct());

    Boundary::new(
        spaces,
        expansion,
        expansion_rule,
        breakable,
        placement,
        ruby_overhang,
        delegation,
    )
}

/// The boundary this evaluator states nothing about: cl-17 or cl-18 on either side, or a
/// coordinate the capture does not hold. Absence of a table cell is a fact ("no table
/// constrains this"), not a guessed answer, so every field here is the total-absence value
/// rather than a value this evaluator invented.
fn empty_boundary() -> Boundary {
    let rule = RuleId::SPACING_BETWEEN_CHARACTERS;
    Boundary::new(
        [None, None],
        Expansion::None,
        None,
        Answer::new(Breakable::Yes, Provenance::of(rule, Standing::Unstated)),
        Answer::new(
            Placement::Permitted,
            Provenance::of(rule, Standing::Unstated),
        ),
        RubyOverhang::None,
        None,
    )
}

/// Table 6's own expansion opportunity at one coordinate — one cell per class pair, read
/// once per boundary rather than once per Table 1 term now that ADR-0021 amends ADR-0014 to
/// move this fact off [`ConditionalSpace`] and onto [`crate::Boundary`] itself.
///
/// The defect this replaces: `spaces_of`'s own loop used to read this table only while
/// iterating `cell.terms`, so a coordinate with zero terms — a solid Table 1 cell, `blank`
/// in `spec/captured/table1.en.tsv` — never reached the lookup at all, regardless of what
/// Table 6 itself stated there. cl-19 against cl-19 (kanji beside kanji) is the coordinate
/// that makes the consequence visible: Table 1's own cell is `blank` (no conditional space
/// of either referent's), but Table 6's is `0-1/4 stage 3` — a real, quarter-em-ceiling
/// opportunity — so §3.8.4's own expansion procedure was structurally unreachable on
/// ordinary Japanese running text before this function existed. Reading the coordinate
/// directly, independent of `cell.terms`, is what makes it reachable.
///
/// That §E's own preamble presupposes Table 1's amount as the *unadjusted* starting point
/// ("The default unadjusted space between two adjacent characters of given character
/// classes shall be determined according to §B") is not evidence the opportunity needs a
/// term to attach to — it is evidence the floor that opportunity expands *from* is whatever
/// §B already states there, zero included: `spec/captured/table6.en.tsv`'s own capture
/// header records exactly this reconciliation for the `0-1/4` token ("solid by default,
/// expandable to a quarter-em ceiling"), and §3.8.4 step (c) itself names no starting
/// amount at all, only the *places* that may be opened up to a quarter em — Table 6 is the
/// complete table of those places, per §3.1.11's own second note.
///
/// Two coordinates narrow that unconditioned read further, by the citing note's own stated
/// condition rather than by anything a class-pair-keyed cell can encode alone —
/// [`note_governed_expansion`] checks both, `(8, 8)` and `(27, 13)`, before this function's
/// own generic lookup ever runs, mirroring [`note_governed_refusal`]'s identical shape for
/// Table 2 at the same two coordinates.
///
/// JLReq: §E, §E.1, §3.8.4
fn expansion_of(before: u8, after: u8, a: Adjacency<'_>) -> Expansion {
    if let Some((suppressed, _rule)) = note_governed_expansion(before, after, a) {
        return suppressed;
    }
    ranged_cell(crate::generated::table6::CELLS, before, after)
        .map_or(Expansion::None, cell_expansion)
}

/// Which rule states [`expansion_of`]'s answer at the identical coordinate — `Some` when a
/// row of Table 6 names this class pair, `None` when it does not.
///
/// Reads the same two sources [`expansion_of`] does, in the same order, for the reason
/// [`crate::Boundary::expansion_rule`]'s own doc states: the citation is the coordinate's
/// own fact, independent of whether what the row says is an opportunity or the note's own
/// denial of one, so `Some` here does not promise `expansion_of` answered anything but
/// [`Expansion::None`].
///
/// [`note_governed_expansion`]'s own two branches carry their citation as a literal
/// [`RuleId`] rather than re-reading `ranged_cell(...).map(|cell| cell.rule)` a second time —
/// checked directly against `crate::generated::table6::CELLS`, both `(8, 8)` and `(27, 13)`
/// already carry that identical literal as their own row's `rule` (`E_2_NOTE_4` and
/// `E_2_NOTE_10` respectively), so the two readings cannot disagree by construction: the note
/// that states the coordinate's row is the same note whose condition decides whether that
/// row's own opportunity is realized or withdrawn. Written as two independent literals
/// instead of one shared lookup so that a future revision of the captured table — one that
/// moved either coordinate's own citation to a different note — would fail this function's
/// own doc rather than silently start citing the *new* row's rule for an *old* note's
/// condition; `RuleId` is `Copy` and comparing two small integers costs nothing this crate
/// needs to avoid paying for that safety.
///
/// JLReq: §E, §E.1, §E.2, §3.8.4
fn expansion_rule_of(before: u8, after: u8, a: Adjacency<'_>) -> Option<RuleId> {
    if let Some((_expansion, rule)) = note_governed_expansion(before, after, a) {
        return Some(rule);
    }
    ranged_cell(crate::generated::table6::CELLS, before, after).map(|cell| cell.rule)
}

/// The cell of Table 1 at one coordinate, if the transcription holds one.
fn spacing_cell(before: u8, after: u8) -> Option<RawSpacingCell> {
    crate::generated::table1::CELLS
        .iter()
        .copied()
        .find(|cell| cell.before == before && cell.after == after)
}

/// The cell of Table 2 at one coordinate, if the transcription holds one. Table 2 carries
/// no line-edge axis, so a coordinate naming one never matches (§C.1).
fn break_cell(before: u8, after: u8) -> Option<raw::RawBreakCell> {
    crate::generated::table2::CELLS
        .iter()
        .copied()
        .find(|cell| cell.before == before && cell.after == after)
}

/// §C.2 note 5's five ordered adjacencies of cl-08's inseparable characters: EM DASH after
/// EM DASH, HORIZONTAL ELLIPSIS after HORIZONTAL ELLIPSIS, TWO DOT LEADER after TWO DOT
/// LEADER, and the two vertical-kana-repeat crossings the note names by their own two code
/// points (`crate::generated::table2` carries this note's own citation at `(8, 8)`, with an
/// empty `levels` bitmask, which is what routes the coordinate here instead of to the
/// kinsoku-level read below). The note's own closing sentence — "When the combination... is
/// different... the two characters are separable" — states a closed enumeration of exactly
/// these five pairs, not a general "identical mark" rule: cl-08 also lists the vertical kana
/// repeat mark lower half (〵, U+3035) on its own, and the note never pairs it with itself,
/// so `before == after` alone would over-reach it.
///
/// JLReq: §C.2#5
fn inseparable_member_pair(before: Option<Member>, after: Option<Member>) -> bool {
    let (Some(before), Some(after)) = (before, after) else {
        return false;
    };
    let em_dash = Member::single('\u{2014}');
    let horizontal_ellipsis = Member::single('\u{2026}');
    let two_dot_leader = Member::single('\u{2025}');
    let kana_repeat_upper = Member::single('\u{3033}');
    let kana_repeat_voiced_upper = Member::single('\u{3034}');
    let kana_repeat_lower = Member::single('\u{3035}');
    [
        (em_dash, em_dash),
        (horizontal_ellipsis, horizontal_ellipsis),
        (two_dot_leader, two_dot_leader),
        (kana_repeat_upper, kana_repeat_lower),
        (kana_repeat_voiced_upper, kana_repeat_lower),
    ]
    .contains(&(before, after))
}

/// §E.2 note 4's own reading of what makes two adjacent inseparable characters (cl-08) "of
/// different kinds" (別の種類の文字, `docs/decisions/inseparable-character-kind.md`): a
/// symmetric classification of the *character*, not an ordered enumeration of *pairs* the way
/// [`inseparable_member_pair`] reads §C.2 note 5's own five inseparable crossings. Four kinds
/// among cl-08's six members — the em dash (U+2014) alone, the horizontal ellipsis (U+2026)
/// alone, the two dot leader (U+2025) alone, and the vertical kana repeat mark's three code
/// points together (U+3033, U+3034, U+3035) — read as one kind rather than three, per that
/// file's own argument from §C.3's addendum (whose Very loose level names exactly the pairs
/// §C.2 note 5 forbids "Inseparable characters (cl-08) of the same kind", 同一の種類の分離禁止
/// 文字 — the identical word 種類 this note itself uses) and from §A.8's own Remarks column
/// ("U+3035 follows this" on both U+3033 and U+3034, naming the lower half as the shared
/// partner of two upper-half variants rather than a fourth independent mark). That file's own
/// "Why" also names the one pair this reading settles by the *shape* of a total, symmetric
/// partition rather than by a sentence of its own — U+3033 against U+3034, which JLReq's text
/// never addresses directly and real text never constructs — rather than hiding the inference.
///
/// `false` when either side resolves no member at all, the same total absence
/// [`inseparable_member_pair`] answers for the identical reason — and, unlike that function,
/// symmetric in `before` and `after`, because a kind is a property of one character, not a
/// fact about an ordered crossing.
///
/// JLReq: §E.2#4
fn cl_08_same_kind(before: Option<Member>, after: Option<Member>) -> bool {
    let (Some(before), Some(after)) = (before, after) else {
        return false;
    };
    if before == after {
        return true;
    }
    let kunojiten = [
        Member::single('\u{3033}'),
        Member::single('\u{3034}'),
        Member::single('\u{3035}'),
    ];
    kunojiten.contains(&before) && kunojiten.contains(&after)
}

/// §C.2 note 10: whether a break is permitted between a preceding grouped numeral (cl-24)
/// and a trailing Western character (cl-27), which `Question::GROUPED_NUMERAL_BEFORE_WESTERN`
/// answers directly rather than through a kinsoku strictness level — the generated cell at
/// `(24, 27)` carries this note's own citation with an empty `levels` bitmask for exactly
/// that reason. `Policy::with`'s own exclusion (§C.3's Very strict level excludes every §C.2
/// alternate rule, `crates/jlreq-spec/src/policy.rs`'s own test) means a `Policy` answering
/// `breakable` here and `very-strict` at `Question::KINSOKU_LEVEL` cannot be built at all, so
/// reading the question alone, with no separate strictness guard, already agrees with §C.3
/// at every level a caller can actually construct.
///
/// JLReq: §C.2#10
fn grouped_numeral_breaks_before_western(policy: Policy) -> bool {
    policy.get(Question::GROUPED_NUMERAL_BEFORE_WESTERN).name() == "breakable"
}

/// This project's own reading of "a European numeral" in §C.2 note 11
/// (`docs/decisions/european-numeral-by-code-point.md`): one of the ten keys U+0030 through
/// U+0039 that §A.19, §A.24 and §A.27 all name (the same set `jlreq_class::classify`'s own
/// `western_rule` reads to place a cl-27 occurrence), taken from the occurrence's own cluster
/// rather than from a caller-declared role.
///
/// JLReq: §C.2#11
fn is_european_numeral(member: Option<Member>) -> bool {
    let Some(member) = member else {
        return false;
    };
    let mut code_points = member.code_points();
    matches!(
        (code_points.next(), code_points.next()),
        (Some(digit), None) if digit.is_ascii_digit()
    )
}

/// §C.2 note 11: whether the preceding Western character (cl-27) is "used as a symbol of a
/// quantity" — [`Role::QuantitySymbol`], the caller's own declared job, matching
/// [`Adjacency::before_role`] — "or a European numeral", which [`is_european_numeral`] reads
/// from the occurrence's own key instead, for the reason its own doc states.
///
/// Read a second time, verbatim, by [`note_governed_expansion`] for §E.2 note 10's own
/// exception at the identical `(27, 13)` coordinate: the two notes state the same exception in
/// the same six words ("used as a symbol of a quantity or a European numeral") over the same
/// ordered class pair, one about breaking and the other about expanding, so a second reading
/// of that phrase for the expansion question would be a divergence with no textual basis
/// rather than a second fact the specification states.
///
/// JLReq: §C.2#11, §E.2#10
fn quantity_or_numeral(before_role: Option<Role>, before_member: Option<Member>) -> bool {
    matches!(before_role, Some(Role::QuantitySymbol)) || is_european_numeral(before_member)
}

/// The three Table 2 cells whose `levels` bitmask is empty because a §C.2 note's own
/// condition decides the boundary directly, rather than a kinsoku strictness level: §C.2
/// note 5 at cl-08 against cl-08, §C.2 note 10 at cl-24 against cl-27, and §C.2 note 11 at
/// cl-27 against cl-13. `None` for every other coordinate, which is what leaves
/// [`evaluate_breakable`]'s own kinsoku-level bitmask read as the answer everywhere else —
/// including every other non-prohibited, all-zero `levels` cell (Table 2's own prohibited
/// cells are all `0b1111`, never `0b0000`, so no cell reaches the generic read by way of an
/// empty bitmask and a `prohibited: true` flag together), which that generic read still
/// answers `Breakable::Yes` at `Standing::Normative`: the cell is a real Table 2 entry naming
/// this coordinate, so an empty bitmask there means no kinsoku level ever prohibits it, not
/// that the coordinate is unstated. `Standing::Unstated` is [`evaluate_breakable`]'s own
/// answer for a coordinate Table 2 does not carry at all ([`break_cell`] returning `None`),
/// which is a different thing from a carried cell whose bitmask happens to be empty.
///
/// JLReq: §C.2#5, §C.2#10, §C.2#11
fn note_governed_refusal(before: u8, after: u8, a: Adjacency<'_>, policy: Policy) -> Option<bool> {
    match (before, after) {
        (8, 8) => Some(inseparable_member_pair(a.before_member(), a.after_member())),
        (24, 27) => Some(!grouped_numeral_breaks_before_western(policy)),
        (27, 13) => Some(quantity_or_numeral(a.before_role(), a.before_member())),
        _ => None,
    }
}

/// The two Table 6 cells whose captured quarter-em ceiling only applies under the citing
/// note's own stated condition, mirroring [`note_governed_refusal`]'s identical shape for
/// Table 2's own three conditioned breakability cells — here at exactly two of Table 6's own
/// coordinates, `(8, 8)` and `(27, 13)`, the same pair [`note_governed_refusal`] itself
/// conditions for the breakability question. Every other of Table 6's own coordinates answers
/// [`expansion_of`]'s generic, unconditioned read correctly; this function exists for exactly
/// these two and no others.
///
/// `(27, 13)` reuses [`quantity_or_numeral`] verbatim rather than re-adjudicating the phrase it
/// reads (that function's own doc states why). `(8, 8)` reads [`cl_08_same_kind`], §E.2 note
/// 4's own condition, rather than [`inseparable_member_pair`], §C.2 note 5's: the two notes
/// state different questions over the same class pair — note 5 asks whether *this specific
/// ordered pair* is one of its own five named, order-specific crossings, and note 4 asks
/// whether the two occurrences are "of a different kind" at all, a symmetric classification of
/// the characters themselves, independent of which one came first. Answering note 4 by reusing
/// note 5's own closed five-pair enumeration would be exactly the reflex
/// [`inseparable_member_pair`]'s own doc already warns off for a different pair (`before ==
/// after` alone over-reaching it there); reusing it here would both miss the reverse-order and
/// upper-against-upper kunojiten crossings note 5 never lists, and mistake note 5's own
/// inseparability list — which is not exhaustive over "same kind" in either direction — for an
/// answer to a question it was never written to answer.
///
/// `Some((Expansion::None, rule))` where the citing note's own condition withdraws the
/// opportunity; `None` everywhere else — including these same two coordinates once their own
/// condition does not hold — so the caller falls through to [`expansion_of`]'s (or
/// [`expansion_rule_of`]'s) own unconditioned Table 6 read, which already carries the correct
/// captured ceiling, and the correct citation, for that case.
///
/// The `rule` half of the pair answers a question the withdrawal alone does not: which note
/// is the honest citation once the opportunity is denied. It is the note that states the
/// condition being checked — `RuleId::E_2_NOTE_4` at `(8, 8)`, `RuleId::E_2_NOTE_10` at `(27,
/// 13)` — never Table 6's own bare §E citation, because a *withdrawn* opportunity at these
/// two coordinates is still a fact §E.2 note 4 or note 10 states (each note's own sentence
/// states both halves of its condition, the opportunity and its absence, in one breath), not
/// a coordinate the table is silent about. [`expansion_rule_of`]'s own doc records that this
/// literal choice happens to agree with `crate::generated::table6::CELLS`' own row at both
/// coordinates — checked directly, not assumed — so the two readings never disagree in the
/// data this crate ships today.
///
/// JLReq: §E.2#4, §E.2#10
fn note_governed_expansion(before: u8, after: u8, a: Adjacency<'_>) -> Option<(Expansion, RuleId)> {
    match (before, after) {
        (8, 8) if cl_08_same_kind(a.before_member(), a.after_member()) => {
            Some((Expansion::None, RuleId::E_2_NOTE_4))
        },
        (27, 13) if quantity_or_numeral(a.before_role(), a.before_member()) => {
            Some((Expansion::None, RuleId::E_2_NOTE_10))
        },
        _ => None,
    }
}

/// §C.1: whether a line may end here — at `policy`'s strictness level for an ordinary Table 2
/// cell, or by §C.2 notes 5, 10 and 11's own condition ([`note_governed_refusal`]) for the
/// three cells whose `levels` bitmask states none.
fn evaluate_breakable(
    before: u8,
    after: u8,
    fallback_rule: RuleId,
    policy: Policy,
    a: Adjacency<'_>,
) -> Answer<Breakable> {
    let Some(cell) = break_cell(before, after) else {
        return Answer::new(
            Breakable::Yes,
            Provenance::of(fallback_rule, Standing::Unstated),
        );
    };
    if cell.prohibited {
        return Answer::new(
            Breakable::No { rule: cell.rule },
            Provenance::of(cell.rule, Standing::Normative),
        );
    }
    if let Some(refused) = note_governed_refusal(before, after, a, policy) {
        return Answer::new(
            if refused {
                Breakable::No { rule: cell.rule }
            } else {
                Breakable::Yes
            },
            Provenance::of(cell.rule, Standing::Normative),
        );
    }
    let level = kinsoku_level(policy);
    let bit = 1u8 << level.saturating_sub(1);
    if cell.levels & bit == bit {
        Answer::new(
            Breakable::No { rule: cell.rule },
            Provenance::of(cell.rule, Standing::Normative),
        )
    } else {
        Answer::new(
            Breakable::Yes,
            Provenance::of(cell.rule, Standing::Normative),
        )
    }
}

/// The strictness level (1 through 4) `Question::KINSOKU_LEVEL` selects.
fn kinsoku_level(policy: Policy) -> u8 {
    match policy.get(Question::KINSOKU_LEVEL).name() {
        "very-loose" => 1,
        "loose" => 2,
        "very-strict" => 4,
        _ => 3, // "strict", JLReq's own default.
    }
}

/// The conditional spaces of a non-prohibited Table 1 cell, with the D.2 stage split, the
/// B.2#2 policy override, the §3.1.3 vertical-writing override, §3.1.6's third-Note override
/// (`sentence_medial_dividing_mark_spaces`) and §3.1.5 pattern 2's own line-head override
/// (`line_head_opening_bracket_space`) applied — the latter two, both outside the per-term
/// loop below, for the identical reason: `cell.terms` is empty at every coordinate either one
/// actually governs, so a synthesis written inside the loop would never run where it matters.
/// Carries no [`Expansion`] (ADR-0021 amends ADR-0014 on this point):
/// `boundary`'s own `expansion_of` call reads Table 6 once per coordinate, independent of
/// how many terms this function builds — including zero, which is the coordinate
/// `expansion_of`'s own doc names.
fn spaces_of(
    before: u8,
    after: u8,
    cell: RawSpacingCell,
    policy: Policy,
    before_role: Option<Role>,
    after_role: Option<Role>,
    direction: Direction,
) -> [Option<ConditionalSpace>; 2] {
    if let Some(withdrawn) = line_end_punctuation_override(before, after, policy) {
        if withdrawn {
            return [None, None];
        }
    }

    let (reduction_cells, table_name) = reduction_table(policy);
    let mut built: [Option<ConditionalSpace>; 2] = [None, None];
    for (slot, term) in built.iter_mut().zip(cell.terms) {
        let referent = if term.trailing {
            Referent::Trailing
        } else {
            Referent::Preceding
        };
        let own_role = if term.trailing {
            after_role
        } else {
            before_role
        };
        if vertical_decimal_solid(own_role, direction) {
            // §3.1.3: the exceptional positioning of the ideographic comma and the
            // katakana middle dot in vertical writing withdraws this component entirely
            // rather than reducing it, which is why it short-circuits before the D.2
            // reduction split below rather than composing with it.
            continue;
        }
        let (reduction, reduction_rule) =
            match special_reduction(before, after, term.trailing, table_name) {
                Some((reduction, rule)) => (reduction, rule),
                None => match ranged_cell(reduction_cells, before, after) {
                    Some(ranged) => (cell_reduction(ranged), ranged.rule),
                    None => (Reduction::Rigid, cell.rule),
                },
            };
        let rule = if matches!(reduction, Reduction::Rigid) {
            cell.rule
        } else {
            reduction_rule
        };
        *slot = Some(ConditionalSpace::new(
            term.amount,
            referent,
            reduction,
            rule,
        ));
    }

    // Outside the per-term loop, deliberately: `cell.terms` is empty at every coordinate
    // §3.1.6's third Note actually governs (`sentence_medial_dividing_mark_spaces`'s own
    // doc), so a synthesis written inside the loop above would never run where it matters —
    // the identical structural defect ADR-0021 fixed for Table 6's own expansion opportunity
    // (`expansion_of`'s own doc narrates it). Only fills a slot the loop above left empty,
    // which every slot already is whenever this function fires at all.
    for (slot, synthesized) in built.iter_mut().zip(sentence_medial_dividing_mark_spaces(
        before,
        after,
        cell.terms.is_empty(),
        before_role,
        after_role,
        policy,
    )) {
        if slot.is_none() {
            *slot = synthesized;
        }
    }

    // Outside the per-term loop for the identical reason as the synthesis immediately above:
    // `cell.terms` is empty at `(0, 1)`, the only coordinate this function can ever fire at
    // (`line_head_opening_bracket_space`'s own guard), so a version written inside the loop
    // would never run. Unlike the sentence-medial synthesis, which can fill either or both
    // slots, this one never produces more than its single `Referent::Trailing` component —
    // a line-head boundary has no preceding item to own a second one — so `find` rather than
    // `zip` is enough to place it in whichever slot the loop above left empty (which, at this
    // coordinate, is always both).
    if let Some(synthesized) = line_head_opening_bracket_space(before, after, policy) {
        if let Some(slot) = built.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(synthesized);
        }
    }

    built
}

/// §3.1.3: in vertical writing, the space usually added after an ideographic comma (、)
/// used as a decimal separator, and the spaces before and after a katakana middle dot (・)
/// used as a decimal point, are omitted. Both are §A.24's role vocabulary
/// (`Role::DigitGroupSeparator`, `Role::DecimalPoint`) rather than a class alone, because
/// the section's own two list items name the *role* the character plays, not cl-05 or
/// cl-07 as a whole.
///
/// This is the one item `docs/direction-sites.toml` names for rule `3.1.3`: every other
/// site in this crate that reads [`Direction`] threads it through a signature without
/// naming a variant (ADR-0011).
fn vertical_decimal_solid(role: Option<Role>, direction: Direction) -> bool {
    matches!(direction, Direction::Vertical)
        && matches!(role, Some(Role::DecimalPoint | Role::DigitGroupSeparator))
}

/// §3.1.6's third Note: whether `Question::SENTENCE_MEDIAL_DIVIDING_MARK`'s "quarter-em"
/// answer synthesizes a conditional space at one side of this boundary for a sentence-medial
/// dividing punctuation mark (cl-04, [`Role::SentenceMedial`]) — a space Table 1 itself
/// carries no term for, because the coordinates this function governs are exactly the ones
/// where Table 1 states nothing (`cell_is_empty`,
/// `docs/decisions/sentence-medial-dividing-mark.md`'s own point 2). "Solid", the answer
/// every published preset carries, needs no branch of its own here: it is the identical
/// silence Table 1 already holds at every coordinate this function can reach, so deleting
/// this function entirely would still answer "solid" correctly — only "quarter-em" is
/// observable, which is exactly what makes reading `policy` here, rather than assuming,
/// load-bearing.
///
/// Scoped by `docs/decisions/sentence-medial-dividing-mark.md`'s own six adjudications
/// rather than guessed:
///
/// - Fires only where `cell_is_empty` holds — never at one of Table 1's own ten cl-04
///   coordinates, six of which are a *neighbor*'s own stated requirement and four of which
///   already give the mark this identical quarter em in its own referent (that file's
///   points 2 and 3).
/// - Draws no exception for a trailing closing bracket at `(4, 2)`: the main body's own
///   sentence-final exception answers a different, unimplemented mechanism (the "one em" as
///   an inserted cl-14 character), and silence is not license to reuse it here (point 4).
/// - Declines outright when the far side of the boundary is [`raw::LINE_EDGE`]: the Note's
///   own two words, "before and after", presuppose a neighboring character on both sides,
///   which a line boundary is not (point 5).
///
/// The amount, when it fires, is [`Referent::Trailing`] — the mark's own em — at the
/// boundary *before* the mark (`after` names the mark), and [`Referent::Preceding`] — again
/// the mark's own em — at the boundary *after* it (`before` names the mark): the inverse of
/// what "before"/"after" suggests read as textual position rather than as ADR-0014's own
/// ownership convention (point 1). [`Reduction::Rigid`] and this Note's own [`RuleId`],
/// never Table 1's generic citation, because there is no captured reduction stage to read
/// for a coordinate Table 1 states nothing at (point 6).
///
/// JLReq: §3.1.6
fn sentence_medial_dividing_mark_spaces(
    before: u8,
    after: u8,
    cell_is_empty: bool,
    before_role: Option<Role>,
    after_role: Option<Role>,
    policy: Policy,
) -> [Option<ConditionalSpace>; 2] {
    if !cell_is_empty || before == raw::LINE_EDGE || after == raw::LINE_EDGE {
        return [None, None];
    }
    if policy.get(Question::SENTENCE_MEDIAL_DIVIDING_MARK).name() != "quarter-em" {
        return [None, None];
    }

    let amount = jlreq_unit::Em::QUARTER;
    let rule =
        RuleId::POSITIONING_OF_DIVIDING_PUNCTUATION_MARKS_QUESTION_MARK_AND_EXCLAMATION_MARK_AND_HYPHENS;
    let mark = Class::DividingPunctuation.number();

    let mut spaces: [Option<ConditionalSpace>; 2] = [None, None];
    let mut next = 0;
    if before == mark && matches!(before_role, Some(Role::SentenceMedial)) {
        // The boundary after the mark: the mark is this boundary's preceding item, so its
        // own em is `Referent::Preceding` (point 1). Filled first: `spaces_of`'s own
        // per-term loop reads a captured cell's `Preceding` term before its `Trailing` one
        // whenever a cell states both (Table 1's own cl-05-against-cl-05 row orders its two
        // terms `be` before `af`, exactly the order §B.2 note 3's own words state, "a
        // quarter em of the *preceding* middle dots and a quarter em of the *trailing*
        // middle dots"), so this override matches that house order rather than the order
        // its own two `if` conditions happen to be written in.
        spaces[next] = Some(ConditionalSpace::new(
            amount,
            Referent::Preceding,
            Reduction::Rigid,
            rule,
        ));
        next = next.saturating_add(1);
    }
    if after == mark && matches!(after_role, Some(Role::SentenceMedial)) {
        // The boundary before the mark: the mark is this boundary's trailing item, so its
        // own em is `Referent::Trailing` (point 1). A boundary between two sentence-medial
        // marks (`(4, 4)`) satisfies both conditions at once, filling the second slot —
        // ADR-0014's own bound of two per boundary, one per referent, exactly as §B.2 note
        // 3's own two-term sum already exercises elsewhere in this module.
        if let Some(slot) = spaces.get_mut(next) {
            *slot = Some(ConditionalSpace::new(
                amount,
                Referent::Trailing,
                Reduction::Rigid,
                rule,
            ));
        }
    }
    spaces
}

/// §3.1.5 Figure 71 pattern 2, identified by §B.2 note 17's own cross-reference as that
/// note's "conditional half em spacing" alternative: whether
/// `Question::LINE_HEAD_OPENING_BRACKET` synthesizes a half em before an opening bracket
/// (cl-01, [`Class::OpeningBracket`]) that starts a line — a term Table 1 itself carries none
/// of at this coordinate (`crate::generated::table1`'s own `(0, 1)` row, `terms: &[]`, citing
/// [`RuleId::B_2_NOTE_17`] by name rather than the generic legend citation an ordinary blank
/// cell carries).
///
/// **Which half of the pattern.** Figure 71's three layouts each answer two positions: the
/// paragraph's own first-line indent after a line feed (改行行頭) and the indent an ordinary
/// in-paragraph wrap gets (折返し行頭). Only the second is a line-head *space* this evaluator
/// can state at all — the first is `jlreq_line::Paragraph`'s own declared first-line indent,
/// not an inter-character spacing fact (`docs/decisions/line-head-opening-bracket.md`'s own
/// "The reading" states the boundary this function stops at; `jlreq_line`'s own "Slots"
/// section names the consequence, pattern 3 in particular). At the wrapped line head, pattern
/// ① is 天付き (no space), pattern ② is 二分アキ (a half em) and pattern ③ is again 天付き —
/// so only pattern 2 produces a real amount here, and every other answer, including no
/// override at all, correctly falls through to nothing.
///
/// **Why this guard requires the line edge while `sentence_medial_dividing_mark_spaces`'s own
/// guard excludes it.** That function above declines outright whenever `before ==
/// raw::LINE_EDGE || after == raw::LINE_EDGE`, because §3.1.6's Note governs a mark sitting
/// *between* two characters — a line edge has no second character on that side, so it cannot
/// be the coordinate the Note describes. This function requires the opposite, `before ==
/// raw::LINE_EDGE` outright, because §3.1.5 governs a bracket's own position *at* the line
/// head, a fact that is only meaningful *at* that edge and nowhere else a boundary can sit.
/// The two guards run in opposite directions because the two rules describe opposite kinds of
/// coordinate — an interior adjacency for one, the line edge itself for the other — not
/// because either function picked its polarity arbitrarily.
///
/// **Why this is §B.2 note 17's own alternative and not a different half em.** The note's own
/// English states the preferred reading is zero and "[a]n alternative way is not to remove a
/// conditional half em spacing accompanying the characters," with a parenthetical naming this
/// exact section by title — "see § 3.1.5 Positioning of Opening Brackets at Line Head
/// including methods of positioning of opening brackets at the beginning of paragraphs"
/// (`spec/derived/notes.tsv`'s own `B.2#17` row). §3.1.5's own Figure 71 Note gives the
/// mechanism the appendix note's prose only gestures at: pattern ② "assume[s] that opening
/// brackets should be always accompanied by the preceding half em spacing as if they were
/// full-width," a half em at the wrapped line head under pattern 2 and nothing under the
/// other two — the identical half-em-or-zero choice B.2#17 states in the appendix's own
/// vocabulary. One note names the other by section title, so this identification needs no
/// inference of a shared subject, only the two notes' own words compared —
/// `docs/decisions/line-head-opening-bracket.md` records the comparison in full.
///
/// Three things this function's own shape had to adjudicate rather than read off the
/// document verbatim, argued at length in `docs/decisions/line-head-opening-bracket.md`
/// and only summarized here:
///
/// 1. **Referent.** [`Referent::Trailing`] — the bracket's own em — because the bracket is
///    this boundary's only possible neighbor (`before` is always [`raw::LINE_EDGE`] here, so
///    there is no preceding item to own anything), and that sole neighbor sits at `after`.
///    Unlike `sentence_medial_dividing_mark_spaces`'s own point 1, whose owner is inverted
///    from the naive "before"/"after" reading because that Note's mark sits at two different
///    boundaries in two different roles, this coordinate needs no inversion: ADR-0014's
///    "which neighbor's frame" convention and the plain textual position agree at once, and
///    §3.1.5's own Note independently names the bracket as the owner ("accompanied by the
///    preceding half em spacing as if they were full-width").
/// 2. **Reduction.** [`Reduction::Rigid`], stated directly rather than read through
///    `ranged_cell` the way the per-term loop above reads Tables 3 through 5. Checked
///    directly against the generated data: `table3.rs`, `table4.rs` and `table5.rs` each
///    carry a `(0, 1)` row, `limit: None` (which `cell_reduction` maps to `Reduction::Rigid`
///    regardless), citing [`RuleId::LEGEND_OF_TABLES_3_4_AND_5`] — but that citation is
///    §D.1's own generic legend statement, the one 833 of Table 3's, and 834 of Table 4's and
///    Table 5's, 841 rows each carry, because all three tables are total over the entire
///    29-by-29 grid rather than sparse over the coordinates Appendix D actually singles out.
///    `special_reduction`'s own three governed coordinates, `(5, 5)`, `(6, 5)` and `(7, 5)`,
///    are the only cells where a D.2 note actually assigns a term its own reduction
///    schedule; `(0, 1)` is not among them, and Table 1's own cell there carries no term in
///    the first place for any reduction table to be describing the reduction *of*. Routing
///    this synthesis through `ranged_cell` would attach a real `RuleId` to a fact nobody
///    stated. `docs/decisions/sentence-medial-dividing-mark.md`'s own point 6 reaches the
///    identical `Reduction::Rigid` answer for its own synthesis, but this function's own
///    decision record states the ground precisely rather than importing that file's
///    "Appendix D states no reduction schedule for a coordinate Table 1 itself states
///    nothing for" verbatim: checked directly, a captured row exists at `(19, 4)` too
///    (`ranged_cell(table3::CELLS, 19, 4)` answers `Some`, `limit: None`,
///    `LEGEND_OF_TABLES_3_4_AND_5`) — the total-matrix fact above, not a genuine absence — so
///    the precedent's own conclusion is right and its own stated reason is not quite what the
///    data holds.
///
///    Whether JLReq's own word "conditional" in "a conditional half em spacing" signals
///    Appendix-D reducibility specifically does not settle this either way: §D.2 note 5
///    calls the middle dot's own quarter em — which Table 3 genuinely does reduce —
///    "conditional" in the identical construction ("the preceding and trailing conditional
///    quarter em space accompanying middle dots"), and ADR-0014's own Context section quotes
///    "the conditional half em space accompanying the preceding comma" as Appendix B's
///    ordinary, blanket vocabulary for any table term, reducible or not. The word names the
///    category — an Appendix B amount realized only under stated conditions — not a promise
///    about Appendix D, so it answers nothing about this coordinate either way; what answers
///    it is the captured reduction tables' own content, read above.
/// 3. **Citation.** [`RuleId::POSITIONING_OF_OPENING_BRACKETS_AT_LINE_HEAD`] (§3.1.5), not
///    [`RuleId::B_2_NOTE_17`]. `RuleId::B_2_NOTE_17` already has a live reader at this exact
///    coordinate that no change this round makes: `crate::generated::table1`'s own `(0, 1)`
///    cell cites it directly, so `boundary`'s own `placement` answer already carries it
///    through `Provenance::of(cell.rule, Standing::Normative)`, and `rules_fired`'s own
///    `rules[1]` reads it regardless of `Question::LINE_HEAD_OPENING_BRACKET`'s answer or of
///    whether this function fires at all. `RuleId::POSITIONING_OF_OPENING_BRACKETS_AT_LINE_HEAD`
///    has no reader anywhere in this workspace before this round — checked directly: its own
///    constant (`crates/jlreq-spec/src/generated/inventory.rs`) and its own row
///    (`spec/derived/rules.tsv`) are its only two occurrences, cited by no generated table
///    cell and called from no evaluator function. Citing `B_2_NOTE_17` a second time here
///    would give one of this coordinate's two paired addresses a second reader while leaving
///    the other silent forever; citing §3.1.5 instead gives both a reader at the one
///    coordinate `docs/design/api-spine.md`'s own frozen doc for this Question already pairs
///    them at (`"Spacing before cl-01 at the line head. JLReq: §B.2#17, §3.1.5"`) — completing
///    the pairing the spine already promised rather than adding to one half of it. This has
///    the same *shape* as `sentence_medial_dividing_mark_spaces`'s own point 6 (a synthesized
///    space citing its own governing §3.x rule rather than Table 1's), but not the same
///    textual ground: that function's own coordinates are Table 1 cells with no specific
///    citation to compete with (the generic legend citation,
///    `RuleId::SPACING_BETWEEN_CHARACTERS`), where this coordinate already carries a real,
///    specific citation, `B_2_NOTE_17`, that this function deliberately does not repeat — for
///    the zero-readers argument above, not for the sibling function's own "nothing else to
///    cite" reason.
///
/// JLReq: §3.1.5, §B.2#17
fn line_head_opening_bracket_space(
    before: u8,
    after: u8,
    policy: Policy,
) -> Option<ConditionalSpace> {
    if before != raw::LINE_EDGE || after != Class::OpeningBracket.number() {
        return None;
    }
    if policy.get(Question::LINE_HEAD_OPENING_BRACKET).name() != "pattern-2" {
        return None;
    }
    Some(ConditionalSpace::new(
        jlreq_unit::Em::HALF,
        Referent::Trailing,
        Reduction::Rigid,
        RuleId::POSITIONING_OF_OPENING_BRACKETS_AT_LINE_HEAD,
    ))
}

/// The ruby-overhang permission Table 1's `hang` token grants, narrowed by §B.2#7 when
/// `policy` reads it as the JIS or "none" alternative.
fn ruby_overhang_of(cell: RawSpacingCell, neighbor: Option<Class>, policy: Policy) -> RubyOverhang {
    if ruby_overhang_kana_withdrawn(neighbor, cell.hang, policy) {
        return RubyOverhang::None;
    }
    match cell.hang {
        RawHang::None => RubyOverhang::None,
        RawHang::OverSpace => {
            let limit = cell
                .terms
                .first()
                .map_or(jlreq_unit::Em::ZERO, |term| term.amount);
            RubyOverhang::OverSpace { limit }
        },
        RawHang::OverCharacter => RubyOverhang::OverCharacter {
            limit: jlreq_unit::Em::FULL,
        },
    }
}

/// Which rules fired at one boundary. Drives the exercised-coverage gate.
///
/// The 6-slot array below is sized to breakable, placement, the two spaces, the delegation
/// and — as of this round — [`crate::Boundary::expansion_rule`]. An earlier revision of this
/// function left the sixth slot out on the ground that `crates/jlreq-conform/src/run.rs`'s
/// own `check_boundary` had no comparison surface for a citation of Table 6's own row; that
/// ground is gone now that `check_expansion` reads `Boundary::expansion_rule` too
/// (`CaseExpansion::rule`, compared under the conditional-equality semantics that function's
/// own doc states), so withholding the slot here would make this function under-report a
/// citation the runner can now act on.
///
/// The running `index` is incremented after *every* write that consumes a slot, including
/// the delegation's — the one place an earlier revision of this function got this wrong. A
/// boundary that carries two conditional spaces and a delegation fills `rules[2]`, `rules[3]`
/// and `rules[4]` in that order; had the delegation write left `index` at `4` instead of
/// advancing it to `5`, appending the expansion citation at `rules[index]` immediately
/// afterward would have overwritten the delegation's own slot rather than taking the next
/// one — silently losing a citation this iterator is supposed to report, at exactly the
/// coordinates where a boundary has the most to say. `rules_fired_reports_two_spaces_a_
/// delegation_and_an_expansion_without_clobbering_any_of_them`, below, is the regression test
/// for that shape.
///
/// JLReq: §B, §C, §D, §E
pub fn rules_fired(a: Adjacency<'_>, policy: Policy) -> impl Iterator<Item = RuleId> {
    let result = boundary(a, policy);
    let mut rules = [None; 6];
    rules[0] = Some(
        result
            .breakable()
            .why()
            .rules()
            .next()
            .unwrap_or(RuleId::SPACING_BETWEEN_CHARACTERS),
    );
    rules[1] = Some(
        result
            .placement()
            .why()
            .rules()
            .next()
            .unwrap_or(RuleId::SPACING_BETWEEN_CHARACTERS),
    );
    let mut index = 2;
    for space in result.spaces() {
        if index >= rules.len() {
            break;
        }
        rules[index] = Some(space.rule());
        index = index.saturating_add(1);
    }
    if let Some(delegation) = result.delegation() {
        if index < rules.len() {
            rules[index] = Some(delegation.rule);
            index = index.saturating_add(1);
        }
    }
    if let Some(expansion_rule) = result.expansion_rule() {
        if index < rules.len() {
            rules[index] = Some(expansion_rule);
        }
    }
    rules.into_iter().flatten()
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU16;

    use jlreq_class::{Class, Text};
    use jlreq_spec::{Choice, Policy, Question, RuleId, Standing};
    use jlreq_unit::{
        Advance, ByteOffset, Construct, ConstructKind, Direction, Em, Frame, InlineExtent, Item,
        ItemIndex, Role, RunId, Runs, Scale, ScaleId,
    };

    use super::{
        Adjacency, boundary, ruby_overhang_kana_withdrawn, ruby_overhang_withdrawal_classes,
        rules_fired,
    };
    use crate::boundary::Breakable;
    use crate::space::{Expansion, ExpansionStage, Reduction, Referent};

    /// One item at byte offset `byte`, half-width — every fixture code point below is one
    /// of §3.1.2's five classes whose advance is half-width.
    fn item(byte: u32, frame: Frame) -> Item {
        let advance = InlineExtent::new(1000).expect("a positive advance");
        Item::new(ByteOffset::new(byte), advance, ScaleId::BASE).with_frame(frame)
    }

    /// The same, with a declared role.
    fn item_with_role(byte: u32, frame: Frame, role: Role) -> Item {
        item(byte, frame).with_role(role)
    }

    fn scale() -> Scale {
        let em = Advance::new(1000).expect("a positive advance");
        Scale::square(em).expect("a positive scale")
    }

    /// The one item `docs/direction-sites.toml` allowlists for rule `3.1.3` in test code:
    /// every fixture below needs *some* direction, and most need horizontal specifically
    /// only to have one at all, which is what `vertical` being the parameter rather than
    /// two separate named constants keeps to one allowlisted item instead of two.
    fn direction_of(vertical: bool) -> Direction {
        if vertical {
            Direction::Vertical
        } else {
            Direction::Horizontal
        }
    }

    fn choice(question: Question, name: &str) -> Choice {
        question
            .permits()
            .iter()
            .find(|choice| choice.name() == name)
            .copied()
            .unwrap_or_else(|| panic!("`{name}` is not one of {question:?}'s answers"))
    }

    #[test]
    fn a_plain_table1_lookup_reads_the_captured_amount() {
        // '(' (cl-01) followed by ':' (cl-05): the transcription's own row reads `1/4 af`
        // with no note, so the citing rule is §B's own legend statement.
        let items = [item(0, Frame::HalfEm), item(1, Frame::HalfEm)];
        let scales = [scale()];
        let text = Text::new("(:", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);

        let mut spaces = result.spaces();
        let only = spaces.next().expect("one term");
        assert!(spaces.next().is_none(), "exactly one term");
        assert_eq!(only.amount(), Em::QUARTER);
        assert_eq!(only.referent(), Referent::Trailing);
        assert!(result.is_permitted());
    }

    #[test]
    fn table_6_expansion_is_reachable_at_a_solid_table_1_cell() {
        // The behavioral proof this round's own defect fix rests on: cl-19 against cl-19
        // (kanji beside kanji, 亜亜) is `blank` in Table 1 (`spec/captured/table1.en.tsv`'s
        // own row) — no conditional space of either referent's — but `0-1/4 stage 3` in
        // Table 6 (`spec/captured/table6.en.tsv`'s own row at the identical coordinate), a
        // real quarter-em ceiling at the third priority stage. §E's own preamble is the
        // authority for reading that ceiling from a floor of zero rather than refusing to
        // answer: "the default unadjusted space between two adjacent characters of given
        // character classes shall be determined according to §B", and §B's own answer here
        // is solid.
        //
        // Before this round, `spaces_of`'s per-term loop only ever read Table 6 while
        // iterating `cell.terms`, so a coordinate with zero terms — exactly this one — never
        // reached the lookup regardless of what Table 6 stated, and this assertion would
        // have failed against `Expansion::None`. `boundary`'s own `expansion_of` call reads
        // the coordinate directly now, which is what this test is here to prove holds for
        // real generated data and not only for the hand-built fixtures `Boundary`'s own
        // `a_boundary_can_answer_a_real_expansion_with_no_conditional_space_at_all` checks.
        let items = [item(0, Frame::FullEm), item(3, Frame::FullEm)];
        let scales = [scale()];
        let text = Text::new("\u{4E9C}\u{4E9C}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);

        assert_eq!(
            result.spaces().count(),
            0,
            "Table 1's own cl-19-against-cl-19 cell is blank"
        );
        assert_eq!(
            result.expansion(),
            Expansion::Range {
                ceiling: Em::QUARTER,
                stage: ExpansionStage::new(3),
            },
            "Table 6's own cl-19-against-cl-19 cell, `0-1/4 stage 3`"
        );
    }

    #[test]
    fn b_2_note_3_reads_as_two_terms_one_per_referent() {
        // ':' followed by ':' (cl-05 x cl-05): §B.2 note 3's sum, "1/4 be + 1/4 af" — two
        // conditional spaces from one cell, exactly ADR-0014's claim.
        let items = [item(0, Frame::HalfEm), item(1, Frame::HalfEm)];
        let scales = [scale()];
        let text = Text::new("::", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);

        let mut preceding = false;
        let mut trailing = false;
        let mut count = 0;
        for space in result.spaces() {
            count += 1;
            assert_eq!(space.amount(), Em::QUARTER);
            // Under Policy::JLREQ's own default (reduction Table 3) both quarter ems are
            // reducible, so `rule()` cites the more specific §D.2#1 rather than the sum's
            // own §B.2#3 — see `spaces_of`'s citation rule.
            assert_eq!(space.rule(), RuleId::D_2_NOTE_1);
            match space.referent() {
                Referent::Preceding => preceding = true,
                Referent::Trailing => trailing = true,
            }
        }
        assert_eq!(count, 2, "one boundary, at most two components (ADR-0014)");
        assert!(preceding && trailing, "one per referent");
    }

    #[test]
    fn d_2_note_1_gives_the_two_quarter_ems_the_reduction_table_says() {
        // §D.2#1: Table 3 reduces both quarter ems to solid at stage 4, Table 4 at stage
        // 2, and Table 5 does not reduce this boundary at all — three readings of one
        // cl-05 x cl-05 cell, selected by `Question::REDUCTION_TABLE` alone.
        let items = [item(0, Frame::HalfEm), item(1, Frame::HalfEm)];
        let scales = [scale()];
        let text = Text::new("::", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");

        for space in boundary(a, Policy::JLREQ).spaces() {
            assert_eq!(
                space.reduction(),
                Reduction::Range {
                    floor: Em::ZERO,
                    stage: crate::space::ReductionStage::new(4),
                },
                "Policy::JLREQ selects reduction Table 3"
            );
        }

        let table4 = Policy::JLREQ
            .with(choice(Question::REDUCTION_TABLE, "table-4"))
            .expect("table-4 is a permitted answer");
        for space in boundary(a, table4).spaces() {
            assert_eq!(
                space.reduction(),
                Reduction::Range {
                    floor: Em::ZERO,
                    stage: crate::space::ReductionStage::new(2),
                }
            );
        }

        let table5 = Policy::JLREQ
            .with(choice(Question::REDUCTION_TABLE, "table-5"))
            .expect("table-5 is a permitted answer");
        for space in boundary(a, table5).spaces() {
            assert_eq!(
                space.reduction(),
                Reduction::Rigid,
                "book practice does not reduce this cell"
            );
        }
    }

    #[test]
    fn ruby_overhang_kana_withdrawal_reads_each_answers_own_stated_classes() {
        // §B.2#7's "jis" answer states katakana (cl-16) by name; its "none" answer states
        // hiragana (cl-15), katakana (cl-16) and ideographic (cl-19) by name
        // (`spec/derived/questions.tsv`'s own statement text for each). The two sets are
        // not the same, and "none" must qualify hiragana where "jis" does not.
        //
        // Every Table 1 cell that grants a ruby-overhang permission pairs cl-10, cl-11,
        // cl-15 or cl-16 with cl-22 or cl-23 (`InNonJukugoRubyComplex`/`InJukugoRubyComplex`
        // — verified directly against `crate::generated::table1::CELLS`), and those two
        // classes are §3.9's ruby-base-run reclassification, which `jlreq_class::resolve`
        // does not yet compute from a `Runs` overlay (ruby placement is M4-a, not this
        // milestone). That makes every ruby-overhang boundary structurally unreachable
        // through the public `boundary` this milestone builds, for any policy answer, not
        // only this fix's own case — so this tests the private decision directly, the same
        // way the bug this fixes was originally found, rather than asserting coverage
        // `boundary` cannot yet exercise.
        let jis = Policy::JLREQ
            .with(choice(Question::RUBY_OVERHANG_KANA, "jis"))
            .expect("jis is a permitted answer");
        let none = Policy::JLREQ
            .with(choice(Question::RUBY_OVERHANG_KANA, "none"))
            .expect("none is a permitted answer");

        assert_eq!(
            ruby_overhang_withdrawal_classes(jis),
            Some(
                jlreq_class::ClassSet::of(Class::Katakana)
                    .with(Class::ProlongedSoundMark)
                    .with(Class::SmallKana)
            )
        );
        assert_eq!(
            ruby_overhang_withdrawal_classes(none),
            Some(
                jlreq_class::ClassSet::of(Class::Katakana)
                    .with(Class::ProlongedSoundMark)
                    .with(Class::SmallKana)
                    .with(Class::Hiragana)
            ),
            "\"none\" additionally withdraws over hiragana, which \"jis\" does not"
        );
        assert_eq!(ruby_overhang_withdrawal_classes(Policy::JLREQ), None);

        assert!(
            !ruby_overhang_kana_withdrawn(Some(Class::Hiragana), super::RawHang::OverSpace, jis),
            "\"jis\" never withdraws over hiragana"
        );
        assert!(
            ruby_overhang_kana_withdrawn(Some(Class::Hiragana), super::RawHang::OverSpace, none),
            "\"none\" withdraws over hiragana, per its own stated text"
        );
    }

    #[test]
    fn b_2_note_2_switches_the_line_end_amount_between_half_em_and_solid() {
        // cl-02 at the line end: JLReq's own preferred half em, or JIS X 4051's solid,
        // selected by `Question::LINE_END_PUNCTUATION` and nothing hardcoded.
        let items = [item(0, Frame::HalfEm)];
        let scales = [scale()];
        let text = Text::new(")", &items, &scales).expect("a well-formed text");
        let a = Adjacency::at_line_end(text, Runs::none(), ItemIndex::new(0), direction_of(false));

        let preferred = boundary(a, Policy::JLREQ);
        let mut spaces = preferred.spaces();
        let only = spaces.next().expect("the preferred half em");
        assert!(spaces.next().is_none());
        assert_eq!(only.amount(), Em::HALF);

        let solid = Policy::JLREQ
            .with(choice(Question::LINE_END_PUNCTUATION, "solid"))
            .expect("solid is a permitted answer");
        let withdrawn = boundary(a, solid);
        assert_eq!(
            withdrawn.spaces().count(),
            0,
            "the solid reading withdraws the space entirely"
        );
    }

    #[test]
    fn b_2_note_6_withdraws_only_the_comma_under_the_jis_reading() {
        // §B.2#6 is its own note, distinct from §B.2#2: JIS X 4051's alternative leaves
        // the full stop (cl-06, U+3002) at its preferred half em and withdraws only the
        // comma's (cl-07, U+3001), which `Question::LINE_END_FULL_STOP_COMMA` must be able
        // to state on its own rather than reusing `Question::LINE_END_PUNCTUATION`'s
        // binary answer, which only cl-02's own note ever names.
        let full_stop_items = [item(0, Frame::HalfEm)];
        let scales = [scale()];
        let full_stop_text =
            Text::new("\u{3002}", &full_stop_items, &scales).expect("a well-formed text");
        let full_stop = Adjacency::at_line_end(
            full_stop_text,
            Runs::none(),
            ItemIndex::new(0),
            direction_of(false),
        );

        let comma_items = [item(0, Frame::HalfEm)];
        let comma_text = Text::new("\u{3001}", &comma_items, &scales).expect("a well-formed text");
        let comma = Adjacency::at_line_end(
            comma_text,
            Runs::none(),
            ItemIndex::new(0),
            direction_of(false),
        );

        for a in [full_stop, comma] {
            let preferred = boundary(a, Policy::JLREQ);
            let mut spaces = preferred.spaces();
            let only = spaces.next().expect("the preferred half em");
            assert!(spaces.next().is_none());
            assert_eq!(only.amount(), Em::HALF);
        }

        let jis = Policy::JLREQ
            .with(choice(Question::LINE_END_FULL_STOP_COMMA, "jis"))
            .expect("jis is a permitted answer");

        let full_stop_under_jis = boundary(full_stop, jis);
        let mut spaces = full_stop_under_jis.spaces();
        let only = spaces
            .next()
            .expect("the full stop keeps its half em under JIS X 4051");
        assert!(spaces.next().is_none());
        assert_eq!(only.amount(), Em::HALF);

        let comma_under_jis = boundary(comma, jis);
        assert_eq!(
            comma_under_jis.spaces().count(),
            0,
            "the comma's space is withdrawn entirely under JIS X 4051, unlike the full stop's"
        );
    }

    #[test]
    fn breakable_reads_table_2_at_the_selected_strictness_level() {
        // ')' (cl-02) is one of §3.1.7's line-start-prohibited classes: Table 2's own
        // column for cl-02 is `not` (prohibited at every strictness level), so a break
        // before it is refused under every kinsoku level Policy::JLREQ can select.
        let items = [item(0, Frame::FullEm), item(1, Frame::HalfEm)];
        let scales = [scale()];
        let text = Text::new(".)", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert!(!result.is_breakable());
        assert!(matches!(result.breakable().value(), Breakable::No { .. }));
    }

    #[test]
    fn rules_fired_names_the_sum_notes_own_rule() {
        let items = [item(0, Frame::HalfEm), item(1, Frame::HalfEm)];
        let scales = [scale()];
        let text = Text::new("::", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        assert!(
            rules_fired(a, Policy::JLREQ).any(|rule| rule == RuleId::B_2_NOTE_3),
            "the middle-dot sum's own note is among the fired rules"
        );
    }

    #[test]
    fn section_3_1_3_withdraws_the_decimal_comma_space_only_in_vertical_writing() {
        // U+3001 IDEOGRAPHIC COMMA (、, cl-07, unambiguous — no Remarks qualification)
        // declared as `Role::DigitGroupSeparator`, before ':' (cl-05). §B.2#5 makes this
        // boundary two components — the comma's own half em and the middle dot's own
        // quarter em — and §3.1.3 withdraws only the comma's, because that component and
        // not the middle dot's is what the role qualifies. Horizontal writing keeps both;
        // vertical writing keeps only the middle dot's.
        let items = [
            item_with_role(0, Frame::HalfEm, Role::DigitGroupSeparator),
            item(3, Frame::HalfEm),
        ];
        let scales = [scale()];
        let text = Text::new("\u{3001}:", &items, &scales).expect("a well-formed text");

        let horizontal =
            Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
                .expect("a two-item text has a boundary at ordinal 0");
        let horizontal_result = boundary(horizontal, Policy::JLREQ);
        assert_eq!(
            horizontal_result.spaces().count(),
            2,
            "horizontal writing keeps both components (§B.2#5)"
        );

        let vertical =
            Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(true))
                .expect("a two-item text has a boundary at ordinal 0");
        let vertical_result = boundary(vertical, Policy::JLREQ);
        let mut spaces = vertical_result.spaces();
        let only = spaces
            .next()
            .expect("the middle dot's own component remains");
        assert!(spaces.next().is_none(), "exactly one component remains");
        assert_eq!(
            only.referent(),
            Referent::Trailing,
            "§3.1.3 withdraws the comma's component (be), not the middle dot's (af)"
        );
    }

    #[test]
    fn an_adjacency_naming_no_table_cell_carries_no_restriction() {
        // Reached in practice for cl-17/cl-18 (§3.7.4), exercised directly since building
        // a fixture whose classification actually lands on a math symbol needs a Unicode
        // code point outside the ASCII alphabet the rest of this module uses.
        let result = super::empty_boundary();
        assert!(result.is_breakable());
        assert!(result.is_permitted());
        assert_eq!(result.spaces().count(), 0);
        assert_eq!(result.placement().why().standing(), Standing::Unstated);
    }

    #[test]
    fn c_2_note_5_forbids_a_break_between_two_identical_em_dashes() {
        // §C.2 note 5's own worked example: "when two EM DASH appears consecutively, these
        // two characters are inseparable". U+2014 is the cl-08 member the note means, not
        // its look-alike U+2015 HORIZONTAL BAR, which no Appendix A table lists at all
        // (`crates/jlreq-conform/cases/A.8.json`'s own `A.8/horizontal-bar/*` cases record
        // that divergence directly, and the confusable pair is exactly why this test spells
        // the code point out rather than pasting the glyph — a probe written this way was
        // how this round's own defect was first confirmed, and how the confusable pair was
        // caught a second time while writing this very test).
        let items = [item(0, Frame::FullEm), item(3, Frame::FullEm)];
        let scales = [scale()];
        let text = Text::new("\u{2014}\u{2014}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert!(
            !result.is_breakable(),
            "two EM DASH in a row are inseparable"
        );
        assert_eq!(
            result.breakable().why().rules().next(),
            Some(RuleId::C_2_NOTE_5)
        );
    }

    #[test]
    fn c_2_note_5_permits_a_break_between_different_inseparable_characters() {
        // The note's own worked example, second half: "consecutive EM DASH and HORIZONTAL
        // ELLIPSIS are separable" — the permissive half that keeps the refusal above from
        // over-reaching to every cl-08 pair.
        let items = [item(0, Frame::FullEm), item(3, Frame::FullEm)];
        let scales = [scale()];
        let text = Text::new("\u{2014}\u{2026}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert!(
            result.is_breakable(),
            "EM DASH then HORIZONTAL ELLIPSIS is separable"
        );
        assert_eq!(
            result.breakable().why().rules().next(),
            Some(RuleId::C_2_NOTE_5)
        );
    }

    #[test]
    fn c_2_note_5_forbids_a_break_at_the_kunojiten_crossing() {
        // The fourth of the note's five ordered adjacencies: 〳 (upper half) then 〵 (lower
        // half) — two different members, so `before == after` alone would miss this pair.
        // Neither key needs a declared frame: §A.8 lists both nowhere else.
        let items = [item(0, Frame::Unstated), item(3, Frame::Unstated)];
        let scales = [scale()];
        let text = Text::new("\u{3033}\u{3035}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert!(
            !result.is_breakable(),
            "〳 then 〵 is one of the note's five named pairs"
        );
        assert_eq!(
            result.breakable().why().rules().next(),
            Some(RuleId::C_2_NOTE_5)
        );
    }

    #[test]
    fn c_2_note_10_reads_the_grouped_numeral_before_western_question() {
        // §C.2 note 10's two approaches, selected by `Question::GROUPED_NUMERAL_BEFORE_WESTERN`
        // alone: `Policy::JLREQ`'s own default is `breakable`.
        let items = [item(0, Frame::HalfEm), item(1, Frame::Proportional)];
        let scales = [scale()];
        let text = Text::new("1A", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");

        let preferred = boundary(a, Policy::JLREQ);
        assert!(
            preferred.is_breakable(),
            "Policy::JLREQ's own default is breakable"
        );
        assert_eq!(
            preferred.breakable().why().rules().next(),
            Some(RuleId::C_2_NOTE_10)
        );

        let unbreakable = Policy::JLREQ
            .with(choice(
                Question::GROUPED_NUMERAL_BEFORE_WESTERN,
                "unbreakable",
            ))
            .expect("unbreakable is a permitted answer");
        let other = boundary(a, unbreakable);
        assert!(
            !other.is_breakable(),
            "the note's other approach forbids the break"
        );
        assert_eq!(
            other.breakable().why().rules().next(),
            Some(RuleId::C_2_NOTE_10)
        );
    }

    #[test]
    fn c_2_note_11_forbids_a_break_after_a_declared_quantity_symbol() {
        // §C.2 note 11's first exception: a preceding Western character (cl-27) "used as a
        // symbol of a quantity", which `Role::QuantitySymbol` is the caller's own way to
        // state.
        let items = [
            item_with_role(0, Frame::Proportional, Role::QuantitySymbol),
            item(1, Frame::FullEm),
        ];
        let scales = [scale()];
        let text = Text::new("A\u{FF05}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert!(
            !result.is_breakable(),
            "a declared quantity symbol forbids the break"
        );
        assert_eq!(
            result.breakable().why().rules().next(),
            Some(RuleId::C_2_NOTE_11)
        );
    }

    #[test]
    fn c_2_note_11_forbids_a_break_after_a_european_numeral() {
        // §C.2 note 11's second exception, this project's own reading of it
        // (`docs/decisions/european-numeral-by-code-point.md`): a preceding cl-27 occurrence
        // whose own key is one of the ten European numerals, read from the code point with
        // no role declared at all.
        let items = [item(0, Frame::Proportional), item(1, Frame::FullEm)];
        let scales = [scale()];
        let text = Text::new("5\u{FF05}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert!(
            !result.is_breakable(),
            "a European numeral forbids the break with no role declared"
        );
        assert_eq!(
            result.breakable().why().rules().next(),
            Some(RuleId::C_2_NOTE_11)
        );
    }

    #[test]
    fn c_2_note_11_permits_a_break_after_an_ordinary_western_character() {
        // The note's own general rule, unqualified: "A line break opportunity generally
        // exists between preceding Western characters (cl-27) and trailing postfixed
        // abbreviations (cl-13)".
        let items = [item(0, Frame::Proportional), item(1, Frame::FullEm)];
        let scales = [scale()];
        let text = Text::new("A\u{FF05}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert!(
            result.is_breakable(),
            "an ordinary Western character permits the break"
        );
        assert_eq!(
            result.breakable().why().rules().next(),
            Some(RuleId::C_2_NOTE_11)
        );
    }

    #[test]
    fn e_2_note_10_expands_between_an_ordinary_western_character_and_a_postfixed_abbreviation() {
        // §E.2 note 10's own general rule, unqualified by either half of its exception: an
        // ordinary Western letter (cl-27) carrying no declared role and no numeral key, before
        // a postfixed abbreviation (cl-13) — the identical fixture
        // `c_2_note_11_permits_a_break_after_an_ordinary_western_character` uses for the
        // breakability sibling of this same note pair, since both questions share the same
        // unqualified condition. `crates/jlreq-spacing/src/generated/table6.rs`'s own `(27,
        // 13)` cell is `0-1/4 stage 3`, and nothing here suppresses it.
        let items = [item(0, Frame::Proportional), item(1, Frame::FullEm)];
        let scales = [scale()];
        let text = Text::new("A\u{FF05}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert_eq!(
            result.expansion(),
            Expansion::Range {
                ceiling: Em::QUARTER,
                stage: ExpansionStage::new(3),
            },
            "an ordinary Western character permits the expansion §E.2 note 10 states"
        );
    }

    #[test]
    fn e_2_note_10_suppresses_expansion_after_a_declared_quantity_symbol() {
        // §E.2 note 10's own exception, its first half: a preceding Western character (cl-27)
        // "used as a symbol of a quantity", read through the identical `quantity_or_numeral`
        // §C.2 note 11's own breakability question already reads — the same declared-role
        // fixture `c_2_note_11_forbids_a_break_after_a_declared_quantity_symbol` uses.
        let items = [
            item_with_role(0, Frame::Proportional, Role::QuantitySymbol),
            item(1, Frame::FullEm),
        ];
        let scales = [scale()];
        let text = Text::new("A\u{FF05}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert_eq!(
            result.expansion(),
            Expansion::None,
            "a declared quantity symbol withdraws the expansion opportunity"
        );
    }

    #[test]
    fn e_2_note_10_suppresses_expansion_after_a_european_numeral() {
        // §E.2 note 10's own exception, its second half: a preceding cl-27 occurrence whose
        // own key is one of the ten European numerals
        // (`docs/decisions/european-numeral-by-code-point.md`), with no role declared at all —
        // the same fixture `c_2_note_11_forbids_a_break_after_a_european_numeral` uses.
        let items = [item(0, Frame::Proportional), item(1, Frame::FullEm)];
        let scales = [scale()];
        let text = Text::new("5\u{FF05}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert_eq!(
            result.expansion(),
            Expansion::None,
            "a European numeral withdraws the expansion opportunity with no role declared"
        );
    }

    #[test]
    fn e_2_note_4_suppresses_expansion_between_two_identical_em_dashes() {
        // §E.2 note 4's own condition denies the opportunity for two occurrences of the same
        // kind: two consecutive EM DASH (U+2014) are certainly the same kind, being the
        // identical character — the same fixture
        // `c_2_note_5_forbids_a_break_between_two_identical_em_dashes` uses for the
        // breakability sibling.
        let items = [item(0, Frame::FullEm), item(3, Frame::FullEm)];
        let scales = [scale()];
        let text = Text::new("\u{2014}\u{2014}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert_eq!(
            result.expansion(),
            Expansion::None,
            "two EM DASH in a row are the same kind, so no expansion opportunity exists"
        );
    }

    #[test]
    fn e_2_note_4_expands_between_two_visibly_different_inseparable_characters() {
        // Two marks no reading of "kind" could conflate — EM DASH and HORIZONTAL ELLIPSIS,
        // the same fixture
        // `c_2_note_5_permits_a_break_between_different_inseparable_characters` uses.
        // `crates/jlreq-spacing/src/generated/table6.rs`'s own `(8, 8)` cell is `0-1/4 stage
        // 3`, and nothing here suppresses it.
        let items = [item(0, Frame::FullEm), item(3, Frame::FullEm)];
        let scales = [scale()];
        let text = Text::new("\u{2014}\u{2026}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert_eq!(
            result.expansion(),
            Expansion::Range {
                ceiling: Em::QUARTER,
                stage: ExpansionStage::new(3),
            },
            "EM DASH then HORIZONTAL ELLIPSIS are different kinds, so the opportunity stands"
        );
    }

    #[test]
    fn e_2_note_4_suppresses_expansion_at_the_kunojiten_crossing() {
        // 〳 (upper half) then 〵 (lower half): one of §C.2 note 5's own five named pairs, and
        // — per `docs/decisions/inseparable-character-kind.md` — the same reading's own
        // clearest textual anchor, §C.3's own "inseparable characters (cl-08) of the same
        // kind" naming exactly the pairs this note forbids. Same fixture
        // `c_2_note_5_forbids_a_break_at_the_kunojiten_crossing` uses.
        let items = [item(0, Frame::Unstated), item(3, Frame::Unstated)];
        let scales = [scale()];
        let text = Text::new("\u{3033}\u{3035}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(true))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert_eq!(
            result.expansion(),
            Expansion::None,
            "the kunojiten's upper and lower halves are one kind, per this project's own reading"
        );
    }

    #[test]
    fn e_2_note_4_suppresses_expansion_at_the_reverse_kunojiten_crossing() {
        // 〵 (lower half) then 〳 (upper half): the reverse of §C.2 note 5's own fourth named
        // pair, which that note never lists (its own enumeration is order-specific) but which
        // this reading's own symmetric partition still calls one kind — the test that
        // distinguishes `cl_08_same_kind` from a reflex reuse of
        // [`inseparable_member_pair`]'s own order-specific five pairs.
        let items = [item(0, Frame::Unstated), item(3, Frame::Unstated)];
        let scales = [scale()];
        let text = Text::new("\u{3035}\u{3033}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(true))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert_eq!(
            result.expansion(),
            Expansion::None,
            "a kind is a property of each character, not of the order they appear in"
        );
    }

    #[test]
    fn expansion_rule_reads_the_citing_notes_address_at_a_noted_cell() {
        // Em dash then horizontal ellipsis (cl-08 x cl-08, two different kinds) — the
        // identical fixture `e_2_note_4_expands_between_two_visibly_different_inseparable_
        // characters` uses. Table 6's own `(8, 8)` cell (`crates/jlreq-spacing/src/
        // generated/table6.rs`) cites `RuleId::E_2_NOTE_4` directly, and `boundary()` reads
        // a real, non-`None` opportunity there too — the ordinary case, where the citation
        // and the amount agree on what the row states.
        let items = [item(0, Frame::FullEm), item(3, Frame::FullEm)];
        let scales = [scale()];
        let text = Text::new("\u{2014}\u{2026}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert_eq!(result.expansion_rule(), Some(RuleId::E_2_NOTE_4));
    }

    #[test]
    fn expansion_rule_reads_the_generic_e_citation_at_an_unnoted_cell() {
        // Kanji beside kanji (cl-19 x cl-19) — the identical fixture `table_6_expansion_is_
        // reachable_at_a_solid_table_1_cell` uses. No §E.2 note is attached to this
        // coordinate, so `crate::generated::table6::CELLS`' own row cites §E's bare opening
        // sentence, `RuleId::OPPORTUNITIES_FOR_INTER_CHARACTER_SPACE_EXPANSION_DURING_LINE_
        // ADJUSTMENT`, the same address `E/dividing-punctuation-then-western`'s own case
        // publishes as `"rule": "E"`.
        let items = [item(0, Frame::FullEm), item(3, Frame::FullEm)];
        let scales = [scale()];
        let text = Text::new("\u{4E9C}\u{4E9C}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert_eq!(
            result.expansion_rule(),
            Some(RuleId::OPPORTUNITIES_FOR_INTER_CHARACTER_SPACE_EXPANSION_DURING_LINE_ADJUSTMENT)
        );
    }

    #[test]
    fn expansion_rule_still_names_the_governing_note_when_it_withdraws_the_opportunity() {
        // Two consecutive EM DASH (cl-08 x cl-08, the same character both sides) — the
        // identical fixture `e_2_note_4_suppresses_expansion_between_two_identical_em_
        // dashes` uses. §E.2 note 4's own condition withdraws the opportunity here
        // (`expansion()` answers `Expansion::None`), but the note is still the honest
        // citation for *why* there is none — this is the coordinate
        // `Boundary::expansion_rule`'s own doc names directly: `Some` does not promise a
        // real ceiling, only that a row (here, a note-governed one) spoke about this
        // coordinate. Distinguishing this from a coordinate no table carries at all is the
        // entire reason this accessor answers `Option<RuleId>` rather than folding the
        // citation into `Expansion` itself.
        let items = [item(0, Frame::FullEm), item(3, Frame::FullEm)];
        let scales = [scale()];
        let text = Text::new("\u{2014}\u{2014}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, Policy::JLREQ);
        assert_eq!(
            result.expansion(),
            Expansion::None,
            "two identical em dashes are certainly of one kind"
        );
        assert_eq!(
            result.expansion_rule(),
            Some(RuleId::E_2_NOTE_4),
            "the note that denies the opportunity is still the note that states this row, \
             not the bare absence a coordinate with no row at all would answer"
        );
    }

    #[test]
    fn expansion_rule_is_none_where_no_table_names_this_coordinate_at_all() {
        // `empty_boundary()` is what `boundary()` answers for cl-17/cl-18 and for any
        // coordinate outside the six matrices' own twenty-eight classes
        // (`an_adjacency_naming_no_table_cell_carries_no_restriction`, above, tests the
        // same function directly for the identical reason: a real fixture that classifies
        // to a math symbol needs a code point outside this module's own ASCII alphabet).
        // `None` here is not "the note denies it" — it is "no row exists to read a citation
        // from at all", the fact `expansion_rule()` exists to keep distinct from the
        // previous test's own `Some(RuleId::E_2_NOTE_4)`.
        let result = super::empty_boundary();
        assert_eq!(result.expansion(), Expansion::None);
        assert_eq!(result.expansion_rule(), None);
    }

    #[test]
    fn rules_fired_reports_two_spaces_a_delegation_and_an_expansion_without_clobbering_any_of_them()
    {
        // The off-by-one this test guards against: `rules_fired`'s own running `index` used
        // to stop advancing once the delegation was written, so appending the expansion
        // citation right after it would have landed on the delegation's own slot instead of
        // the next one — silently losing a citation at exactly the boundary with the most
        // to report. Two middle dots (cl-05 x cl-05) already carry two conditional spaces
        // (§B.2 note 3's own sum, the identical fixture
        // `b_2_note_3_reads_as_two_terms_one_per_referent` uses); overlaying both items as
        // one non-jukugo-ruby construct run adds the delegation §B.2 note 10 states for
        // that construct kind — `delegation_of` reads `Runs` alone, independent of either
        // item's own class. Table 6's own `(5, 5)` cell
        // (`crates/jlreq-spacing/src/generated/table6.rs`) states no ceiling (`limit:
        // None`), so `expansion()` answers `Expansion::None`, but the row still exists and
        // still cites §E's own opening sentence — exactly the shape the two tests above
        // this one already establish, now exercised through `rules_fired` rather than
        // `Boundary::expansion_rule` directly.
        let items = [item(0, Frame::HalfEm), item(1, Frame::HalfEm)];
        let scales = [scale()];
        let text = Text::new("::", &items, &scales).expect("a well-formed text");
        let run = RunId::new(NonZeroU16::new(1).expect("one is non-zero"));
        let construct = Construct::new(ConstructKind::NonJukugoRuby, run, None);
        let slots = [Some(construct), Some(construct)];
        let runs = Runs::new(&slots).expect("one contiguous run over both items");
        let a = Adjacency::between(text, runs, ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");

        let result = boundary(a, Policy::JLREQ);
        assert_eq!(result.spaces().count(), 2, "§B.2 note 3's own sum");
        assert!(
            result.delegation().is_some(),
            "both items share one non-jukugo-ruby run"
        );
        assert_eq!(
            result.expansion(),
            Expansion::None,
            "Table 6's own (5, 5) cell states no ceiling"
        );
        assert_eq!(
            result.expansion_rule(),
            Some(RuleId::OPPORTUNITIES_FOR_INTER_CHARACTER_SPACE_EXPANSION_DURING_LINE_ADJUSTMENT),
            "the row still exists even though it states no ceiling"
        );

        // No `Vec` collected here (this crate is `#![no_std]`): `rules_fired` is a pure
        // function of `a` and the policy, so each assertion below asks it fresh rather than
        // buffering one answer to inspect several times.
        assert_eq!(
            rules_fired(a, Policy::JLREQ).count(),
            6,
            "breakable, placement, two spaces, the delegation and the expansion citation, \
             each in its own slot"
        );
        assert_eq!(
            rules_fired(a, Policy::JLREQ)
                .filter(|&rule| rule == RuleId::D_2_NOTE_1)
                .count(),
            2,
            "both spaces' own reduction citation survives"
        );
        assert!(
            rules_fired(a, Policy::JLREQ).any(|rule| rule == RuleId::B_2_NOTE_10),
            "the delegation survives the expansion slot's own write, which is the bug this \
             test exists to catch"
        );
        assert!(
            rules_fired(a, Policy::JLREQ).any(|rule| {
                rule
                == RuleId::OPPORTUNITIES_FOR_INTER_CHARACTER_SPACE_EXPANSION_DURING_LINE_ADJUSTMENT
            }),
            "the expansion citation is present too, in its own slot rather than the \
             delegation's"
        );
    }

    /// U+2048 QUESTION EXCLAMATION MARK, `A.4.json`'s own
    /// `A.4/question-exclamation-mark/sentence-medial-role` code point, declared
    /// [`Frame::FullEm`] exactly as that case declares it, surrounded by an ordinary
    /// ideograph (cl-19, 亜) on each side — the "ordinary running text" coordinates §3.1.6's
    /// third Note is about, `(19, 4)` and `(4, 19)`, both `terms: &[]` in
    /// `spec/captured/table1.en.tsv`'s own row, not one of the ten coordinates Table 1
    /// already answers for cl-04. Shared by every §3.1.6 test below rather than
    /// hand-duplicated, so a wrong byte offset or a wrong `Frame` fails every one of them
    /// identically instead of only the one test that happens to assert a non-empty result.
    const SENTENCE_MEDIAL_TEXT: &str = "\u{4E9C}\u{2048}\u{4E9C}";

    /// The three items [`SENTENCE_MEDIAL_TEXT`] carries, with `role` declared on the middle
    /// one (the mark itself).
    fn sentence_medial_fixture(role: Role) -> [Item; 3] {
        [
            item(0, Frame::FullEm),
            item_with_role(3, Frame::FullEm, role),
            item(6, Frame::FullEm),
        ]
    }

    #[test]
    fn section_3_1_6_quarter_em_synthesizes_a_space_on_both_sides_of_an_ordinary_medial_mark() {
        // 亜⁈亜, under the `quarter-em` overlay: both boundaries around the mark get its own
        // quarter em, and the referent flips between them exactly as
        // `docs/decisions/sentence-medial-dividing-mark.md`'s own point 1 derives from
        // ADR-0014 — the mark is the *trailing* item at the boundary before it, and the
        // *preceding* item at the boundary after it.
        let items = sentence_medial_fixture(Role::SentenceMedial);
        let scales = [scale()];
        let text = Text::new(SENTENCE_MEDIAL_TEXT, &items, &scales).expect("a well-formed text");
        let quarter_em = Policy::JLREQ
            .with(choice(
                Question::SENTENCE_MEDIAL_DIVIDING_MARK,
                "quarter-em",
            ))
            .expect("quarter-em is a permitted answer");
        let rule = RuleId::POSITIONING_OF_DIVIDING_PUNCTUATION_MARKS_QUESTION_MARK_AND_EXCLAMATION_MARK_AND_HYPHENS;

        let before_mark =
            Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
                .expect("a three-item text has a boundary at ordinal 0");
        let before_result = boundary(before_mark, quarter_em);
        let mut spaces = before_result.spaces();
        let only = spaces.next().expect("the mark's own leading quarter em");
        assert!(spaces.next().is_none(), "exactly one component");
        assert_eq!(only.amount(), Em::QUARTER);
        assert_eq!(
            only.referent(),
            Referent::Trailing,
            "the mark is this boundary's trailing item, so its own em is `af`"
        );
        assert_eq!(only.reduction(), Reduction::Rigid);
        assert_eq!(only.rule(), rule);

        let after_mark =
            Adjacency::between(text, Runs::none(), ItemIndex::new(1), direction_of(false))
                .expect("a three-item text has a boundary at ordinal 1");
        let after_result = boundary(after_mark, quarter_em);
        let mut spaces = after_result.spaces();
        let only = spaces.next().expect("the mark's own trailing quarter em");
        assert!(spaces.next().is_none(), "exactly one component");
        assert_eq!(only.amount(), Em::QUARTER);
        assert_eq!(
            only.referent(),
            Referent::Preceding,
            "the mark is this boundary's preceding item, so its own em is `be`"
        );
        assert_eq!(only.reduction(), Reduction::Rigid);
        assert_eq!(only.rule(), rule);
    }

    #[test]
    fn section_3_1_6_a_boundary_between_two_medial_marks_fills_both_referents_in_order() {
        // ⁈⁈: `(4, 4)`, a boundary between two sentence-medial marks — the doc comment on
        // `sentence_medial_dividing_mark_spaces` claims both of its conditions hold at once
        // here, filling both slots; this is that claim made into an assertion rather than
        // left as prose. The fill order matches the house convention `spaces_of`'s own
        // per-term loop already uses for a captured two-term cell — `Referent::Preceding`
        // before `Referent::Trailing` (Table 1's own cl-05-against-cl-05 row orders its two
        // terms `be` before `af`, and §B.2 note 3's own words read "preceding" before
        // "trailing") — which this override matches deliberately rather than by the
        // incidental order its own two `if` conditions are written in.
        let items = [
            item_with_role(0, Frame::FullEm, Role::SentenceMedial),
            item_with_role(3, Frame::FullEm, Role::SentenceMedial),
        ];
        let scales = [scale()];
        let text = Text::new("\u{2048}\u{2048}", &items, &scales).expect("a well-formed text");
        let quarter_em = Policy::JLREQ
            .with(choice(
                Question::SENTENCE_MEDIAL_DIVIDING_MARK,
                "quarter-em",
            ))
            .expect("quarter-em is a permitted answer");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, quarter_em);
        let rule = RuleId::POSITIONING_OF_DIVIDING_PUNCTUATION_MARKS_QUESTION_MARK_AND_EXCLAMATION_MARK_AND_HYPHENS;

        let mut spaces = result.spaces();
        let first = spaces
            .next()
            .expect("the first mark's own trailing-side em (`be`)");
        let second = spaces
            .next()
            .expect("the second mark's own leading-side em (`af`)");
        assert!(spaces.next().is_none(), "exactly two components");
        assert_eq!(first.referent(), Referent::Preceding);
        assert_eq!(second.referent(), Referent::Trailing);
        for space in [first, second] {
            assert_eq!(space.amount(), Em::QUARTER);
            assert_eq!(space.reduction(), Reduction::Rigid);
            assert_eq!(space.rule(), rule);
        }
    }

    #[test]
    fn section_3_1_6_the_default_solid_answer_leaves_ordinary_running_text_untouched() {
        // The identical fixture, under `Policy::JLREQ`'s own default (`solid`, which every
        // published preset answers): both boundaries stay at zero spaces, which is the same
        // silence Table 1 already holds at `(19, 4)` and `(4, 19)` with no override at all —
        // this assertion would hold even if `sentence_medial_dividing_mark_spaces` were
        // deleted outright, and it exists to pin that fact down rather than to exercise the
        // override (the override's own behavior is `quarter_em`'s job, above).
        let items = sentence_medial_fixture(Role::SentenceMedial);
        let scales = [scale()];
        let text = Text::new(SENTENCE_MEDIAL_TEXT, &items, &scales).expect("a well-formed text");

        for ordinal in [0, 1] {
            let a = Adjacency::between(
                text,
                Runs::none(),
                ItemIndex::new(ordinal),
                direction_of(false),
            )
            .expect("a three-item text has a boundary at this ordinal");
            let result = boundary(a, Policy::JLREQ);
            assert_eq!(
                result.spaces().count(),
                0,
                "solid is Table 1's own silence, not a withdrawal this override performs"
            );
        }
    }

    #[test]
    fn section_3_1_6_does_not_touch_a_coordinate_table_1_already_answers_for_the_mark() {
        // ⁈( : a sentence-medial mark immediately followed by an opening bracket, `(4, 1)`
        // — one of Table 1's own ten cl-04 coordinates, and one whose term is owned by the
        // *bracket* (`Referent::Trailing`), not the mark (`spec/captured/table1.en.tsv`'s
        // own trailing half em there). `docs/decisions/sentence-medial-dividing-mark.md`'s
        // own points 2 and 3: the override never reaches a coordinate `cell.terms` already
        // answers, even under the `quarter-em` overlay, so the bracket's own half em
        // survives untouched and nothing is added beside it — the live alternative reading
        // that file's own "Why" names would fire here instead, which is exactly what this
        // test would catch.
        let items = [
            item_with_role(0, Frame::FullEm, Role::SentenceMedial),
            item(3, Frame::HalfEm),
        ];
        let scales = [scale()];
        let text = Text::new("\u{2048}(", &items, &scales).expect("a well-formed text");
        let quarter_em = Policy::JLREQ
            .with(choice(
                Question::SENTENCE_MEDIAL_DIVIDING_MARK,
                "quarter-em",
            ))
            .expect("quarter-em is a permitted answer");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, quarter_em);

        let mut spaces = result.spaces();
        let only = spaces
            .next()
            .expect("the bracket's own half em, from Table 1 alone");
        assert!(
            spaces.next().is_none(),
            "the override adds nothing beside it"
        );
        assert_eq!(only.amount(), Em::HALF);
        assert_eq!(
            only.referent(),
            Referent::Trailing,
            "owned by the bracket, not the mark"
        );
        assert_ne!(
            only.rule(),
            RuleId::POSITIONING_OF_DIVIDING_PUNCTUATION_MARKS_QUESTION_MARK_AND_EXCLAMATION_MARK_AND_HYPHENS,
            "cited by Table 1 or its selected reduction table (`spaces_of`'s own citation \
             rule), never by §3.1.6's Note, since the override never touched this term"
        );
    }

    #[test]
    fn section_3_1_6_fires_at_a_trailing_closing_bracket_with_no_exception_carved_out() {
        // ⁈) : a sentence-medial mark immediately followed by a closing bracket, `(4, 2)` —
        // unlike `(4, 1)`'s opening bracket above, this coordinate's own term is empty
        // (`table1.rs`'s own cell), so it is exactly the kind of coordinate the override
        // reaches. `docs/decisions/sentence-medial-dividing-mark.md`'s own point 4 argues
        // the override fires here uniformly, drawing no exception for the trailing bracket
        // the way the main body's own sentence-final "one em" rule does (a different,
        // unimplemented mechanism) — this test makes that argument's own sharpest edge an
        // assertion rather than leaving it resting on prose alone.
        let items = [
            item_with_role(0, Frame::FullEm, Role::SentenceMedial),
            item(3, Frame::HalfEm),
        ];
        let scales = [scale()];
        let text = Text::new("\u{2048})", &items, &scales).expect("a well-formed text");
        let quarter_em = Policy::JLREQ
            .with(choice(
                Question::SENTENCE_MEDIAL_DIVIDING_MARK,
                "quarter-em",
            ))
            .expect("quarter-em is a permitted answer");
        let a = Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let result = boundary(a, quarter_em);
        let rule = RuleId::POSITIONING_OF_DIVIDING_PUNCTUATION_MARKS_QUESTION_MARK_AND_EXCLAMATION_MARK_AND_HYPHENS;

        let mut spaces = result.spaces();
        let only = spaces
            .next()
            .expect("the mark's own quarter em, with no bracket exception carved out");
        assert!(spaces.next().is_none(), "exactly one component");
        assert_eq!(only.amount(), Em::QUARTER);
        assert_eq!(
            only.referent(),
            Referent::Preceding,
            "the mark is this boundary's preceding item, so its own em is `be`"
        );
        assert_eq!(only.reduction(), Reduction::Rigid);
        assert_eq!(only.rule(), rule);
    }

    #[test]
    fn section_3_1_6_override_does_not_fire_for_a_sentence_final_mark() {
        // The identical 亜⁈亜 shape, but the mark declares `Role::SentenceTerminator`
        // rather than `Role::SentenceMedial` — this Note's own subject, and the class the
        // override reads the role for, is the medial job specifically (`A.4.json`'s own
        // `sentence-medial-role` case is a `classify` case and never exercises spacing at
        // all, so this is the one place that role distinction is checked against an
        // amount). A sentence-final mark at the identical empty-term coordinate gets
        // nothing from this override even under the `quarter-em` overlay.
        let items = sentence_medial_fixture(Role::SentenceTerminator);
        let scales = [scale()];
        let text = Text::new(SENTENCE_MEDIAL_TEXT, &items, &scales).expect("a well-formed text");
        let quarter_em = Policy::JLREQ
            .with(choice(
                Question::SENTENCE_MEDIAL_DIVIDING_MARK,
                "quarter-em",
            ))
            .expect("quarter-em is a permitted answer");

        for ordinal in [0, 1] {
            let a = Adjacency::between(
                text,
                Runs::none(),
                ItemIndex::new(ordinal),
                direction_of(false),
            )
            .expect("a three-item text has a boundary at this ordinal");
            let result = boundary(a, quarter_em);
            assert_eq!(
                result.spaces().count(),
                0,
                "a sentence-final mark is not this Note's own subject"
            );
        }
    }

    #[test]
    fn section_3_1_6_declines_a_line_edge_even_under_the_quarter_em_overlay() {
        // 亜⁈ with the mark ending the line: `(4, 0)`, a real, non-prohibited, empty-term
        // coordinate — unlike `(0, 4)`, which Table 1 already prohibits outright at the
        // line head (§3.1.7), so that coordinate is excluded structurally and never reaches
        // this override regardless. `(4, 0)` has no such exclusion, so
        // `docs/decisions/sentence-medial-dividing-mark.md`'s own point 5 declines it
        // explicitly: the Note's own "before and after" presupposes a neighboring
        // character on both sides, which a line boundary is not.
        let items = [
            item(0, Frame::FullEm),
            item_with_role(3, Frame::FullEm, Role::SentenceMedial),
        ];
        let scales = [scale()];
        let text = Text::new("\u{4E9C}\u{2048}", &items, &scales).expect("a well-formed text");
        let quarter_em = Policy::JLREQ
            .with(choice(
                Question::SENTENCE_MEDIAL_DIVIDING_MARK,
                "quarter-em",
            ))
            .expect("quarter-em is a permitted answer");
        let a = Adjacency::at_line_end(text, Runs::none(), ItemIndex::new(1), direction_of(false));
        let result = boundary(a, quarter_em);
        assert_eq!(
            result.spaces().count(),
            0,
            "a line edge is declined explicitly, even though `(4, 0)` is otherwise empty-term"
        );
    }

    #[test]
    fn line_head_opening_bracket_default_presets_answer_no_space() {
        // §3.1.5's own two zero-space patterns: `Policy::JLREQ` (and `Policy::JIS_READING`,
        // `Policy::MAGAZINE`, `Policy::NEWSPAPER`) answer pattern 1
        // (`spec/derived/questions.tsv`'s own row for `spacing.line_head_opening_bracket`),
        // and `Policy::BOOK` answers pattern 3
        // (`the_book_preset_diverges_from_jlreq_at_exactly_its_documented_overrides`,
        // `crates/jlreq-spec/src/policy.rs`). Patterns 1 and 3 both answer 天付き — no space —
        // at the wrapped line head (Figure 71's own Note), so no built-in preset can regress
        // from this round's own change, which only pattern 2 (reachable through an explicit
        // override, never a preset) can observe.
        let items = [item(0, Frame::HalfEm)];
        let scales = [scale()];
        let text = Text::new("(", &items, &scales).expect("a well-formed text");
        let a = Adjacency::at_line_head(text, Runs::none(), ItemIndex::new(0), direction_of(false));

        for policy in [Policy::JLREQ, Policy::BOOK] {
            let result = boundary(a, policy);
            assert_eq!(
                result.spaces().count(),
                0,
                "no built-in preset answers pattern 2"
            );
        }
    }

    #[test]
    fn line_head_opening_bracket_pattern_2_synthesizes_the_wrapped_half_em() {
        // §3.1.5 Figure 71 pattern 2 / §B.2#17's own alternative: a half em, owned by the
        // bracket (`Referent::Trailing`, the bracket being this boundary's only neighbor),
        // not reducible (Appendix D states no schedule for it — `line_head_opening_bracket_
        // space`'s own doc, point 2), citing §3.1.5's own rule rather than §B.2#17's (point
        // 3 of the same doc).
        let items = [item(0, Frame::HalfEm)];
        let scales = [scale()];
        let text = Text::new("(", &items, &scales).expect("a well-formed text");
        let a = Adjacency::at_line_head(text, Runs::none(), ItemIndex::new(0), direction_of(false));
        let pattern_2 = Policy::JLREQ
            .with(choice(Question::LINE_HEAD_OPENING_BRACKET, "pattern-2"))
            .expect("pattern-2 is a permitted answer");

        let result = boundary(a, pattern_2);
        let mut spaces = result.spaces();
        let only = spaces.next().expect("pattern 2's own half em");
        assert!(spaces.next().is_none(), "exactly one component");
        assert_eq!(only.amount(), Em::HALF);
        assert_eq!(
            only.referent(),
            Referent::Trailing,
            "the bracket is the boundary's only neighbor"
        );
        assert_eq!(
            only.reduction(),
            Reduction::Rigid,
            "Appendix D states no schedule for a term Table 1 never states"
        );
        assert_eq!(
            only.rule(),
            RuleId::POSITIONING_OF_OPENING_BRACKETS_AT_LINE_HEAD,
            "§3.1.5's own rule, not §B.2#17's, which already has a reader through `placement`"
        );
    }

    #[test]
    fn line_head_opening_bracket_only_fires_at_the_line_head_before_cl_01() {
        let pattern_2 = Policy::JLREQ
            .with(choice(Question::LINE_HEAD_OPENING_BRACKET, "pattern-2"))
            .expect("pattern-2 is a permitted answer");

        // (line head, cl-19): an ideograph starting a line — the synthesis is scoped to
        // cl-01, not every line-head boundary.
        let items = [item(0, Frame::FullEm)];
        let scales = [scale()];
        let text = Text::new("\u{4E9C}", &items, &scales).expect("a well-formed text");
        let a = Adjacency::at_line_head(text, Runs::none(), ItemIndex::new(0), direction_of(false));
        assert_eq!(
            boundary(a, pattern_2).spaces().count(),
            0,
            "the synthesis is scoped to cl-01, not every line-head boundary"
        );

        // (cl-01, cl-01): an interior boundary between two opening brackets — the synthesis
        // is scoped to the line head, not every boundary that happens to precede cl-01.
        let items = [item(0, Frame::HalfEm), item(1, Frame::HalfEm)];
        let scales = [scale()];
        let text = Text::new("((", &items, &scales).expect("a well-formed text");
        let interior =
            Adjacency::between(text, Runs::none(), ItemIndex::new(0), direction_of(false))
                .expect("a two-item text has a boundary at ordinal 0");
        assert_eq!(
            boundary(interior, pattern_2).spaces().count(),
            0,
            "the synthesis is scoped to the line head, not every boundary before cl-01"
        );
    }

    #[test]
    fn line_head_opening_bracket_pattern_2_reports_both_paired_addresses_without_clobbering() {
        // The proof this round's own citation argument rests on: at `(0, 1)` under pattern
        // 2, `rules_fired` names *both* addresses `docs/design/api-spine.md`'s own doc for
        // this Question pairs — `B_2_NOTE_17` through `placement`'s own provenance (real
        // before this round and unchanged by it) and
        // `POSITIONING_OF_OPENING_BRACKETS_AT_LINE_HEAD` through the new space (real only
        // this round) — each in its own slot, neither overwriting the other.
        let items = [item(0, Frame::HalfEm)];
        let scales = [scale()];
        let text = Text::new("(", &items, &scales).expect("a well-formed text");
        let a = Adjacency::at_line_head(text, Runs::none(), ItemIndex::new(0), direction_of(false));
        let pattern_2 = Policy::JLREQ
            .with(choice(Question::LINE_HEAD_OPENING_BRACKET, "pattern-2"))
            .expect("pattern-2 is a permitted answer");

        assert_eq!(
            rules_fired(a, pattern_2).count(),
            3,
            "breakable (Table 2 has no line-head row, so this falls back to Table 1's own \
             citation), placement, and the one synthesized space, each in its own slot"
        );
        assert_eq!(
            rules_fired(a, pattern_2)
                .filter(|&rule| rule == RuleId::B_2_NOTE_17)
                .count(),
            2,
            "the breakable fallback and the placement answer both cite Table 1's own row"
        );
        assert_eq!(
            rules_fired(a, pattern_2)
                .filter(|&rule| rule == RuleId::POSITIONING_OF_OPENING_BRACKETS_AT_LINE_HEAD)
                .count(),
            1,
            "the synthesized space's own citation survives in its own slot"
        );
    }

    #[test]
    fn line_head_opening_bracket_pattern_2_does_not_disturb_an_unrelated_boundary() {
        // The displacement test: turning pattern 2 *on* must not change anything at a
        // coordinate the question does not govern. Re-runs
        // `rules_fired_reports_two_spaces_a_delegation_and_an_expansion_without_clobbering_
        // any_of_them`'s own `(5, 5)` fixture — two conditional spaces (§B.2 note 3's sum), a
        // same-run delegation (§B.2 note 10) and a stated-but-empty Table 6 opportunity —
        // under a policy that also answers pattern 2, and expects the identical result.
        let items = [item(0, Frame::HalfEm), item(1, Frame::HalfEm)];
        let scales = [scale()];
        let text = Text::new("::", &items, &scales).expect("a well-formed text");
        let run = RunId::new(NonZeroU16::new(1).expect("one is non-zero"));
        let construct = Construct::new(ConstructKind::NonJukugoRuby, run, None);
        let slots = [Some(construct), Some(construct)];
        let runs = Runs::new(&slots).expect("one contiguous run over both items");
        let a = Adjacency::between(text, runs, ItemIndex::new(0), direction_of(false))
            .expect("a two-item text has a boundary at ordinal 0");
        let pattern_2 = Policy::JLREQ
            .with(choice(Question::LINE_HEAD_OPENING_BRACKET, "pattern-2"))
            .expect("pattern-2 is a permitted answer");

        let result = boundary(a, pattern_2);
        assert_eq!(
            result.spaces().count(),
            2,
            "§B.2 note 3's own sum, unchanged"
        );
        assert!(
            result.delegation().is_some(),
            "the same-run delegation, unchanged"
        );
        assert_eq!(
            result.expansion(),
            Expansion::None,
            "Table 6's own (5, 5) cell, unchanged"
        );
        assert_eq!(
            rules_fired(a, pattern_2).count(),
            6,
            "unchanged from the pattern-2-agnostic fixture this test re-runs"
        );
        assert_eq!(
            rules_fired(a, pattern_2)
                .filter(|&rule| rule == RuleId::D_2_NOTE_1)
                .count(),
            2,
            "both spaces' own reduction citation, unchanged"
        );
        assert!(
            rules_fired(a, pattern_2).any(|rule| rule == RuleId::B_2_NOTE_10),
            "the delegation, unchanged"
        );
        assert!(
            rules_fired(a, pattern_2).any(|rule| {
                rule
                    == RuleId::OPPORTUNITIES_FOR_INTER_CHARACTER_SPACE_EXPANSION_DURING_LINE_ADJUSTMENT
            }),
            "the expansion citation, unchanged"
        );
    }
}
