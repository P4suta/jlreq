// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Construct runs: the same-run predicate the spacing rules ask about, and the overlay
//! that carries it.
//!
//! Run identity is here and **not** on an item, because an item is what the caller
//! measured and a run is what lowering computed; two carriers of one fact are two things
//! a caller can desynchronize (see `docs/adr/0015` and `docs/adr/0019`).

use core::num::NonZeroU16;

use crate::item::ItemIndex;

/// Which construct run this occurrence belongs to.
///
/// Run identity *is* the same-run/different-run predicate of §B.2 notes 9 through 11 and
/// §C.2 notes 6 through 8 and 13, so `jlreq-spacing` compares two of these for equality
/// without knowing that equality means "the same ruby group". `group` is the one further
/// level §C.2 note 8 needs: a break is allowed between two base characters of one
/// jukugo-ruby (熟語ルビ) complex but not between the ruby characters attached to one base
/// character. No note anywhere needs a second level.
///
/// JLReq: §A.20–§A.23, §A.30, §B.2#9–#11, §C.2#6–#8, §C.2#13
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Construct {
    kind: ConstructKind,
    run: RunId,
    group: Option<GroupId>,
}

impl Construct {
    /// One occurrence's membership: the kind of construct, the run it belongs to, and
    /// the group inside that run where §C.2 note 8 needs one.
    ///
    /// JLReq: §B.2#9–#11, §C.2#6–#8, §C.2#13
    #[must_use]
    pub const fn new(kind: ConstructKind, run: RunId, group: Option<GroupId>) -> Self {
        Self { kind, run, group }
    }

    /// Which construct this is one occurrence of. JLReq: §A.20–§A.23, §A.30
    #[must_use]
    pub const fn kind(self) -> ConstructKind {
        self.kind
    }

    /// The run identity the same-run predicates compare. JLReq: §B.2#9–#11
    #[must_use]
    pub const fn run(self) -> RunId {
        self.run
    }

    /// The group inside the run, where the specification needs one. JLReq: §C.2#8
    #[must_use]
    pub const fn group(self) -> Option<GroupId> {
        self.group
    }
}

/// Which of JLReq's constructs a run is.
///
/// JLReq: §A.20–§A.23, §A.30, §3.4.2, §3.7.2, §3.7.4
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConstructKind {
    /// cl-20, characters as reference marks (合印). JLReq: §A.20, §4.2.3
    ReferenceMark,
    /// cl-21, ornamented character complex — a base character with its superscripts and
    /// subscripts, which §3.7.1 makes indivisible and unexpandable. JLReq: §A.21, §3.7.1
    Ornamented,
    /// cl-22, simple-ruby complex — mono-ruby *and* group-ruby, per §A.22's
    /// "ruby other than jukugo-ruby". JLReq: §A.22, §3.3.5, §3.3.6
    NonJukugoRuby,
    /// cl-23, jukugo-ruby complex. JLReq: §A.23, §3.3.7
    JukugoRuby,
    /// cl-30, characters in tate-chu-yoko (縦中横). JLReq: §A.30, §3.2.5
    TateChuYoko,
    /// The interior of a warichu (割注); carries no class of its own, unlike the cl-28 and
    /// cl-29 delimiters that bound it. JLReq: §3.4.2
    WarichuInterior,
    /// The interior of a furiwake (振分け). §3.1.10 item 12 makes "a unit of furiwake"
    /// one object, which is a same-run indivisibility like the others.
    /// JLReq: §3.7.2, §3.1.10
    Furiwake,
    /// A math or chemical formula set in running text or on a line of its own. Carries
    /// no class — cl-17 and cl-18 are the members' own classes — but §3.7.4 states four
    /// different spacings for the same class pair depending on this run's setting, so
    /// the run is what the override predicate asks about. JLReq: §3.7.4
    MathFormula(FormulaSetting),
}

/// §3.7.4 states its spacings twice, once for a formula in running text and once for a
/// formula set on a line of its own, and the two answers differ for the same class pair.
///
/// JLReq: §3.7.4
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormulaSetting {
    /// Set inside running text. JLReq: §3.7.4
    InLine,
    /// Set on a line of its own. JLReq: §3.7.4
    IndependentLine,
}

/// One construct run's identity within one stream.
///
/// Non-zero so that an overlay slot is one word: an absent construct and the first run
/// are different values without a discriminant beside them.
///
/// JLReq: §B.2#9–#11, §C.2#6–#8
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct RunId(NonZeroU16);

