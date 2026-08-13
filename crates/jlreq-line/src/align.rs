// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Single line alignment: [`Alignment`], [`align`].
//!
//! §3.8.1's own Note is why this is a separate module rather than a mode of
//! `crate::compose`: "There is another adjustment processing, besides line adjustment,
//! called single line alignment." Line adjustment (§3.8) fits a paragraph's own text to a
//! fixed measure by choosing where to break it; single line alignment fits one run —
//! shorter than a caller-stated target by construction, a heading or a line of a poem
//! rather than running prose — into that target. [`align`] never breaks: it calls
//! `geometry_of` once, over the whole of `text`, the same normalized-geometry pass
//! `compose` runs per candidate line, and never searches over more than the one line the
//! caller already decided this run is.
//!
//! # The four methods, one spacing computation
//!
//! [`Alignment::LineHead`], [`Alignment::LineEnd`] and [`Alignment::Centered`] differ only
//! in how far the line's own origin (`geometry_of`'s own `indent` parameter) is pushed in
//! before the *same* natural placements are laid out from it. Pushing every placement by
//! the same amount is exactly what `indent` already does — it is the same quantity
//! `crate::compose::Paragraph`'s own head indent uses to shift a first line — so each of
//! the three is one more call to `geometry_of`, not a second algorithm:
//!
//! - `LineHead` pushes by nothing: the natural pass, flush at the target's own start, is
//!   already this alignment. The residual between the natural extent and `target` is left
//!   as unconsumed trailing room rather than added anywhere, so `crate::Line::extent`
//!   reports the run's own natural extent, not `target` — see `# extent is the realized
//!   geometry, not a promise about target` below.
//! - `LineEnd` pushes by the *whole* residual, so the run's natural extent lands flush
//!   against `target`'s own far edge, and `crate::Line::extent` — the realized geometry —
//!   consequently reaches `target` too.
//! - `Centered` pushes by *half* the residual — as equal a half as
//!   [`jlreq_unit::distribute`] can make two sites, favoring the side `Policy::remainder`
//!   names for the one unit an odd residual leaves over — so the other half is left
//!   unconsumed past the run's own end, the same as `LineHead`'s whole residual, and for
//!   the same reason absent from `crate::Line::extent`.
//!
//! [`Alignment::EvenSpacing`] is §3.7.3's jidori (字取り) and cannot be reduced to a
//! single `indent`, because its residual is spread *between* items rather than in front of
//! all of them: each item's own cumulative shift differs from its neighbors'. It reuses the
//! same natural placements `geometry_of` already computed and the same
//! [`jlreq_unit::distribute`] primitive the other three call, over the interior boundaries
//! between items rather than over two fixed sites — still one spacing computation, applied
//! at more sites, not a second one.
//!
//! # §3.7.3's two stated exceptions
//!
//! "Spacing is not added where a break is prohibited." An interior boundary Table 2 (§C.1)
//! forbids a line from ending at is excluded from the weights [`jlreq_unit::distribute`]
//! splits the residual across, rather than merely given a weight of zero: a weight of zero
//! still receives its equal share of `distribute`'s own leveling and any leftover unit
//! (`distribute`'s own doc: weights that are all zero are all equal, so the split is
//! equal), so *excluding* the site is the only reading under which it is added none.
//! Eligibility here reads `jlreq_spacing::boundary`'s own `Breakable` answer alone, the
//! same table `Feasible::compute` reads for an ordinary interior break — not the fuller
//! kinsoku verdict `Feasible::compute` additionally applies. The two line-edge placement
//! checks are rightly absent from that narrower test: they guard the line *edge* a break
//! would create, and no interior jidori site ever creates one. `feasible::same_run_refusal`
//! is absent for an unrelated reason, not the line-edge argument above: it is private to
//! `crate::feasible` and this module never calls it, so `even_spacing_placements` (below)
//! reads `runs` only through `Adjacency::between` and `boundary(..).is_breakable()`, the
//! identical read an item in no construct at all gets. Whether that is the right answer is
//! unresolved rather than decided here: §3.7.3's own list of excluded positions is not
//! closed — "between grouped numerals (cl-24); between Western characters (cl-27); between
//! two inseparable characters (cl-08); *and so on*" (JLReq §3.7.3) — wording broad enough
//! that a same-run boundary of §C.2#6, #7, #8 or #13 plausibly belongs on it too, but no
//! rule address states that, and this eligibility test does not act as though it does: a
//! caller supplying a real overlay that declares two same-run occurrences of one of those
//! four constructs across an otherwise-eligible interior boundary gets a share of the
//! residual distributed there today. Recorded here as a scope limit rather than resolved
//! either way, because settling it needs a reading of "and so on" this pass does not
//! adjudicate. This narrower test is
//! consequently policy-dependent exactly as far as Table 2 itself is:
//! `Question::KINSOKU_LEVEL` changes which boundaries are eligible (`evaluate_breakable`
//! reads it directly), so the same heading run against the same target can distribute its
//! residual over a different number of sites at a stricter level. That is a stated
//! consequence of reading the level-selected table, not a second policy question this
//! module invents.
//!
//! "A single character is set at the inline start of the block." A run of fewer than two
//! items has no interior boundary at all, so [`jlreq_unit::distribute`] is called with no
//! weights, and its own documented behavior for that case — an empty iterator, because a
//! caller holding space and no site has nothing to place it at — answers this exception
//! without this module testing the item count itself. Such a run's *placements* are
//! consequently the natural, unshifted ones — the inline start — and so is its *extent*
//! (`crate::Line::extent` is the realized geometry, see `# extent is the realized geometry,
//! not a promise about target` below, and no residual was ever placed anywhere for it to
//! include): the item sits at the block's own start, exactly `LineHead`'s own case, which
//! is the exception as stated, not a second reading of it.
//!
//! The same "no eligible site" state is reached a second, more common way: where every
//! interior boundary of a run of two or more items happens to be kinsoku-prohibited,
//! `distribute` is called with an empty weight slice for the same reason as the
//! single-character case, and the whole run is placed unshifted — natural placements,
//! natural extent — for the same reason: not a special case this function tests for, but
//! the same fallback the item-count-under-two case already relies on, reached by a
//! different route.
//!
//! # `extent` is the realized geometry, not a promise about `target`
//!
//! `crate::Line::extent` is defined once, for every producer of a `Line`
//! (`docs/adr/0017-normalized-line-geometry.md`): from the line-head origin to the line
//! end, including the realized trailing space, and never past what is actually placed
//! there. `align` does not carve out a second meaning for its own four methods — each
//! reports whatever `crate::compose::geometry_of`'s cursor actually reaches once the
//! residual (or the share of it that method places) has been walked, the same as `compose`
//! does for its own lines.
//!
//! That is why `extent` reaches `target` for some methods and not others, without the
//! methods disagreeing about what `extent` means. `LineEnd` pushes the whole residual in
//! front of the run, so the cursor walks all the way to `target`. `EvenSpacing` does the
//! same whenever at least one interior boundary is eligible, because every eligible share
//! is consumed by the time the last item is placed. `LineHead` never pushes anything, and
//! `Centered` only pushes half, so for those the cursor stops short of `target` and
//! `extent` honestly says so; §3.7.3's own "no eligible site" exceptions land in the same
//! place as `LineHead`, for the same reason — nothing was ever pushed for `extent` to
//! include.
//!
//! # What this is not
//!
//! `Alignment::EvenSpacing` does not touch `crate::Ladder`: no reduction table, no
//! expansion table, no `jlreq_spacing::ExpansionStage` accounting, and the `crate::Line`
//! it returns still carries an empty `Adjustment` (`Line::from_geometry`'s own contract —
//! see `crate::compose`'s own `# Status`), because no ladder stage ever drained one. It is
//! §3.7.3's own even distribution against a caller-stated target, over the same
//! [`jlreq_unit::distribute`] primitive `crate::ladder`'s reduction and expansion drain
//! their own tables through (`crate::ladder`'s own `# Status`), not an early, partial
//! implementation of either.

