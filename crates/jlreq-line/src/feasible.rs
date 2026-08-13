// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Break candidates and the kinsoku evaluator: [`Candidate`], [`Feasible`], [`FeasibleBreak`].
//!
//! # Status
//!
//! [`Feasible::compute`] is a real, wired evaluator: for an ordinary interior candidate it
//! reads [`jlreq_spacing::boundary`]'s already-built Table 2 breakability (§3.1.7, §3.1.8,
//! at every strictness level `Question::KINSOKU_LEVEL` selects — a class being unable to
//! *start* a line is exactly "may a line end between these two classes" read at an
//! ordinary interior break) and Table 1 placement (`×`) at the two edges the break would
//! create. §C.2 note 12's caller-supplied hyphenation discretionary is read directly off
//! the generated cl-27×cl-27 cell of Table 2 (`prohibited: false, levels: 0b1111` — every
//! level refuses it by default) and is the one candidate kind this evaluator exempts from
//! that refusal, because without the exemption [`Candidate::Discretionary`] would mean
//! nothing.
//!
//! §C.2 notes 6 through 8 and 13's same-run breakability — the one fact no table cell can
//! express, because a class pair alone does not say whether two items share one construct
//! run (`docs/design/api-spine.md`'s own words on [`Feasible::compute`]: "the same-run
//! refusals... are decided here, in the crate that owns break refusal", ADR-0015) — is
//! `same_run_refusal`'s own answer, real as of this pass and called from every candidate
//! [`Feasible::compute`] evaluates (its own doc, below, states the four notes, the one
//! reading this pass had to adjudicate, and the construct kinds none of the four governs).
//! It is reachable today through the public [`Feasible::compute`] alone:
//! [`crate::compose::compose`] still composes plain text (see that function's own comment
//! at its `Runs::none()` call), so a caller has to build a real [`jlreq_unit::Runs`]
//! overlay and call `compute` directly for the refusal ever to fire — this module's own
//! tests do exactly that.

use alloc::vec::Vec;

use jlreq_class::Text;
use jlreq_spacing::{Adjacency, Breakable, Placement, boundary};
use jlreq_spec::{Policy, Provenance, RuleId};
use jlreq_unit::{ByteOffset, ConstructKind, Direction, InlineExtent, ItemIndex, Runs};

/// A break the caller's UAX #14 implementation offered, in the caller's coordinates.
///
/// JLReq can only *remove* opportunities the caller offered, never add them (ADR-0003).
/// The one exception the specification names is hyphenation, which is not an added
/// opportunity but a caller-supplied discretionary.
///
/// A candidate at byte offset zero or at the end of the text names the paragraph's own
/// edges rather than an interior break. Both are accepted and neither creates a line: the
/// last line ends where the text does, whether or not a candidate says so. They are
/// accepted rather than refused because every UAX #14 implementation an adopter already
/// runs emits the second, and a library that made callers strip it would be charging them
/// for our tidiness (ADR-0018).
///
/// JLReq: §3.2.6, ADR-0003, ADR-0018
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Candidate {
    /// A plain opportunity at a byte offset.
    At(ByteOffset),
    /// §C.2 note 12: "In order to break a line in the middle of a Western word, it needs
    /// to be divided into two syllables first. Then a line can be broken between the two
    /// by adding HYPHEN at the line end." Taking this break inserts a glyph and
    /// lengthens the line, so the caller supplies its advance.
    ///
    /// JLReq: §C.2#12, §3.2.6
    Discretionary {
        /// Where the break falls.
        at: ByteOffset,
        /// How much the inserted glyph adds to the line that ends here.
        pre_break: InlineExtent,
    },
}

impl Candidate {
    /// Where this candidate falls.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn at(self) -> ByteOffset {
        match self {
            Self::At(at) | Self::Discretionary { at, .. } => at,
        }
    }

    /// How much the break itself adds to the line that ends here: zero for an ordinary
    /// opportunity, the caller's stated advance for a discretionary hyphen.
    ///
    /// JLReq: §C.2#12
    #[must_use]
    pub const fn pre_break(self) -> InlineExtent {
        match self {
            Self::At(_) => InlineExtent::ZERO,
            Self::Discretionary { pre_break, .. } => pre_break,
        }
    }
}