impl RunId {
    /// The run identified by `id`.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn new(id: NonZeroU16) -> Self {
        Self(id)
    }

    /// The identity, as the number it is.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn get(self) -> NonZeroU16 {
        self.0
    }
}

/// One group inside one run: §C.2 note 8's level below the run, and the only one.
///
/// JLReq: §C.2#8, §3.3.7
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct GroupId(NonZeroU16);

impl GroupId {
    /// The group identified by `id`.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn new(id: NonZeroU16) -> Self {
        Self(id)
    }

    /// The identity, as the number it is.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn get(self) -> NonZeroU16 {
        self.0
    }
}

/// Which declared construct one [`RunId`] came from.
///
/// `lower` allocates the identities, so the caller never saw them; this is the map back,
/// and it is the caller's own coordinates — the construct kind and the position in the
/// slice it passed. Every error and every placed annotation names a construct this way, so
/// a report reads "the ruby you passed third" rather than an ordinal the caller cannot
/// resolve (ADR-0015).
///
/// A caller that built [`Runs`] itself owns the identities already and does not need this.
///
/// JLReq: n/a (ADR-0015)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConstructRef {
    kind: ConstructKind,
    ordinal: u16,
}

impl ConstructRef {
    /// The `ordinal`-th construct of kind `kind` in the slice the caller passed.
    ///
    /// Public because `jlreq-inline` builds these and every error and placed annotation
    /// carries one, and neither is this crate: a seam type readable at one end and not
    /// writable at the other is a seam with nothing on the far end (ADR-0012).
    ///
    /// JLReq: n/a (ADR-0015)
    #[must_use]
    pub const fn new(kind: ConstructKind, ordinal: u16) -> Self {
        Self { kind, ordinal }
    }

    /// Which kind of construct the caller declared. JLReq: n/a (ADR-0015)
    #[must_use]
    pub const fn kind(self) -> ConstructKind {
        self.kind
    }

    /// The position in the slice the caller passed for that kind.
    ///
    /// JLReq: n/a (ADR-0015)
    #[must_use]
    pub const fn ordinal(self) -> u16 {
        self.ordinal
    }
}

/// Which construct run each item of one `Text` belongs to.
///
/// Run identity is here and **not** on [`crate::Item`], because an item is what the
/// caller measured and a run is what lowering computed; two carriers of one fact are two
/// things a caller can desynchronize (ADR-0015).
///
/// The constructor validates rather than trusts: every identity must name one contiguous
/// span, and no two kinds may share one. A caller with its own construct model can
/// therefore build this directly and skip `jlreq-inline` entirely — a real capability,
/// and the reason uniqueness is checked rather than promised by whoever allocated it.
///
/// JLReq: §B.2#9–#11, §C.2#6–#8, §C.2#13
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Runs<'a> {
    slots: &'a [Option<Construct>],
}

impl<'a> Runs<'a> {
    /// One slot per item of the text this overlays.
    ///
    /// The walk is one forward pass with two rescans of the prefix. Contiguity is
    /// rescanned once per construct block, so that half costs one pass over the items
    /// plus one per block; text with no constructs costs the pass alone. Group membership
    /// is rescanned once per item that carries a [`GroupId`], because a group may recur
    /// inside its own block — §C.2 note 8's jukugo-ruby (熟語ルビ) complex returns to a
    /// base — so its absence from earlier blocks has to be asked for each occurrence and
    /// cannot be answered once per block. That half is quadratic in a paragraph whose
    /// every base item carries a group, and it is stated rather than hidden: telling two
    /// occurrences of one group apart in one pass needs a set, and nothing at this depth
    /// may allocate to validate its input.
    ///
    /// JLReq: §B.2#9–#11, §C.2#6–#8, §C.2#13
    pub fn new(slots: &'a [Option<Construct>]) -> Result<Self, RunsError> {
        let mut previous: Option<Construct> = None;
        let mut block_start = 0_usize;

        for (index, slot) in slots.iter().enumerate() {
            let Some(here) = *slot else {
                previous = None;
                continue;
            };

            match previous {
                // The run continues from the item before it, so only the kind can be
                // wrong: §C.2 note 13 asks whether two items are in the same run, and a
                // run that is two constructs at once has no answer.
                Some(before) if before.run == here.run => {
                    if before.kind != here.kind {
                        return Err(RunsError::RunKindConflict {
                            run: here.run,
                            at: ordinal(index),
                        });
                    }
                },
                // A block opens here. The identity must be new, or it names two spans.
                _ => {
                    let earlier = slots.get(..index).unwrap_or_default();
                    if earlier
                        .iter()
                        .flatten()
                        .any(|before| before.run == here.run)
                    {
                        return Err(RunsError::RunNotContiguous {
                            run: here.run,
                            at: ordinal(index),
                        });
                    }
                    block_start = index;
                },
            }

            // A group is a level inside one run, so it may repeat freely within this
            // block and may not appear before it. The prefix outside the block is the
            // whole of what "another run" means once contiguity above has held.
            if let Some(group) = here.group {
                let outside = slots.get(..block_start).unwrap_or_default();
                if outside
                    .iter()
                    .flatten()
                    .any(|before| before.group == Some(group))
                {
                    return Err(RunsError::GroupCrossesRun {
                        group,
                        at: ordinal(index),
                    });
                }
            }

            previous = Some(here);
        }

        Ok(Self { slots })
    }