use alloc::vec::Vec;

use jlreq_class::Text;
use jlreq_spacing::{Adjacency, boundary};
use jlreq_spec::Policy;
use jlreq_unit::{Advance, Direction, InlineExtent, InlineOffset, ItemIndex, Runs, distribute};

use crate::compose::{ComposeError, Edges, Geometry, Line, byte_range, geometry_of, shift_by};
use crate::ladder::Adjustment;
use crate::objective::Demerits;

/// Align a run shorter than the target length. Used for headings and poems.
///
/// All four methods share one spacing computation and differ only in where the residual
/// goes; only [`Alignment::EvenSpacing`] consumes the §3.8 expansion opportunities.
///
/// JLReq: §3.5.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Alignment {
    /// Center the run in `target`: the residual splits into two shares as equal as
    /// [`jlreq_unit::distribute`] can make them, one pushing the whole run in from the
    /// start, the other left unconsumed past the run's own end — so `Line::extent` reports
    /// only the pushed half, not `target` (see this module's own `# extent is the realized
    /// geometry, not a promise about target`).
    ///
    /// JLReq: §3.5.3
    Centered,
    /// Flush the run against the inline start of `target` — `geometry_of`'s own natural
    /// output, unshifted. The residual is left as unconsumed trailing room: `Line::extent`
    /// consequently reports the run's *natural* extent here, not `target` (see this
    /// module's own `# extent is the realized geometry, not a promise about target`).
    ///
    /// JLReq: §3.5.3
    LineHead,
    /// Flush the run against the inline end of `target`: the whole residual pushes the run
    /// in from the start, so `Line::extent` — the realized geometry — consequently reaches
    /// `target` here.
    ///
    /// JLReq: §3.5.3
    LineEnd,
    /// §3.7.3's jidori (字取り): spread the residual evenly across the boundaries between
    /// consecutive items, skipping a boundary a break is prohibited at.
    ///
    /// JLReq: §3.5.3, §3.7.3
    EvenSpacing,
}