/// An ordinal into the caller's own candidate slice.
///
/// ADR-0003 says kumihan may only remove opportunities, never add one. That is held by
/// this type rather than asserted in prose: a feasible break stores the ordinal of the
/// candidate it came from, so a break that is not one of the caller's candidates has no
/// representation and the subset property needs no test.
///
/// JLReq: n/a (addressing)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct CandidateIndex(u32);

impl CandidateIndex {
    /// The candidate at ordinal `index` of the caller's own slice.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The ordinal.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Breaks that kinsoku permits.
///
/// No public constructor, on this type or on [`FeasibleBreak`]: only
/// [`Feasible::compute`] can build one, so the optimizer cannot be handed a prohibited
/// break even by a caller who wants to (ADR-0010).
///
/// JLReq: §3.1.7, §3.1.8, §C.2#12
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Feasible<'r> {
    breaks: Vec<FeasibleBreak>,
    rejected: Vec<(CandidateIndex, RuleId)>,
    _lifetime: core::marker::PhantomData<&'r ()>,
}

impl<'r> Feasible<'r> {
    /// [`Runs`] is a parameter rather than a field of the items, so the same-run
    /// refusals of §C.2 notes 6 through 8 and 13 belong here, in the crate that owns
    /// break refusal, and appear in [`Feasible::rejected`] with their rule like every
    /// other refusal (ADR-0015) whenever a caller supplies a `Runs` naming a governed
    /// construct; see this module's own `# Status` and `same_run_refusal`'s own doc.
    ///
    /// The `×` of Tables 1 and 2 is refused here too, and is not a separate mechanism.
    /// Its Japanese legend says the placement is prohibited by 行頭禁則, 行末禁則 or
    /// another rule, which is the kinsoku prohibition restated at a line edge, so a
    /// candidate whose resulting line edge would produce one is rejected with that cell
    /// as its rule.
    ///
    /// A candidate at the text's own start or end is accepted unconditionally: neither is
    /// an adjacency kinsoku evaluates, both are the paragraph's own edges (ADR-0018). A
    /// candidate whose byte offset names neither an item's start nor the text's own end
    /// is passed over rather than guessed at: kinsoku has no adjacency to evaluate there,
    /// so it is neither accepted nor refused by a rule.
    ///
    /// JLReq: §B.1, §C.1, §D.1, §E.1 legends
    #[must_use]
    pub fn compute(
        text: Text<'r>,
        runs: Runs<'r>,
        candidates: &'r [Candidate],
        policy: Policy,
        direction: Direction,
    ) -> Self {
        let mut breaks = Vec::with_capacity(candidates.len());
        let mut rejected = Vec::new();

        for (ordinal, candidate) in candidates.iter().enumerate() {
            let index = CandidateIndex::new(u32::try_from(ordinal).unwrap_or(u32::MAX));
            let Some(at) = item_boundary(text, *candidate) else {
                continue;
            };

            match verdict(text, runs, *candidate, at, policy, direction) {
                Ok(why) => breaks.push(FeasibleBreak::new(index, at, *candidate, why)),
                Err(rule) => rejected.push((index, rule)),
            }
        }

        Self {
            breaks,
            rejected,
            _lifetime: core::marker::PhantomData,
        }
    }

    /// The candidates kinsoku leaves standing, in the order the caller supplied them.
    ///
    /// JLReq: §3.1.7, §3.1.8
    #[must_use]
    pub fn breaks(&self) -> &[FeasibleBreak] {
        &self.breaks
    }

    /// Candidates that were refused, each with the rule that refused it. A caller can
    /// see why its opportunity disappeared instead of guessing.
    ///
    /// JLReq: §3.1.7, §3.1.8
    #[must_use]
    pub fn rejected(&self) -> &[(CandidateIndex, RuleId)] {
        &self.rejected
    }
}

/// A break kinsoku permits, at one item boundary.
///
/// JLReq: §3.1.7, §3.1.8
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FeasibleBreak {
    candidate: CandidateIndex,
    at: ItemIndex,
    pre_break: InlineExtent,
    why: Provenance,
}

impl FeasibleBreak {
    /// Build one. Crate-visible: [`Feasible::compute`] is the only producer (ADR-0010).
    const fn new(
        candidate: CandidateIndex,
        at: ItemIndex,
        source: Candidate,
        why: Provenance,
    ) -> Self {
        Self {
            candidate,
            at,
            pre_break: source.pre_break(),
            why,
        }
    }

    /// Which of the caller's own candidates this is (ADR-0003).
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn candidate(self) -> CandidateIndex {
        self.candidate
    }