    /// Text with no constructs. Total, so every signature taking `Runs` has an answer
    /// for plain text and there is no second code path for it.
    ///
    /// JLReq: n/a (ADR-0015)
    #[must_use]
    pub const fn none() -> Self {
        Self { slots: &[] }
    }

    /// The construct one item belongs to, if any.
    ///
    /// `None` both for an item inside no construct and for an ordinal past the overlay,
    /// which is what makes [`Runs::none`] answer for every item of every text.
    ///
    /// JLReq: §B.2#9–#11
    #[must_use]
    pub fn of(self, item: ItemIndex) -> Option<Construct> {
        let index = usize::try_from(item.get()).ok()?;
        self.slots.get(index).copied().flatten()
    }

    /// How many items this overlay covers.
    ///
    /// JLReq: n/a (ADR-0015)
    #[must_use]
    pub fn len(self) -> usize {
        self.slots.len()
    }

    /// Whether the overlay covers no items at all, which is what [`Runs::none`] is.
    ///
    /// JLReq: n/a (ADR-0015)
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.slots.is_empty()
    }
}

/// The ordinal of a slot, for an error to name.
///
/// Saturates rather than wrapping: an overlay longer than [`u32::MAX`] items has slots no
/// [`ItemIndex`] can name, and a report about the last nameable one is better than a
/// report about the wrong one.
fn ordinal(index: usize) -> ItemIndex {
    ItemIndex::new(u32::try_from(index).unwrap_or(u32::MAX))
}