/// A weight of one caller unit, for the equal-weighted [`distribute`] splits this module
/// makes: [`Alignment::Centered`]'s two sites and [`Alignment::EvenSpacing`]'s eligible
/// interior boundaries both want every site to carry the same share, which `distribute`
/// reads from equal weights rather than offering a dedicated "equal" mode (`distribute`'s
/// own doc: weights that are all zero are all equal, so the split is equal; equal nonzero
/// weights are the same case, since only the ratio between them is read).
///
/// Crate-visible: [`crate::tab`] halves a run's own natural extent for
/// `TabKind::Centered` the identical way, over the identical `distribute` primitive, and
/// calls this rather than declaring a second `one` (`docs/scalar-sites.toml`'s own entry
/// for this item already covers the one crossing this function makes; a second `one`
/// elsewhere in this crate would be attributed to that same entry by name alone, which is
/// the gate gotcha this project does not exploit).
///
/// A function rather than a `const` value: the `Advance::new` this needs to build one is a
/// reviewed crossing of the untyped channel (`docs/scalar-sites.toml`), and that gate
/// attributes a crossing to its enclosing item, which a bare `const` initializer has none
/// of ("crate scope"); wrapping it in a function gives the crossing a name to be reviewed
/// under, the same way `jlreq_spacing::raw`'s `em` does for its own narrower one.
///
/// The `None` arm is unreachable for the literal `1` and is written out because the
/// alternative is `unwrap` (`jlreq_unit::length`'s `TWO`/`THREE` are the same pattern over
/// a narrower type). If it were ever reached anyway, falling back to `Advance::ZERO` is
/// harmless rather than wrong, for the same reason: an all-zero weight slice is still an
/// equal split.
pub(crate) const fn one() -> Advance {
    match Advance::new(1) {
        Some(value) => value,
        None => Advance::ZERO,
    }
}

/// Align `text` as a single line against `target`.
///
/// The precondition the doc names — "a run shorter than the target length" — is read as a
/// degradation rather than an input error: a run whose natural extent already reaches or
/// exceeds `target` gets a residual of zero and every method's placements collapse to the
/// natural, unshifted ones (`Alignment::LineHead`'s own case exactly), because
/// [`ComposeError`] has only `OutOfRange` and `CandidateOutOfRange`, and neither names "the
/// run did not need aligning."
///
/// `Err(ComposeError::CandidateOutOfRange { .. })` is never returned: this function takes
/// no candidates, so there is nothing of that shape to be out of range.
/// `Err(ComposeError::OutOfRange { .. })` mirrors `compose`'s own defensive check over its
/// candidates, applied here to `text`'s own items; `Text::new` already validates every
/// item's placement in its stream (ADR-0018), so this is expected to never fire in
/// practice and is written anyway rather than assumed, the same discipline `compose`'s own
/// candidate loop applies.
///
/// JLReq: §3.5.3, §3.7.3
pub fn align(
    text: Text<'_>,
    runs: Runs<'_>,
    target: InlineExtent,
    alignment: Alignment,
    policy: Policy,
    direction: Direction,
) -> Result<Line, ComposeError> {
    let items = text.items();
    for (ordinal, item) in items.iter().enumerate() {
        if item.start().get() as usize > text.as_str().len() {
            let at = ItemIndex::new(u32::try_from(ordinal).unwrap_or(u32::MAX));
            return Err(ComposeError::OutOfRange { at });
        }
    }

    let item_count = u32::try_from(items.len()).unwrap_or(u32::MAX);
    let start = ItemIndex::new(0);
    let end = ItemIndex::new(item_count);

    let natural = geometry_of(
        text,
        runs,
        start..end,
        InlineExtent::ZERO,
        direction,
        policy,
        Edges::BOTH,
    );
    let residual = target.sub_sat(natural.extent).max(InlineExtent::ZERO);

    let geometry = match alignment {
        Alignment::LineHead => natural,
        Alignment::Centered => {
            let two_equal_sites = [one(), one()];
            let mut shares = distribute(residual, &two_equal_sites, policy.remainder());
            let leading = shares.next().unwrap_or(InlineExtent::ZERO);
            let shifted = geometry_of(
                text,
                runs,
                start..end,
                leading,
                direction,
                policy,
                Edges::BOTH,
            );
            Geometry {
                placements: shifted.placements,
                extent: shifted.extent,
                trailing: natural.trailing,
                trims: natural.trims,
                sites: Vec::new(),
            }
        },
        Alignment::LineEnd => {
            let shifted = geometry_of(
                text,
                runs,
                start..end,
                residual,
                direction,
                policy,
                Edges::BOTH,
            );
            Geometry {
                placements: shifted.placements,
                extent: shifted.extent,
                trailing: natural.trailing,
                trims: natural.trims,
                sites: Vec::new(),
            }
        },
        Alignment::EvenSpacing => {
            let (placements, applied) = even_spacing_placements(
                text,
                runs,
                direction,
                policy,
                &natural.placements,
                residual,
            );
            Geometry {
                placements,
                extent: natural.extent.add_sat(applied),
                trailing: natural.trailing,
                trims: natural.trims,
                sites: Vec::new(),
            }
        },
    };

    Ok(Line::from_geometry(
        start..end,
        byte_range(text, start, end),
        geometry,
        Demerits::ZERO,
        true,
        Adjustment::empty(),
        None,
    ))
}