    /// The item that would begin the following line.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn at(self) -> ItemIndex {
        self.at
    }

    /// What breaking here adds to the line that ends here.
    ///
    /// JLReq: §C.2#12
    #[must_use]
    pub const fn pre_break(self) -> InlineExtent {
        self.pre_break
    }

    /// Why kinsoku permits it.
    ///
    /// JLReq: §3.1.7, §3.1.8
    #[must_use]
    pub const fn why(self) -> Provenance {
        self.why
    }
}

/// The rule this evaluator cites for a fact about *where* the adjacencies of Appendix C
/// are, rather than about which one is breakable — the paragraph's own edge, and the one
/// defensive fallback [`verdict`] cannot actually reach given how [`item_boundary`] builds
/// its argument.
///
/// JLReq: n/a (addressing)
const APPENDIX_C_SCOPE: RuleId = RuleId::POSSIBILITIES_FOR_LINE_BREAKING_BETWEEN_CHARACTERS;

/// The item ordinal a candidate's byte offset names — the item that would open the
/// following line — or `None` when the offset lands inside an item rather than on a
/// boundary this stream's own segmentation carries.
///
/// Byte offset zero and the text's own length are always boundaries, whether or not any
/// item starts or ends exactly there (ADR-0018): the paragraph itself has two edges no
/// item is required to align with.
fn item_boundary(text: Text<'_>, candidate: Candidate) -> Option<ItemIndex> {
    let offset = candidate.at().get();
    let items = text.items();
    let total = u32::try_from(items.len()).unwrap_or(u32::MAX);
    if offset == 0 {
        return Some(ItemIndex::new(0));
    }
    if offset as usize == text.as_str().len() {
        return Some(ItemIndex::new(total));
    }
    items
        .iter()
        .position(|item| item.start().get() == offset)
        .map(|position| ItemIndex::new(u32::try_from(position).unwrap_or(u32::MAX)))
}

/// The item that would close the line before an item-ordinal boundary, and the item that
/// would open the line after it — `None` on either side at a paragraph edge.
struct Boundary {
    before: Option<ItemIndex>,
    after: Option<ItemIndex>,
}

/// Split an item-ordinal boundary into the item before it and the item after it.
fn split(item_count: u32, at: ItemIndex) -> Boundary {
    let ordinal = at.get();
    Boundary {
        before: (ordinal > 0).then(|| ItemIndex::new(ordinal.saturating_sub(1))),
        after: (ordinal < item_count).then_some(at),
    }
}

/// Whether kinsoku permits breaking at item boundary `at`: `Ok` with the provenance of the
/// rule that permits it, `Err` with the rule that refuses it.
fn verdict(
    text: Text<'_>,
    runs: Runs<'_>,
    candidate: Candidate,
    at: ItemIndex,
    policy: Policy,
    direction: Direction,
) -> Result<Provenance, RuleId> {
    let item_count = u32::try_from(text.items().len()).unwrap_or(u32::MAX);
    let Boundary {
        before: Some(before),
        after: Some(after),
    } = split(item_count, at)
    else {
        // The paragraph's own edge: not an adjacency kinsoku evaluates, and always
        // accepted (ADR-0018).
        return Ok(Provenance::of(
            APPENDIX_C_SCOPE,
            APPENDIX_C_SCOPE.standing(),
        ));
    };

    let permitted = if matches!(candidate, Candidate::Discretionary { .. }) {
        // §C.2#12: a caller-supplied discretionary is exempt from Table 2's default
        // refusal between two consecutive Western characters — the one prohibition this
        // candidate kind exists to lift.
        Provenance::of(RuleId::C_2_NOTE_12, RuleId::C_2_NOTE_12.standing())
    } else {
        let adjacency =
            Adjacency::between(text, runs, before, direction).ok_or(APPENDIX_C_SCOPE)?;
        let answer = boundary(adjacency, policy);
        match answer.breakable().value() {
            Breakable::No { rule } => return Err(rule),
            Breakable::Yes => answer.breakable().why(),
        }
    };

    // §C.2 notes 6 through 8 and 13 run after Table 2's own class-pair verdict above, not
    // before it, and the ordering is structural rather than a choice stated anywhere: the
    // `Err(rule)` inside the `Breakable::No` arm above already returns before this line is
    // reached, so `same_run_refusal` only ever sees a boundary Table 2 has already
    // permitted, and a boundary both would refuse is always reported under Table 2's own
    // rule. This call also runs unconditionally after either arm of the `if` above,
    // `Candidate::Discretionary` included: a caller-supplied hyphen exempted from Table 2's
    // default cl-27×cl-27 refusal (§C.2#12) is still refused here if it falls inside one of
    // the four governed construct runs — the discretionary lifts Table 2's own prohibition,
    // not this one.
    if let Some(rule) = same_run_refusal(runs, before, after) {
        return Err(rule);
    }

    let at_line_end = boundary(
        Adjacency::at_line_end(text, runs, before, direction),
        policy,
    );
    if let Placement::Forbidden { rule } = at_line_end.placement().value() {
        return Err(rule);
    }
    let at_line_head = boundary(
        Adjacency::at_line_head(text, runs, after, direction),
        policy,
    );
    if let Placement::Forbidden { rule } = at_line_head.placement().value() {
        return Err(rule);
    }

    Ok(permitted)
}