/// Why an overlay is not a valid statement about run identity.
///
/// JLReq: §B.2#9–#11, §C.2#6–#8, §C.2#13
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RunsError {
    /// One [`RunId`] appears at two non-adjacent positions.
    RunNotContiguous {
        /// The identity naming two spans.
        run: RunId,
        /// Where the second span opens.
        at: ItemIndex,
    },
    /// One [`RunId`] is used by two different [`ConstructKind`]s.
    RunKindConflict {
        /// The identity two kinds share.
        run: RunId,
        /// Where the second kind appears.
        at: ItemIndex,
    },
    /// A [`GroupId`] spans two runs.
    GroupCrossesRun {
        /// The group reaching outside its run.
        group: GroupId,
        /// Where it reappears.
        at: ItemIndex,
    },
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU16;

    use super::{
        Construct, ConstructKind, ConstructRef, FormulaSetting, GroupId, RunId, Runs, RunsError,
    };
    use crate::item::ItemIndex;

    /// The run identified by `id`, which is never zero in a test.
    fn run(id: u16) -> RunId {
        RunId::new(NonZeroU16::new(id).unwrap())
    }

    /// The group identified by `id`.
    fn group(id: u16) -> GroupId {
        GroupId::new(NonZeroU16::new(id).unwrap())
    }

    /// One ruby occurrence in run `id`, with no group.
    fn ruby(id: u16) -> Construct {
        Construct::new(ConstructKind::NonJukugoRuby, run(id), None)
    }

    #[test]
    fn a_contiguous_overlay_is_accepted() {
        let slots = [None, Some(ruby(1)), Some(ruby(1)), None, Some(ruby(2))];
        assert!(
            Runs::new(&slots).is_ok(),
            "two runs, each one span, is what the same-run predicate is asking about"
        );
    }

    #[test]
    fn a_run_reopening_after_a_gap_is_refused() {
        let slots = [Some(ruby(1)), None, Some(ruby(1))];
        assert_eq!(
            Runs::new(&slots).err(),
            Some(RunsError::RunNotContiguous {
                run: run(1),
                at: ItemIndex::new(2),
            }),
            "an identity naming two spans is an identity the caller can desynchronize"
        );
    }

    #[test]
    fn a_run_reopening_after_another_run_is_refused() {
        let slots = [Some(ruby(1)), Some(ruby(2)), Some(ruby(1))];
        assert_eq!(
            Runs::new(&slots).err(),
            Some(RunsError::RunNotContiguous {
                run: run(1),
                at: ItemIndex::new(2),
            }),
            "contiguity is broken by another run just as it is by a gap"
        );
    }

    #[test]
    fn a_run_used_by_two_kinds_is_refused() {
        let slots = [
            Some(ruby(1)),
            Some(Construct::new(ConstructKind::TateChuYoko, run(1), None)),
        ];
        assert_eq!(
            Runs::new(&slots).err(),
            Some(RunsError::RunKindConflict {
                run: run(1),
                at: ItemIndex::new(1),
            }),
            "a run that is two constructs at once has no answer for §C.2 note 13"
        );
    }

    #[test]
    fn a_group_spanning_two_runs_is_refused() {
        let slots = [
            Some(Construct::new(
                ConstructKind::JukugoRuby,
                run(1),
                Some(group(1)),
            )),
            Some(Construct::new(
                ConstructKind::JukugoRuby,
                run(2),
                Some(group(1)),
            )),
        ];
        assert_eq!(
            Runs::new(&slots).err(),
            Some(RunsError::GroupCrossesRun {
                group: group(1),
                at: ItemIndex::new(1),
            }),
            "§C.2 note 8's level sits inside a run and has no meaning across two"
        );
    }

    #[test]
    fn groups_repeating_inside_one_run_are_accepted() {
        let jukugo = |group_id| {
            Some(Construct::new(
                ConstructKind::JukugoRuby,
                run(1),
                Some(group(group_id)),
            ))
        };
        let slots = [jukugo(1), jukugo(2), jukugo(1)];
        assert!(
            Runs::new(&slots).is_ok(),
            "only crossing a run is refused; a jukugo complex may return to a base \
             character's group inside its own run"
        );
    }

    #[test]
    fn an_overlay_answers_for_the_item_it_covers() {
        let slots = [None, Some(ruby(1))];
        let runs = Runs::new(&slots).unwrap();
        assert_eq!(
            runs.of(ItemIndex::new(1)).map(Construct::run),
            Some(run(1)),
            "the overlay is read by the ordinal of the item it overlays"
        );
    }

    #[test]
    fn an_overlay_answers_none_for_an_item_in_no_construct() {
        let slots = [None, Some(ruby(1))];
        let runs = Runs::new(&slots).unwrap();
        assert!(
            runs.of(ItemIndex::new(0)).is_none(),
            "plain text inside a paragraph with constructs is still plain text"
        );
    }

    #[test]
    fn an_overlay_answers_none_past_its_end() {
        let slots = [Some(ruby(1))];
        let runs = Runs::new(&slots).unwrap();
        assert!(
            runs.of(ItemIndex::new(7)).is_none(),
            "an ordinal past the overlay is answered, not refused, so no call site branches"
        );
    }

    #[test]
    fn text_with_no_constructs_answers_none_everywhere() {
        let runs = Runs::none();
        assert!(
            runs.is_empty() && runs.of(ItemIndex::new(0)).is_none(),
            "the total value is what removes the second code path for plain text"
        );
    }

    #[test]
    fn an_overlay_covers_one_slot_per_item() {
        let slots = [None, Some(ruby(1)), Some(ruby(1))];
        assert_eq!(
            Runs::new(&slots).unwrap().len(),
            3,
            "the overlay is one slot per item of the text it overlays"
        );
    }

    #[test]
    fn a_construct_reference_is_the_callers_own_coordinates() {
        let reference = ConstructRef::new(
            ConstructKind::MathFormula(FormulaSetting::IndependentLine),
            2,
        );
        assert_eq!(
            (reference.kind(), reference.ordinal()),
            (
                ConstructKind::MathFormula(FormulaSetting::IndependentLine),
                2
            ),
            "a report reads as the position in the slice the caller passed"
        );
    }
}