/// §3.7.3's interior redistribution: the placements [`Alignment::EvenSpacing`] returns,
/// built from the natural (unaligned) placements `geometry_of` already computed and the
/// per-boundary shares [`distribute`] gives `residual` across every *eligible* interior
/// boundary — paired with the total share actually applied, which the caller needs to
/// report `crate::Line::extent` honestly rather than restate `residual`
/// (`docs/adr/0017-normalized-line-geometry.md`: extent is the realized geometry, from the
/// line-head origin to the line end, not a claim about what the caller asked for). The
/// total returned is `residual` itself whenever at least one boundary is eligible
/// ([`distribute`]'s own exactness guarantee once its weight slice is nonempty), and zero
/// when none is — the same state [`Alignment::LineHead`] is in, reached here rather than
/// tested for directly.
///
/// An interior boundary is eligible when Table 2 (§C.1) permits a line to end there —
/// `jlreq_spacing::boundary(..).is_breakable()`, the same lookup `Feasible::compute` uses
/// for an ordinary candidate — read directly rather than through `Feasible`, because
/// `Feasible::compute` answers a question about the caller's own candidates and this
/// module has none: there is no [`crate::Candidate`] at an interior jidori site, so nothing
/// here is exempted, refused, or rejected the way a break candidate is, only weighed in or
/// out of the distribution.
fn even_spacing_placements(
    text: Text<'_>,
    runs: Runs<'_>,
    direction: Direction,
    policy: Policy,
    natural: &[InlineOffset],
    residual: InlineExtent,
) -> (Vec<InlineOffset>, InlineExtent) {
    let boundary_count = natural.len().saturating_sub(1);
    let mut eligible = Vec::with_capacity(boundary_count);
    for ordinal in 0..boundary_count {
        let before = ItemIndex::new(u32::try_from(ordinal).unwrap_or(u32::MAX));
        // `None` only when `before` names no item or names the text's last item, neither of
        // which this range reaches (`before` is always strictly before `natural`'s last
        // index): the fallback of `false` — not eligible — is defensive and, if ever
        // reached, the conservative reading of "spacing is not added where a break is
        // prohibited".
        let allowed = Adjacency::between(text, runs, before, direction)
            .is_some_and(|adjacency| boundary(adjacency, policy).is_breakable());
        eligible.push(allowed);
    }

    let weight_count = eligible.iter().filter(|&&allowed| allowed).count();
    let weights: Vec<Advance> = core::iter::repeat_n(one(), weight_count).collect();
    let mut shares = distribute(residual, &weights, policy.remainder());

    let mut placements = Vec::with_capacity(natural.len());
    let mut cumulative = InlineExtent::ZERO;
    for (ordinal, &offset) in natural.iter().enumerate() {
        placements.push(shift_by(offset, cumulative));
        // Boundary `ordinal` sits between item `ordinal` and item `ordinal + 1`; its share,
        // once crossed, is carried by every later item, which is why `cumulative` is
        // updated only *after* this item's own placement was pushed.
        if eligible.get(ordinal).copied().unwrap_or(false) {
            cumulative = cumulative.add_sat(shares.next().unwrap_or(InlineExtent::ZERO));
        }
    }
    (placements, cumulative)
}