/// §C.2 notes 6 through 8 and 13's same-run breakability: no break inside one ornamented
/// complex (cl-21), one simple-ruby complex (cl-22), one jukugo-ruby complex (cl-23,
/// subject to the group-level adjudication below), or one tate-chu-yoko run (cl-30); a
/// break is permitted between two different runs of the same kind, and between an item in
/// no construct and any neighbor — [`Runs::of`] answers `None` for both, and the `?` below
/// reads that `None` the identical way on either side.
///
/// Table 2's generated cell is a function of the two classes alone
/// (`crates/jlreq-spacing/src/raw.rs`'s `RawBreakCell` carries `before`, `after`,
/// `prohibited` and `levels`, and no run field), so it cannot distinguish "two different
/// ornamented complexes, adjacent" from "the middle of one" — the same class pair either
/// way. `docs/design/api-spine.md`'s own text on [`Feasible::compute`] states that this
/// decision belongs here rather than in `jlreq-spacing`, which is why it is a function of
/// this crate rather than a further call into that one (ADR-0015).
///
/// # The three unconditional notes
///
/// §C.2#6, §C.2#7 and §C.2#13 share one shape: no break inside the same run, a break
/// between two runs of the kind. [`Runs::new`]'s own constructor already refuses an
/// overlay where one [`jlreq_unit::RunId`] names two different [`ConstructKind`]s
/// ([`jlreq_unit::RunsError::RunKindConflict`]), so once `before.run() == after.run()`
/// holds, `before.kind() == after.kind()` is guaranteed by the overlay's own invariant
/// rather than reread below.
///
/// # §C.2#8, the one note whose same-run answer is not "always refuse"
///
/// Jukugo-ruby's (cl-23) own second sentence grants a break between two consecutive base
/// characters of the *same* complex and between two runs of ruby text; only its third
/// sentence — a base character and its own accompanying ruby text are indivisible —
/// refuses one, at the level below the run [`jlreq_unit::Construct::group`] carries for
/// exactly this note. Two occurrences with equal, declared [`jlreq_unit::GroupId`]s are
/// refused; two with unequal, declared ids are permitted, unconditionally on kind, per the
/// note's own second sentence.
///
/// **The adjudicated case: one or both sides carry no group**
/// (`docs/decisions/jukugo-ruby-unset-group.md`). §C.2#8 states nothing about
/// an occurrence with no declared group, so this pass answers it rather than letting
/// `Some(g) == Some(g)` decide it by the accident of `None == None`: **absent a matching,
/// declared group on both sides, this function permits the break.** It refuses only on
/// positive evidence that the two occurrences share one indivisible base-and-ruby unit —
/// covering `(None, None)` and the mixed `(Some(_), None)` the identical way — and permits
/// two occurrences declared in different groups exactly as the note's own second sentence
/// already requires regardless of group. This is the direction the note's own words argue
/// for, not merely the safer of two guesses: [`jlreq_unit::Construct::group`]'s own doc
/// frames the level as "the ruby characters attached to one base character," and this
/// crate's own item stream never carries ruby text at all — a jukugo-ruby run's
/// accompanying reading is a nested `Segment` `jlreq_inline::Contribution` would place, and
/// the crate graph gives `jlreq-line` no edge to `jlreq-inline` (`crate`'s own module doc;
/// jukugo-ruby is M4-a) — so every occurrence that can reach this function tagged
/// [`ConstructKind::JukugoRuby`] today is a base character, exactly the pairing the note's
/// own second sentence names unconditionally on group. Refusing whenever a group is absent
/// would consequently apply a group-level prohibition (indivisibility) to the run-level
/// case the note states in its most explicit sentence a break exists for — inventing a
/// prohibition from data the caller did not supply, the wrong direction for a library that
/// only ever removes an opportunity the caller offered (ADR-0003's own spirit, read onto a
/// construct declaration rather than onto a candidate). What would change this reading: a
/// caller-declared role distinguishing a base occurrence from a ruby occurrence within one
/// `JukugoRuby` run, or a sentence of the specification conditioning the second sentence's
/// own permission on group membership rather than stating it without one.
///
/// # The kinds these four notes do not govern
///
/// [`ConstructKind::ReferenceMark`], [`ConstructKind::WarichuInterior`],
/// [`ConstructKind::Furiwake`] and [`ConstructKind::MathFormula`] are real constructs this
/// workspace already models, and none of §C.2 notes 6, 7, 8 or 13 addresses their own
/// same-run breakability — refusing a break for any of them here would be inventing a
/// prohibition no sentence states, so this function answers `None` for all four rather
/// than guessing which of them behaves like the other four:
///
/// - `ReferenceMark` (cl-20): §A.20 and §4.2.3 place it; no §C.2 note reaches it.
/// - `WarichuInterior`: §3.4.2 states whether a line may break inside a warichu — a
///   different address from these four notes, `[[owned]]` at M1 in
///   `docs/conformance-deferrals.toml` with no layer answering it yet; its own straddle
///   across two lines is §3.4.3, deferred to M4.
/// - `Furiwake`: §3.1.10 item 12 makes "a unit of furiwake" one object, a same-run
///   indivisibility of the identical shape as these four notes but stated at a different
///   address; §3.7.2 places the construct itself, both deferred to M4.
/// - `MathFormula`: §3.7.4 states the spacing a formula's own setting chooses, deferred to
///   M4, and no note here addresses its own same-run breakability either.
///
/// JLReq: §C.2#6, §C.2#7, §C.2#8, §C.2#13
fn same_run_refusal(runs: Runs<'_>, before: ItemIndex, after: ItemIndex) -> Option<RuleId> {
    let before = runs.of(before)?;
    let after = runs.of(after)?;
    if before.run() != after.run() {
        return None;
    }
    match before.kind() {
        ConstructKind::Ornamented => Some(RuleId::C_2_NOTE_6),
        ConstructKind::NonJukugoRuby => Some(RuleId::C_2_NOTE_7),
        ConstructKind::TateChuYoko => Some(RuleId::C_2_NOTE_13),
        ConstructKind::JukugoRuby => match (before.group(), after.group()) {
            (Some(before_group), Some(after_group)) if before_group == after_group => {
                Some(RuleId::C_2_NOTE_8)
            },
            _ => None,
        },
        // §C.2 notes 6 through 8 and 13 govern exactly the four kinds matched above.
        // `ReferenceMark`, `WarichuInterior`, `Furiwake` and `MathFormula` — named in
        // full in this function's own doc, with what governs each instead — fall to this
        // wildcard on the same terms as a `ConstructKind` this workspace has not named
        // yet: nothing here refuses a break for it, whether because no note reaches it or
        // because its own note's target is not yet reachable
        // (`jlreq_spacing::evaluate::delegation_of` is the precedent for this shape).
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU16;

    use jlreq_class::Text;
    use jlreq_spec::{Policy, RuleId};
    use jlreq_unit::{
        Advance, ByteOffset, Construct, ConstructKind, Direction, Frame, GroupId, InlineExtent,
        Item, RunId, Runs, Scale, ScaleId,
    };

    use super::{Candidate, CandidateIndex, Feasible};

    /// Two ideographs ("亜亜"), the shared fixture: Table 1's and Table 2's own cl-19
    /// against cl-19 cell is blank — no prohibition, no conditional space, no line-edge
    /// placement concern — so the single interior candidate every test below asks about
    /// reaches `same_run_refusal` rather than being decided, refused or permitted, by
    /// anything earlier in `verdict`.
    fn two_ideographs() -> [Item; 2] {
        let extent = || InlineExtent::new(1000).expect("a valid extent");
        [
            Item::new(ByteOffset::new(0), extent(), ScaleId::new(0)).with_frame(Frame::FullEm),
            Item::new(ByteOffset::new(3), extent(), ScaleId::new(0)).with_frame(Frame::FullEm),
        ]
    }

    /// The one declared size both ideographs are set at.
    fn one_em() -> Scale {
        let em = Advance::new(1000).expect("a positive advance");
        Scale::new(em, em).expect("a positive scale")
    }

    /// The single interior candidate: the boundary between the two ideographs, at the
    /// second item's own byte offset. Every test in this module supplies exactly this one
    /// candidate, so a permitted answer is `CandidateIndex::new(0)` in [`Feasible::breaks`]
    /// and a refused one is the same index in [`Feasible::rejected`] — there is no second
    /// candidate either could be confused with.
    fn interior_candidate() -> [Candidate; 1] {
        [Candidate::At(ByteOffset::new(3))]
    }

    /// The identity a test names as `run(1)`, `run(2)`, and so on.
    fn run(id: u16) -> RunId {
        RunId::new(NonZeroU16::new(id).expect("a nonzero run id"))
    }

    /// The identity a test names as `group(1)`, `group(2)`, and so on.
    fn group(id: u16) -> GroupId {
        GroupId::new(NonZeroU16::new(id).expect("a nonzero group id"))
    }

    /// Compute [`Feasible`] for the two-ideograph fixture under one declared overlay.
    /// Every argument is already a caller-owned local borrowed for one shared lifetime
    /// `'a`, matching [`Feasible::compute`]'s own `Feasible<'r>`: a function that built
    /// `text` or `candidates` from its own locals could only return a value borrowing
    /// them, which cannot outlive the call, so this one takes them from the test instead.
    fn feasible_over<'a>(
        items: &'a [Item],
        scales: &'a [Scale],
        candidates: &'a [Candidate],
        slots: &'a [Option<Construct>],
    ) -> Feasible<'a> {
        let text = Text::new("亜亜", items, scales).expect("a well formed stream");
        let runs = Runs::new(slots).expect("a contiguous, single-kind overlay");
        Feasible::compute(text, runs, candidates, Policy::JLREQ, Direction::Horizontal)
    }

    #[test]
    fn two_items_in_the_same_ornamented_run_are_refused_with_c_2_note_6() {
        let items = two_ideographs();
        let scales = [one_em()];
        let candidates = interior_candidate();
        let slots = [
            Some(Construct::new(ConstructKind::Ornamented, run(1), None)),
            Some(Construct::new(ConstructKind::Ornamented, run(1), None)),
        ];
        let feasible = feasible_over(&items, &scales, &candidates, &slots);
        assert_eq!(
            feasible.rejected().len(),
            1,
            "exactly the one declared candidate, and nothing else, is refused"
        );
        assert!(
            feasible
                .rejected()
                .iter()
                .any(|&(index, rule)| index == CandidateIndex::new(0)
                    && rule == RuleId::C_2_NOTE_6),
            "§C.2#6's first sentence denies a break 'between two consecutive characters \
             belonging to the same ornamented character complex (cl-21)'. Both items name \
             the identical `RunId`, so `same_run_refusal` refuses the boundary rather than \
             leaving it to Table 2's own cl-19×cl-19 cell, which alone would have \
             permitted it."
        );
    }

    #[test]
    fn two_items_in_different_ornamented_runs_are_not_refused_by_this_function() {
        let items = two_ideographs();
        let scales = [one_em()];
        let candidates = interior_candidate();
        let slots = [
            Some(Construct::new(ConstructKind::Ornamented, run(1), None)),
            Some(Construct::new(ConstructKind::Ornamented, run(2), None)),
        ];
        let feasible = feasible_over(&items, &scales, &candidates, &slots);
        assert!(
            feasible
                .breaks()
                .iter()
                .any(|b| b.candidate() == CandidateIndex::new(0)),
            "§C.2#6's second sentence: 'If two consecutive characters belong to different \
             ornamented character complexes (cl-21), a line break opportunity exists \
             between them.' The two items name different `RunId`s, so `same_run_refusal` \
             declines and the candidate is asserted present in `breaks()` rather than \
             merely absent from `rejected()` — an absence there is also what a wrong, \
             unrelated refusal would look like."
        );
    }

    #[test]
    fn two_items_in_the_same_non_jukugo_ruby_run_are_refused_with_c_2_note_7() {
        let items = two_ideographs();
        let scales = [one_em()];
        let candidates = interior_candidate();
        let slots = [
            Some(Construct::new(ConstructKind::NonJukugoRuby, run(1), None)),
            Some(Construct::new(ConstructKind::NonJukugoRuby, run(1), None)),
        ];
        let feasible = feasible_over(&items, &scales, &candidates, &slots);
        assert_eq!(
            feasible.rejected().len(),
            1,
            "exactly the one declared candidate, and nothing else, is refused"
        );
        assert!(
            feasible
                .rejected()
                .iter()
                .any(|&(index, rule)| index == CandidateIndex::new(0)
                    && rule == RuleId::C_2_NOTE_7),
            "§C.2#7's first sentence, the identical shape as §C.2#6 over the simple-ruby \
             complex (cl-22): no break inside one run."
        );
    }

    #[test]
    fn two_items_in_different_non_jukugo_ruby_runs_are_not_refused_by_this_function() {
        let items = two_ideographs();
        let scales = [one_em()];
        let candidates = interior_candidate();
        let slots = [
            Some(Construct::new(ConstructKind::NonJukugoRuby, run(1), None)),
            Some(Construct::new(ConstructKind::NonJukugoRuby, run(2), None)),
        ];
        let feasible = feasible_over(&items, &scales, &candidates, &slots);
        assert!(
            feasible
                .breaks()
                .iter()
                .any(|b| b.candidate() == CandidateIndex::new(0)),
            "§C.2#7's second sentence permits a break between two different simple-ruby \
             complexes; different `RunId`s, so `same_run_refusal` declines and the \
             candidate stands."
        );
    }

    #[test]
    fn two_items_in_the_same_tate_chu_yoko_run_are_refused_with_c_2_note_13() {
        let items = two_ideographs();
        let scales = [one_em()];
        let candidates = interior_candidate();
        let slots = [
            Some(Construct::new(ConstructKind::TateChuYoko, run(1), None)),
            Some(Construct::new(ConstructKind::TateChuYoko, run(1), None)),
        ];
        let feasible = feasible_over(&items, &scales, &candidates, &slots);
        assert_eq!(
            feasible.rejected().len(),
            1,
            "exactly the one declared candidate, and nothing else, is refused"
        );
        assert!(
            feasible.rejected().iter().any(
                |&(index, rule)| index == CandidateIndex::new(0) && rule == RuleId::C_2_NOTE_13
            ),
            "§C.2#13's first sentence: no break 'between two consecutive characters \
             belonging to the same set of characters in tate-chu-yoko (cl-30)'."
        );
    }

    #[test]
    fn two_items_in_different_tate_chu_yoko_runs_are_not_refused_by_this_function() {
        let items = two_ideographs();
        let scales = [one_em()];
        let candidates = interior_candidate();
        let slots = [
            Some(Construct::new(ConstructKind::TateChuYoko, run(1), None)),
            Some(Construct::new(ConstructKind::TateChuYoko, run(2), None)),
        ];
        let feasible = feasible_over(&items, &scales, &candidates, &slots);
        assert!(
            feasible
                .breaks()
                .iter()
                .any(|b| b.candidate() == CandidateIndex::new(0)),
            "§C.2#13's second sentence permits a break between two different \
             tate-chu-yoko runs; different `RunId`s, so `same_run_refusal` declines."
        );
    }

    #[test]
    fn two_base_characters_of_one_jukugo_ruby_complex_in_the_same_group_are_refused_with_c_2_note_8()
     {
        let items = two_ideographs();
        let scales = [one_em()];
        let candidates = interior_candidate();
        let slots = [
            Some(Construct::new(
                ConstructKind::JukugoRuby,
                run(1),
                Some(group(1)),
            )),
            Some(Construct::new(
                ConstructKind::JukugoRuby,
                run(1),
                Some(group(1)),
            )),
        ];
        let feasible = feasible_over(&items, &scales, &candidates, &slots);
        assert_eq!(
            feasible.rejected().len(),
            1,
            "exactly the one declared candidate, and nothing else, is refused"
        );
        assert!(
            feasible
                .rejected()
                .iter()
                .any(|&(index, rule)| index == CandidateIndex::new(0)
                    && rule == RuleId::C_2_NOTE_8),
            "§C.2#8's third sentence: 'a base character and the accompanying ruby text \
             shall be indivisible'. Equal, declared `GroupId`s are this function's own \
             positive evidence that the two occurrences share one base-and-ruby unit, so \
             the boundary is refused even though both sides are also in the same run — the \
             fact the next test shows is not, on its own, what §C.2#8 refuses."
        );
    }

    #[test]
    fn two_base_characters_of_one_jukugo_ruby_complex_in_different_groups_are_not_refused() {
        let items = two_ideographs();
        let scales = [one_em()];
        let candidates = interior_candidate();
        let slots = [
            Some(Construct::new(
                ConstructKind::JukugoRuby,
                run(1),
                Some(group(1)),
            )),
            Some(Construct::new(
                ConstructKind::JukugoRuby,
                run(1),
                Some(group(2)),
            )),
        ];
        let feasible = feasible_over(&items, &scales, &candidates, &slots);
        assert!(
            feasible
                .breaks()
                .iter()
                .any(|b| b.candidate() == CandidateIndex::new(0)),
            "§C.2#8's second sentence: 'There is also a line break opportunity between two \
             consecutive base characters belonging to the same jukugo-ruby character \
             complex (cl-23)'. This is the asymmetry a naive same-run implementation gets \
             wrong: both items share `run(1)`, exactly the fact that refuses a break for \
             the other three governed kinds, but jukugo-ruby refuses only at the group \
             level below the run, and these two groups differ."
        );
    }

    #[test]
    fn two_base_characters_of_one_jukugo_ruby_complex_with_no_declared_group_are_not_refused() {
        let items = two_ideographs();
        let scales = [one_em()];
        let candidates = interior_candidate();
        let slots = [
            Some(Construct::new(ConstructKind::JukugoRuby, run(1), None)),
            Some(Construct::new(ConstructKind::JukugoRuby, run(1), None)),
        ];
        let feasible = feasible_over(&items, &scales, &candidates, &slots);
        assert!(
            feasible
                .breaks()
                .iter()
                .any(|b| b.candidate() == CandidateIndex::new(0)),
            "The adjudicated case `same_run_refusal`'s own doc argues for: §C.2#8 states \
             nothing about an occurrence with no declared group, and `None == None` is not \
             treated as the positive evidence of shared indivisibility the same-group case \
             above supplies. Refusing here would apply the group-level prohibition to the \
             plain same-complex case §C.2#8's own second sentence permits unconditionally \
             on group — exactly the conflation with `Ornamented`'s always-refuse shape this \
             reading exists to avoid."
        );
    }

    #[test]
    fn two_items_in_the_same_warichu_interior_run_are_not_refused_by_this_function() {
        let items = two_ideographs();
        let scales = [one_em()];
        let candidates = interior_candidate();
        let slots = [
            Some(Construct::new(ConstructKind::WarichuInterior, run(1), None)),
            Some(Construct::new(ConstructKind::WarichuInterior, run(1), None)),
        ];
        let feasible = feasible_over(&items, &scales, &candidates, &slots);
        assert!(
            feasible
                .breaks()
                .iter()
                .any(|b| b.candidate() == CandidateIndex::new(0)),
            "None of §C.2 notes 6, 7, 8 or 13 addresses a warichu interior's own same-run \
             breakability — §3.4.2 does, a different address this function does not answer \
             — so `same_run_refusal` declines for this kind exactly as it does for an item \
             in no construct at all, rather than guessing it behaves like `Ornamented`."
        );
    }

    #[test]
    fn text_with_no_declared_overlay_is_identical_to_todays_answer() {
        let scales = [one_em()];
        let items = two_ideographs();
        let text = Text::new("亜亜", &items, &scales).expect("a well formed stream");
        let candidates = interior_candidate();
        let feasible = Feasible::compute(
            text,
            Runs::none(),
            &candidates,
            Policy::JLREQ,
            Direction::Horizontal,
        );
        assert!(
            feasible
                .breaks()
                .iter()
                .any(|b| b.candidate() == CandidateIndex::new(0)),
            "`Runs::none()` answers `None` for every item, exactly as an item in no \
             construct does, so `same_run_refusal` declines for every boundary a caller \
             who declares no overlay ever asks about — the guarantee that filling this \
             function does not change what `crate::compose::compose` reports today, since \
             it always passes `Runs::none()` (see that function's own comment)."
        );
        assert!(
            feasible
                .rejected()
                .iter()
                .all(|&(_, rule)| rule != RuleId::C_2_NOTE_6
                    && rule != RuleId::C_2_NOTE_7
                    && rule != RuleId::C_2_NOTE_8
                    && rule != RuleId::C_2_NOTE_13),
            "restated over `rejected()` directly: none of the four notes this function \
             answers ever fires when the overlay declares nothing."
        );
    }
}
