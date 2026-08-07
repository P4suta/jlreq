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
//! #11); and the three policy
//! questions whose answer changes a Table 1 amount or a ruby-overhang permission directly
//! (§B.2#2 `spacing.line_end_punctuation`, §B.2#6 `spacing.line_end_full_stop_comma`,
//! §B.2#7 `ruby.overhang_kana`). Every other note is
//! either already the shape Table 1, Table 2 or Appendix D's legend states outright (so the
//! generated cell alone answers it), belongs to kinsoku relaxation or line breaking
//! (`jlreq-line`, M1-b), or is not yet wired in and is named as such at its own site rather
//! than silently answered.

use jlreq_class::{Class, ClassSet, Member, Text, resolve};
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

/// The predicate forms an override may take. Closed, and derived from the notes rather
/// than assumed: a note that no form covers is a build failure in the generator, so the
/// claim that this set is complete is checked rather than asserted.
///
/// Not every form is load-bearing yet — [`Predicate::MemberPair`], [`Predicate::HasRole`],
/// [`Predicate::InFormula`] and [`Predicate::Relaxes`] name the shape a later note takes and
/// are not yet matched by [`boundary`], which is stated in this module's own doc rather than
/// left for a reader to discover by grep.
///
/// JLReq: §B.2, §C.2, §C.3, §3.7.4
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Predicate {
    /// §C.2 note 5: only identical marks are inseparable.
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
    /// legitimate case ([`class_of`]'s own doc states the invariant this bound now holds by
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
}

/// Resolve one item's class, ignoring what the specification cannot decide over one
/// occurrence: [`resolve`] already folds ambiguity into this project's published reading
/// (`jlreq_class`'s own doc), so the only `None` this function passes through is "no item
/// at this ordinal", which `Adjacency`'s own constructors never produce for a valid text.
fn class_of(text: Text<'_>, index: ItemIndex) -> Option<Class> {
    resolve(text, index, Policy::JLREQ).map(jlreq_spec::Answer::value)
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

    let breakable = evaluate_breakable(before_raw, after_raw, cell.rule, policy);

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

    Boundary::new(spaces, breakable, placement, ruby_overhang, delegation)
}

/// The boundary this evaluator states nothing about: cl-17 or cl-18 on either side, or a
/// coordinate the capture does not hold. Absence of a table cell is a fact ("no table
/// constrains this"), not a guessed answer, so every field here is the total-absence value
/// rather than a value this evaluator invented.
fn empty_boundary() -> Boundary {
    let rule = RuleId::SPACING_BETWEEN_CHARACTERS;
    Boundary::new(
        [None, None],
        Answer::new(Breakable::Yes, Provenance::of(rule, Standing::Unstated)),
        Answer::new(
            Placement::Permitted,
            Provenance::of(rule, Standing::Unstated),
        ),
        RubyOverhang::None,
        None,
    )
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

/// §C.1: whether a line may end here, at `policy`'s strictness level.
fn evaluate_breakable(
    before: u8,
    after: u8,
    fallback_rule: RuleId,
    policy: Policy,
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
/// B.2#2 policy override and the §3.1.3 vertical-writing override applied.
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
        let expansion = ranged_cell(crate::generated::table6::CELLS, before, after)
            .map_or(Expansion::None, cell_expansion);
        let rule = if matches!(reduction, Reduction::Rigid) {
            cell.rule
        } else {
            reduction_rule
        };
        *slot = Some(ConditionalSpace::new(
            term.amount,
            referent,
            reduction,
            expansion,
            rule,
        ));
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
/// JLReq: §B, §C, §D, §E
pub fn rules_fired(a: Adjacency<'_>, policy: Policy) -> impl Iterator<Item = RuleId> {
    let result = boundary(a, policy);
    let mut rules = [None; 5];
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
        }
    }
    rules.into_iter().flatten()
}

#[cfg(test)]
mod tests {
    use jlreq_class::{Class, Text};
    use jlreq_spec::{Choice, Policy, Question, RuleId, Standing};
    use jlreq_unit::{
        Advance, ByteOffset, Direction, Em, Frame, InlineExtent, Item, ItemIndex, Role, Runs,
        Scale, ScaleId,
    };

    use super::{
        Adjacency, boundary, ruby_overhang_kana_withdrawn, ruby_overhang_withdrawal_classes,
        rules_fired,
    };
    use crate::boundary::Breakable;
    use crate::space::{Reduction, Referent};

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
}
