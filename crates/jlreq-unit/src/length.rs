// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The two length scalars, the exact ratio, the character size, and the carried
//! remainder.
//!
//! A quantity the writing system *states* is an [`Em`]; a quantity someone *measured* is
//! an [`Advance`]. They never mix, and [`Em::resolve_inline`] and [`Em::resolve_block`]
//! over one private computation are the only bridge (see `docs/adr/0007`).
//!
//! This module and [`crate::axis`] are the two places where a length is built from a
//! plain integer or read back as one. Everywhere else in the workspace that channel is
//! narrowed to a reviewed list (see `docs/adr/0011`). The closed arithmetic surface is
//! generated here rather than in [`crate::arith`] for that reason: the macro is written
//! once there and expanded at home, so its `Self(units)` never opens the channel in a
//! third module.

use core::num::NonZeroU16;

use crate::arith::closed_arithmetic;
use crate::axis::{BlockExtent, InlineExtent};

/// Units per ideographic em (全角, zenkaku).
///
/// `720 = 120 × 6`. 120 is the least common multiple of every fraction JLReq names —
/// halves, thirds, quarters, fifths, eighths. The factor of 6 is the least common
/// multiple of the two ruby scales JLReq names (§3.3.3: half the base size, and
/// one-third ruby 三分ルビ), so a quantity at either ruby scale is still exact when
/// stated in base ems, which is what lets a conformance case be read by a human.
///
/// Changing this is the workspace's one permanently breaking change (ADR-0007).
///
/// JLReq: §3.1.6, §B.1, §3.3.3, §3.8.3
pub const UNITS_PER_EM: i32 = 720;

/// The largest magnitude any length in the workspace may hold.
///
/// `2^30 - 1`. Two valid values sum to less than `i32::MAX`, so a single addition cannot
/// wrap and saturation can only report a breach of this bound (ADR-0007).
pub(crate) const UPPER: i32 = (1 << 30) - 1;

/// The mirror of [`UPPER`]. Both scalars are signed, because reduction deltas and
/// hanging punctuation are naturally signed (ADR-0007).
pub(crate) const LOWER: i32 = UPPER.saturating_neg();

/// The most em lengths one axis may carry a remainder for, as an array length.
const SIZE_SLOTS: usize = 32;

/// Clamp a raw count of units into the shared bound.
///
/// Saturation is the report, not a repair: a value outside `[LOWER, UPPER]` is a measure
/// beyond anything a page can hold, and the bound is what makes a single addition
/// unable to wrap (ADR-0007).
pub(crate) const fn bounded(units: i32) -> i32 {
    if units > UPPER {
        UPPER
    } else if units < LOWER {
        LOWER
    } else {
        units
    }
}

/// The greatest common divisor of `|a|` and `b`, for `b` greater than zero.
///
/// Total: `checked_rem_euclid` answers `None` only for a zero divisor, which the loop
/// condition already excludes, and the Euclidean sequence is strictly decreasing so the
/// loop terminates.
const fn greatest_common_divisor(a: i32, b: i32) -> i32 {
    let mut divisor = b;
    let mut rest = match a.checked_rem_euclid(b) {
        Some(rest) => rest,
        None => 0,
    };
    while rest != 0 {
        let next = match divisor.checked_rem_euclid(rest) {
            Some(next) => next,
            None => 0,
        };
        divisor = rest;
        rest = next;
    }
    divisor
}

/// `units × ratio`, exactly or not at all.
///
/// Reducing by the greatest common divisor first keeps the whole computation inside
/// `i32`: the reduced value and the reduced denominator are coprime, so the product is a
/// whole number exactly when the reduced denominator divides the numerator, and the only
/// multiplication left is the one whose result is being asked for.
///
/// `None` reports either that the ratio cannot be taken exactly or that the result
/// leaves the bound. Rounding a proportion quietly is the failure
/// [`crate::distribute`] and [`Em::resolve_inline`] exist to prevent, so it is not
/// offered here.
pub(crate) const fn scale_exact(units: i32, ratio: Ratio) -> Option<i32> {
    let denominator = ratio.denominator.get() as i32;
    let numerator = ratio.numerator as i32;
    let common = greatest_common_divisor(units, denominator);
    let (Some(reduced_units), Some(reduced_denominator)) =
        (units.checked_div(common), denominator.checked_div(common))
    else {
        return None;
    };
    let share = match numerator.checked_rem(reduced_denominator) {
        Some(0) => match numerator.checked_div(reduced_denominator) {
            Some(share) => share,
            None => return None,
        },
        _ => return None,
    };
    match reduced_units.checked_mul(share) {
        Some(product) if product <= UPPER && product >= LOWER => Some(product),
        _ => None,
    }
}

/// A quantity the writing system states, as a fraction of the ideographic em.
///
/// This is the unit of every table amount and every rule. It is *not* the unit of a
/// measured advance; see [`Advance`]. The two never mix, and [`Em::resolve_inline`] and
/// [`Em::resolve_block`] over one private computation are the only bridge.
///
/// JLReq: §B.1, ADR-0007
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Em(i32);

closed_arithmetic!(Em, "writing-system fraction");

impl Em {
    /// The largest magnitude an `Em` may hold: `2^30 - 1`, or `1_491_308` em.
    ///
    /// The bound is the overflow argument: two valid values sum to less than
    /// `i32::MAX`, so a single addition cannot wrap and saturation can only report a
    /// breach of this bound.
    ///
    /// JLReq: n/a (arithmetic)
    pub const LIMIT: i32 = UPPER;

    /// Solid setting (ベタ組, beta gumi): no space at all. JLReq: §B.1 blank cell
    pub const ZERO: Self = Self(0);
    /// One eighth em, the Japanese/Latin reduction floor. JLReq: §3.8.3 step 6
    pub const EIGHTH: Self = Self(90);
    /// One fifth em, the alternative word-space reduction floor. JLReq: §D preamble
    pub const FIFTH: Self = Self(144);
    /// A quarter em (四分アキ, shibu aki). JLReq: §B.1 `1/4`
    pub const QUARTER: Self = Self(180);
    /// A third em, the default Western word space. JLReq: §3.2.2
    pub const THIRD: Self = Self(240);
    /// A half em (二分アキ, nibu aki). JLReq: §B.1 `1/2`
    pub const HALF: Self = Self(360);
    /// One full ideographic em, the amount required after a dividing punctuation mark
    /// (cl-04) ending a sentence. Table 1's legend has no token for it.
    /// JLReq: §3.1.6
    pub const FULL: Self = Self(UNITS_PER_EM);

    /// An amount in units of 1/720 of the ideographic em, or `None` beyond [`Em::LIMIT`].
    ///
    /// JLReq: n/a (arithmetic)
    #[must_use]
    pub const fn from_units(units: i32) -> Option<Self> {
        if units > UPPER || units < LOWER {
            None
        } else {
            Some(Self(units))
        }
    }

    /// The amount, in units of 1/720 of the ideographic em.
    ///
    /// JLReq: n/a (arithmetic)
    #[must_use]
    pub const fn units(self) -> i32 {
        self.0
    }

    /// Build from a ratio, rejecting anything 1/720 cannot state exactly.
    ///
    /// JLReq: n/a (arithmetic)
    #[must_use]
    pub const fn ratio(ratio: Ratio) -> Option<Self> {
        match scale_exact(UNITS_PER_EM, ratio) {
            Some(units) => Some(Self(units)),
            None => None,
        }
    }

    /// Convert a writing-system fraction into an inline length in the caller's unit.
    ///
    /// Exact when 720 divides the size's inline em; otherwise the remainder is folded
    /// into the next call *at the same size*, so a run of resolutions at one size sums to
    /// the rounding of its exact total rather than to the sum of roundings. See [`Carry`]
    /// for why the qualification is load bearing.
    ///
    /// The remainder is not a parameter and there is no public type for one. [`Carry`]
    /// keys the remainder on the em length the resolution is against, which is the
    /// quantity the remainder is a remainder *of*, so spending at one size a remainder
    /// produced at another is not an expression that can be written (ADR-0007,
    /// ADR-0019).
    ///
    /// This and [`Em::resolve_block`] are the only places in the workspace where an
    /// `Em` becomes a length in the caller's unit, and they share one private
    /// computation. They are two functions rather than one with an axis argument so
    /// that no axis-free length is ever produced for a later call site to put on the
    /// wrong axis (ADR-0011).
    ///
    /// JLReq: §B.1, ADR-0007
    #[must_use]
    pub fn resolve_inline(self, size: Size, carry: &mut Carry) -> InlineExtent {
        InlineExtent::of(self.inline_units(size, carry))
    }

    /// The block-axis twin. §3.3.3 is the one rule that scales the two axes
    /// differently, which is why [`Scale`] is anisotropic and this is not the same
    /// function.
    ///
    /// JLReq: §3.3.3, §B.1, ADR-0007
    #[must_use]
    pub fn resolve_block(self, size: Size, carry: &mut Carry) -> BlockExtent {
        BlockExtent::of(self.block_units(size, carry))
    }

    /// The inline resolution as a bare count of the caller's units.
    ///
    /// Exists so [`crate::InlineCursor`] can accumulate without the `arith` module
    /// reaching for [`Advance::get`], which belongs to this module and to
    /// [`crate::axis`] (ADR-0011).
    pub(crate) const fn inline_units(self, size: Size, carry: &mut Carry) -> i32 {
        let em_length = size.scale.inline_em.0;
        match carry.inline_slot(em_length) {
            Some(slot) => resolve(self.0, em_length, slot),
            None => resolve(self.0, em_length, &mut 0),
        }
    }

    /// The block resolution as a bare count of the caller's units.
    pub(crate) const fn block_units(self, size: Size, carry: &mut Carry) -> i32 {
        let em_length = size.scale.block_em.0;
        match carry.block_slot(em_length) {
            Some(slot) => resolve(self.0, em_length, slot),
            None => resolve(self.0, em_length, &mut 0),
        }
    }
}

/// The one computation where an [`Em`] becomes a length in the caller's unit.
///
/// Answers `⌊(units · em_length + carried) / 720⌋` and leaves the new remainder in
/// `slot`, so a run of calls through one slot sums to the rounding of its exact total.
///
/// The whole computation stays in `i32`. Writing `em_length = 720·qs + rs` and
/// `units = 720·qu + ru` with Euclidean division gives
/// `units · em_length + carried = 720·(units·qs + qu·rs) + (ru·rs + carried)`, and the
/// trailing term is below `720·720 + 720`, which an `i32` holds with room to spare.
/// `qu·rs` is below `2^30/720 · 720`, so it cannot leave `i32` either. Only `units·qs`
/// can, and when it does the true quotient exceeds the bound as well — the gap between
/// `i32::MAX` and the largest `qu·rs` is wider than [`UPPER`] — so saturating there and
/// clamping at the end reports the breach rather than hiding a wrap (ADR-0007).
const fn resolve(units: i32, em_length: i32, slot: &mut i32) -> i32 {
    let whole_ems = em_length.div_euclid(UNITS_PER_EM);
    let em_fraction = em_length.rem_euclid(UNITS_PER_EM);
    let whole_units = units.div_euclid(UNITS_PER_EM);
    let unit_fraction = units.rem_euclid(UNITS_PER_EM);

    let residue = unit_fraction
        .saturating_mul(em_fraction)
        .saturating_add(*slot);
    let quotient = units
        .saturating_mul(whole_ems)
        .saturating_add(whole_units.saturating_mul(em_fraction))
        .saturating_add(residue.div_euclid(UNITS_PER_EM));

    *slot = residue.rem_euclid(UNITS_PER_EM);
    bounded(quotient)
}

/// A length in the caller's own unit.
///
/// kumihan adds, compares, and negates these; it never interprets one. Font units,
/// 1/64 px, points, and scaled points are equally valid: the unit is whatever the
/// caller's advances are already in, and returned positions are in the same unit.
///
/// This type is the unit of a [`Scale`] and the weight of [`crate::distribute`], and
/// appears nowhere else in the public surface. It is the weight there because §3.8.3
/// reduces spacing "in proportion to the character size", and a character size is a
/// length in the caller's own unit.
///
/// In particular there is no conversion between it and the four axis types: a pair of
/// them in either direction would let any value round-trip between the inline and block
/// axes in two well-typed steps, which no gate can see, so the pair does not exist
/// (ADR-0011). An axis type is built from and read as a plain integer in the caller's
/// unit.
///
/// JLReq: n/a (ADR-0002, ADR-0007)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Advance(i32);

closed_arithmetic!(Advance, "length in the caller's unit");

impl Advance {
    /// No length at all. JLReq: n/a (arithmetic)
    pub const ZERO: Self = Self(0);

    /// `2^30 - 1`. Shared by every length type in the workspace. A measure beyond this
    /// is refused rather than silently wrapped.
    ///
    /// JLReq: n/a (arithmetic)
    pub const LIMIT: i32 = UPPER;

    /// A length in the caller's unit, or `None` beyond [`Advance::LIMIT`].
    ///
    /// JLReq: n/a (arithmetic)
    #[must_use]
    pub const fn new(value: i32) -> Option<Self> {
        if value > UPPER || value < LOWER {
            None
        } else {
            Some(Self(value))
        }
    }

    /// The length, in the caller's unit.
    ///
    /// JLReq: n/a (arithmetic)
    #[must_use]
    pub const fn get(self) -> i32 {
        self.0
    }
}

/// `2`, as a denominator that cannot be zero. The `None` arm is unreachable for a
/// non-zero literal and is written out because the alternative is `unwrap`.
const TWO: NonZeroU16 = match NonZeroU16::new(2) {
    Some(two) => two,
    None => NonZeroU16::MIN,
};

/// `3`, as a denominator that cannot be zero.
const THREE: NonZeroU16 = match NonZeroU16::new(3) {
    Some(three) => three,
    None => NonZeroU16::MIN,
};

/// An exact ratio. Serves every rule that states a proportion rather than an amount: the
/// emphasis-dot size of §3.3.9, the group-ruby split of §3.3.6, the warichu (割注) size of
/// §3.4.2, and [`Em::scaled`].
///
/// It deliberately does *not* serve the ruby size. §3.3.3 leaves that open — for headings
/// at twelve points or more the ruby is "generally smaller than half" with no ratio at
/// all — and the caller has measured the reading anyway, so the ruby em is the annotation
/// stream's declared [`Scale`] (ADR-0019).
///
/// JLReq: §3.3.6, §3.3.9, §3.4.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Ratio {
    numerator: u16,
    denominator: NonZeroU16,
}

impl Ratio {
    /// One half: §3.3.3's principal ruby proportion and §3.3.9's emphasis-dot size.
    /// JLReq: §3.3.3, §3.3.9
    pub const HALF: Self = Self::new(1, TWO);

    /// One third: the inline extent of one-third ruby (三分ルビ). JLReq: §3.3.3
    pub const THIRD: Self = Self::new(1, THREE);

    /// A ratio of `numerator` to `denominator`.
    ///
    /// Not reduced: `2/4` and `1/2` are different values of this type and the same
    /// proportion, because a rule that says "twice in four parts" is quoted as it is
    /// written.
    ///
    /// JLReq: n/a (arithmetic)
    #[must_use]
    pub const fn new(numerator: u16, denominator: NonZeroU16) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    /// The numerator. JLReq: n/a (arithmetic)
    #[must_use]
    pub const fn numerator(self) -> u16 {
        self.numerator
    }

    /// The denominator, which cannot be zero. JLReq: n/a (arithmetic)
    #[must_use]
    pub const fn denominator(self) -> NonZeroU16 {
        self.denominator
    }
}

/// One character size, in the caller's unit.
///
/// Anisotropic on purpose: §3.3.3 gives one-third ruby (三分ルビ) a block extent of half
/// the base em and an inline extent of a third, so a single scalar per size cannot hold
/// it. For an ordinary square size use [`Scale::square`].
///
/// A paragraph declares one `Scale` per character size it contains — the base size, the
/// ruby size, the warichu (割注) size — and Appendix B's `be`/`af` referent selects which one a
/// fraction is a fraction of, so kumihan never computes "half of twelve points".
///
/// Both constructors refuse an em that is not strictly positive. [`Advance`] is signed
/// because a reduction delta and hanging punctuation (ぶら下げ, burasage) are naturally
/// signed, but a *size* is not a delta: §2.1.2's character size is a positive length, a
/// half em of a negative em is a negative advance that would flow into every extent on
/// the line, and the conformance case format already refuses a scale whose em is not
/// positive. An input the types accept and no rule permits is the dual of the
/// unconstructible input ADR-0012's gate exists for.
///
/// JLReq: §B.1, §2.1.2, §3.3.3, §3.4.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Scale {
    inline_em: Advance,
    block_em: Advance,
}

impl Scale {
    /// The ordinary case: one em, the same on both axes. `None` for an em that is not
    /// strictly positive.
    ///
    /// JLReq: §2.1.2
    #[must_use]
    pub const fn square(em: Advance) -> Option<Self> {
        Self::new(em, em)
    }

    /// A size whose two axes differ, which §3.3.3 needs and nothing else does. `None`
    /// when either em is not strictly positive.
    ///
    /// JLReq: §3.3.3
    #[must_use]
    pub const fn new(inline_em: Advance, block_em: Advance) -> Option<Self> {
        if inline_em.0 <= 0 || block_em.0 <= 0 {
            return None;
        }
        Some(Self {
            inline_em,
            block_em,
        })
    }

    /// The ideographic em along the inline axis. JLReq: §3.3.3
    #[must_use]
    pub const fn inline_em(self) -> Advance {
        self.inline_em
    }

    /// The ideographic em along the block axis. JLReq: §3.3.3
    #[must_use]
    pub const fn block_em(self) -> Advance {
        self.block_em
    }
}

/// Index of a [`Scale`] in a stream's scale table. `ScaleId::BASE` is the first
/// declared size; a caller with one size writes it explicitly rather than omitting it.
///
/// JLReq: §B.1, ADR-0007
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct ScaleId(u8);

impl ScaleId {
    /// The first declared size.
    ///
    /// JLReq: n/a (addressing)
    pub const BASE: Self = Self(0);

    /// The size declared at `index` in the stream's scale table.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn new(index: u8) -> Self {
        Self(index)
    }

    /// The ordinal in the stream's scale table.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn index(self) -> u8 {
        self.0
    }
}

/// One character size together with its ordinal in the stream that declared it.
///
/// The argument every resolution takes. A [`Scale`] alone says how big; a `Size` also
/// says *which* size, which is what a report, a trim record and a conformance case name
/// it by.
///
/// Ordinarily obtained from a stream — `Text::size_of`, `Text::size`, and their
/// annotation twins. [`Size::new`] is public because those accessors live in
/// `jlreq-class`, a separate crate, and a seam type readable at one end and not writable
/// at the other is a seam with nothing on the far end (ADR-0012).
///
/// The ordinal is therefore caller-supplied, and the per-size exactness claim of ADR-0007
/// does *not* rest on it: [`Carry`] keys the rounding remainder on the em length the
/// resolution is against, which is the quantity the remainder is a remainder of. Pairing
/// one ordinal with two different scales is expressible and harmless, because the ordinal
/// is not what the arithmetic reads.
///
/// JLReq: §B.1, §3.3.3, ADR-0007, ADR-0019
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Size {
    id: ScaleId,
    scale: Scale,
}

impl Size {
    /// Pair a scale with the ordinal the stream declaring it gave it.
    ///
    /// JLReq: §B.1, ADR-0007
    #[must_use]
    pub const fn new(id: ScaleId, scale: Scale) -> Self {
        Self { id, scale }
    }

    /// Which declared size this is. JLReq: §B.1
    #[must_use]
    pub const fn id(self) -> ScaleId {
        self.id
    }

    /// How big it is, on each axis. JLReq: §3.3.3
    #[must_use]
    pub const fn scale(self) -> Scale {
        self.scale
    }
}

/// One slot of a [`Carry`]: the em length a remainder was produced against, and the
/// remainder.
///
/// An `em` of zero is a slot no resolution has claimed. Zero cannot be a real key,
/// because [`Scale`] refuses an em that is not strictly positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Slot {
    /// The em length, in the caller's unit, this slot's remainder belongs to.
    em: i32,
    /// The remainder, in units of 1/720 of that em.
    carried: i32,
}

/// The slot a given em length's remainder lives in, claiming a free one if it has none.
///
/// `None` once every slot belongs to some other em length, which is the over-capacity
/// case [`Carry::SIZES`] bounds.
const fn slot_of(slots: &[Slot; SIZE_SLOTS], em: i32) -> Option<usize> {
    let mut index = 0;
    while index < SIZE_SLOTS {
        if slots[index].em == em {
            return Some(index);
        }
        index = index.saturating_add(1);
    }
    let mut free = 0;
    while free < SIZE_SLOTS {
        if slots[free].em == 0 {
            return Some(free);
        }
        free = free.saturating_add(1);
    }
    None
}

/// One carried rounding remainder per em length, on each axis.
///
/// A remainder produced against a 1000-unit em and spent against a 500-unit em is a
/// different absolute length, so a single carry across a line of mixed sizes would not
/// be exact — and a line of mixed sizes is the case [`Scale`] and Appendix B's `be`/`af`
/// referent both exist for. The carry is therefore keyed by the em length itself, and the
/// exactness claim is per em: a run of resolutions against one em sums to the rounding of
/// its exact total, and a line's total error is bounded by one unit per em present rather
/// than one per gap.
///
/// It keys on the em rather than on the [`ScaleId`] because the em is the quantity the
/// remainder is a remainder *of*, and the ordinal is only a proxy for it — one a caller
/// supplies, since [`Size::new`] is public and the accessors that ordinarily produce a
/// [`Size`] live in another crate. Keying on the proxy would let a misstated ordinal
/// spend one size's remainder against another's em, which is the substitution ADR-0007
/// forbids; keying on the fact makes that unrepresentable for any `Size` a caller can
/// build (ADR-0019). Two declared sizes that share an em length share a slot, and that is
/// correct rather than tolerated: the same absolute length is the same remainder.
///
/// It is also per axis, for the same argument one step further in. §3.3.3 gives one-third
/// ruby (三分ルビ) a block em of half the base and an inline em of a third, so one
/// [`Size`] names two different lengths; sharing one slot between them would spend on the
/// block axis a remainder produced against the inline em, and would do it across the axis
/// boundary ADR-0011 keeps closed.
///
/// There is no public remainder type and no way to obtain one. This is the only carrier
/// (ADR-0019), it is always taken as `&mut`, and every resolution reads and writes the
/// slot its em length and its axis name.
///
/// Fixed capacity, [`Carry::SIZES`] entries per axis, no allocation.
///
/// JLReq: §B.1, §3.3.3, ADR-0007
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Carry {
    inline: [Slot; SIZE_SLOTS],
    block: [Slot; SIZE_SLOTS],
}

impl Carry {
    /// The most em lengths one axis may carry a remainder for: 32. Well above the four
    /// character sizes the specification ever needs at once — base, ruby, warichu (割注),
    /// tate-chu-yoko (縦中横) — and bounded because this type keeps one remainder per em
    /// without allocating. `Text::new` validates a stream's scale table against it.
    ///
    /// A thirty-third em length on one axis has no slot of its own and resolves against a
    /// scratch remainder discarded on every call, so the exactness claim is about the
    /// first [`Carry::SIZES`] and no others. That is stated rather than hidden: the
    /// signature has no error channel, and evicting a live remainder to make room would
    /// lose it just as quietly.
    ///
    /// JLReq: n/a (ADR-0007)
    pub const SIZES: usize = SIZE_SLOTS;

    /// A carry with nothing carried yet.
    ///
    /// JLReq: n/a (ADR-0007)
    #[must_use]
    pub const fn new() -> Self {
        const EMPTY: Slot = Slot { em: 0, carried: 0 };
        Self {
            inline: [EMPTY; SIZE_SLOTS],
            block: [EMPTY; SIZE_SLOTS],
        }
    }

    /// The inline slot one em length's remainder lives in.
    const fn inline_slot(&mut self, em: i32) -> Option<&mut i32> {
        match slot_of(&self.inline, em) {
            Some(index) => {
                self.inline[index].em = em;
                Some(&mut self.inline[index].carried)
            },
            None => None,
        }
    }

    /// The block slot one em length's remainder lives in.
    const fn block_slot(&mut self, em: i32) -> Option<&mut i32> {
        match slot_of(&self.block, em) {
            Some(index) => {
                self.block[index].em = em;
                Some(&mut self.block[index].carried)
            },
            None => None,
        }
    }
}

impl Default for Carry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU16;

    use super::{
        Advance, Carry, Em, LOWER, Ratio, Scale, ScaleId, Size, Slot, THREE, TWO, UNITS_PER_EM,
        UPPER, greatest_common_divisor, resolve, scale_exact,
    };

    /// A square size at `em` caller units, declared first.
    fn base(em: i32) -> Size {
        sized(ScaleId::BASE, em)
    }

    /// A square size at `em` caller units, declared at `id`.
    fn sized(id: ScaleId, em: i32) -> Size {
        Size::new(
            id,
            Scale::square(Advance::new(em).unwrap()).expect("a character size is positive"),
        )
    }

    /// The denominator `value`, which is never zero in a test.
    fn over(value: u16) -> NonZeroU16 {
        NonZeroU16::new(value).unwrap()
    }

    #[test]
    fn the_unit_states_every_fraction_jlreq_names() {
        for divisor in [2, 3, 4, 5, 8] {
            assert_eq!(
                UNITS_PER_EM % divisor,
                0,
                "{divisor} is a denominator JLReq names, so one part in {divisor} of the \
                 ideographic em must be exact"
            );
        }
    }

    #[test]
    fn the_unit_states_a_quantity_at_either_ruby_scale_in_base_ems() {
        for ruby_scale in [2, 3] {
            for divisor in [2, 3, 4, 5, 8] {
                assert_eq!(
                    (UNITS_PER_EM / ruby_scale) % divisor,
                    0,
                    "one part in {divisor} of a ruby em one {ruby_scale}th the base size \
                     must still be exact when stated in base ems"
                );
            }
        }
    }

    #[test]
    fn the_named_amounts_are_the_fractions_they_are_named_for() {
        for (amount, divisor) in [
            (Em::EIGHTH, 8),
            (Em::FIFTH, 5),
            (Em::QUARTER, 4),
            (Em::THIRD, 3),
            (Em::HALF, 2),
            (Em::FULL, 1),
        ] {
            assert_eq!(
                amount.units() * divisor,
                UNITS_PER_EM,
                "the named amount must be exactly one part in {divisor} of the ideographic em"
            );
        }
    }

    #[test]
    fn an_amount_beyond_the_bound_has_no_value() {
        assert!(
            Em::from_units(UPPER.saturating_add(1)).is_none(),
            "the bound is what makes a single addition unable to wrap, so it is refused at entry"
        );
        assert!(
            Advance::new(LOWER.saturating_sub(1)).is_none(),
            "the bound is symmetric because both scalars are signed"
        );
    }

    #[test]
    fn an_amount_at_the_bound_has_a_value() {
        assert_eq!(
            Em::from_units(UPPER).map(Em::units),
            Some(UPPER),
            "the bound itself is a valid amount, not the first refused one"
        );
    }

    #[test]
    fn a_ratio_the_unit_cannot_state_exactly_is_refused() {
        assert!(
            Em::ratio(Ratio::new(1, over(7))).is_none(),
            "720 has no factor of seven, so a seventh of an em would have to be rounded"
        );
    }

    #[test]
    fn a_ratio_the_unit_states_exactly_becomes_that_amount() {
        assert_eq!(
            Em::ratio(Ratio::HALF),
            Some(Em::HALF),
            "a half stated as a ratio and a half stated as an amount are one value"
        );
    }

    #[test]
    fn a_ratio_is_kept_as_written_rather_than_reduced() {
        let two_quarters = Ratio::new(2, over(4));
        assert_ne!(
            two_quarters,
            Ratio::HALF,
            "a rule that says two parts in four is quoted as it is written"
        );
        assert_eq!(
            Em::ratio(two_quarters),
            Em::ratio(Ratio::HALF),
            "and the two still name the same proportion"
        );
    }

    #[test]
    fn a_ratio_reports_the_parts_it_was_built_with() {
        assert_eq!(
            (Ratio::THIRD.numerator(), Ratio::THIRD.denominator()),
            (1, THREE),
            "one third is one part in three"
        );
        assert_eq!(
            (Ratio::HALF.numerator(), Ratio::HALF.denominator()),
            (1, TWO),
            "one half is one part in two"
        );
    }

    #[test]
    fn the_greatest_common_divisor_of_zero_and_a_denominator_is_the_denominator() {
        assert_eq!(
            greatest_common_divisor(0, 5),
            5,
            "nothing scaled by anything is nothing, and the reduction must reach it without \
             dividing by zero"
        );
    }

    #[test]
    fn the_greatest_common_divisor_ignores_the_sign_of_the_amount() {
        assert_eq!(
            greatest_common_divisor(-12, 8),
            4,
            "a reduction delta is negative and reduces by the same factor a positive one does"
        );
    }

    #[test]
    fn scaling_is_refused_when_the_denominator_does_not_divide() {
        assert!(
            scale_exact(100, Ratio::THIRD).is_none(),
            "a third of 100 units is not a whole number of units and is not rounded here"
        );
    }

    #[test]
    fn scaling_is_exact_when_the_denominator_divides() {
        assert_eq!(
            scale_exact(Em::HALF.units(), Ratio::THIRD),
            Some(120),
            "a third of a half em is a sixth of an em, which the unit states exactly"
        );
    }

    #[test]
    fn scaling_reduces_before_it_multiplies_so_a_whole_ratio_is_not_an_overflow() {
        assert_eq!(
            scale_exact(UPPER, Ratio::new(65535, over(65535))),
            Some(UPPER),
            "the intermediate product would leave i32, so the ratio is reduced first"
        );
    }

    #[test]
    fn scaling_beyond_the_bound_is_refused() {
        assert!(
            scale_exact(UPPER.saturating_sub(1), Ratio::new(3, TWO)).is_none(),
            "three halves of the bound is past the bound, and past it there is no value"
        );
    }

    #[test]
    fn a_fraction_dividing_the_em_resolves_exactly() {
        let mut carry = Carry::new();
        assert_eq!(
            Em::HALF.resolve_inline(base(1000), &mut carry).units(),
            500,
            "half of a 1000-unit em is 500 units and needs no remainder"
        );
    }

    #[test]
    fn a_fraction_not_dividing_the_em_carries_its_remainder_into_the_next_call() {
        let mut carry = Carry::new();
        let size = base(1000);
        let thirds = [
            Em::THIRD.resolve_inline(size, &mut carry).units(),
            Em::THIRD.resolve_inline(size, &mut carry).units(),
            Em::THIRD.resolve_inline(size, &mut carry).units(),
        ];
        assert_eq!(
            thirds.iter().sum::<i32>(),
            1000,
            "three thirds of an em are one em, however the individual thirds round"
        );
    }

    #[test]
    fn a_run_of_resolutions_rounds_its_total_rather_than_its_parts() {
        let mut carry = Carry::new();
        let size = base(1000);
        let first = Em::THIRD.resolve_inline(size, &mut carry).units();
        let second = Em::THIRD.resolve_inline(size, &mut carry).units();
        let third = Em::THIRD.resolve_inline(size, &mut carry).units();
        assert_eq!(
            (first, second, third),
            (333, 333, 334),
            "the sum of roundings would give 333 three times and lose a unit; the rounding \
             of the sum gives the unit to the call that completes the em"
        );
    }

    #[test]
    fn a_remainder_produced_at_one_size_is_not_spent_at_another() {
        let text = base(1000);
        let ruby = sized(ScaleId::new(1), 500);
        let mut carry = Carry::new();

        let first = Em::THIRD.resolve_inline(text, &mut carry).units();
        let _at_the_ruby_size = Em::THIRD.resolve_inline(ruby, &mut carry);
        let second = Em::THIRD.resolve_inline(text, &mut carry).units();
        let third = Em::THIRD.resolve_inline(text, &mut carry).units();

        assert_eq!(
            first + second + third,
            1000,
            "a resolution against a 500-unit em must not consume a remainder produced \
             against a 1000-unit one"
        );
    }

    #[test]
    fn a_remainder_is_keyed_on_the_em_and_not_on_the_ordinal_a_caller_states() {
        // The same ordinal paired with two different scales, which `Size::new` makes
        // expressible. Keying on the ordinal would spend against a 500-unit em a
        // remainder produced against a 1000-unit one; keying on the em cannot.
        let text = base(1000);
        let misstated = sized(ScaleId::BASE, 500);
        let mut carry = Carry::new();

        let first = Em::THIRD.resolve_inline(text, &mut carry).units();
        let _at_the_other_size = Em::THIRD.resolve_inline(misstated, &mut carry);
        let second = Em::THIRD.resolve_inline(text, &mut carry).units();
        let third = Em::THIRD.resolve_inline(text, &mut carry).units();

        assert_eq!(
            first + second + third,
            1000,
            "the ordinal is a proxy the caller supplies; the em is the quantity the \
             remainder is a remainder of"
        );
    }

    #[test]
    fn two_declared_sizes_with_one_em_share_one_remainder() {
        // The dual of the test above: the same absolute length is the same remainder,
        // whatever ordinal the stream gave each of them.
        let text = base(1000);
        let same_em = sized(ScaleId::new(3), 1000);
        let mut carry = Carry::new();

        let thirds = [
            Em::THIRD.resolve_inline(text, &mut carry).units(),
            Em::THIRD.resolve_inline(same_em, &mut carry).units(),
            Em::THIRD.resolve_inline(text, &mut carry).units(),
        ];
        assert_eq!(
            thirds.iter().sum::<i32>(),
            1000,
            "three thirds of a 1000-unit em are 1000 units however the sizes are numbered"
        );
    }

    #[test]
    fn a_character_size_is_neither_zero_nor_negative() {
        assert!(
            Scale::square(Advance::ZERO).is_none(),
            "§2.1.2's character size is a length, and a half of nothing is not a size"
        );
        assert!(
            Scale::new(Advance::new(1000).unwrap(), Advance::new(-1000).unwrap()).is_none(),
            "an `Advance` is signed for reduction deltas; a size is not a delta"
        );
        assert!(
            Scale::square(Advance::new(1).unwrap()).is_some(),
            "one unit is a very small size and still a size"
        );
    }

    #[test]
    fn the_two_axes_of_one_size_carry_separately() {
        // A size whose inline em is 1000 caller units and whose block em is 500, which is
        // the shape §3.3.3 gives one-third ruby.
        let anisotropic = Size::new(
            ScaleId::BASE,
            Scale::new(Advance::new(1000).unwrap(), Advance::new(500).unwrap())
                .expect("both ems are positive"),
        );
        let mut carry = Carry::new();
        let mut inline_total = 0;
        let mut block_total = 0;

        for _ in 0..3 {
            inline_total += Em::THIRD.resolve_inline(anisotropic, &mut carry).units();
            block_total += Em::THIRD.resolve_block(anisotropic, &mut carry).units();
        }

        assert_eq!(
            (inline_total, block_total),
            (1000, 500),
            "one size names two em lengths, so a shared remainder would be spent against \
             the em it was not produced against"
        );
    }

    #[test]
    fn an_anisotropic_size_answers_differently_on_the_two_axes() {
        let ruby = Size::new(
            ScaleId::BASE,
            Scale::new(Advance::new(333).unwrap(), Advance::new(500).unwrap())
                .expect("both ems are positive"),
        );
        let mut carry = Carry::new();
        let inline = Em::FULL.resolve_inline(ruby, &mut carry).units();
        let block = Em::FULL.resolve_block(ruby, &mut carry).units();
        assert_ne!(
            inline, block,
            "§3.3.3 is the one rule that scales the two axes differently, so a single \
             scalar per size cannot hold one-third ruby"
        );
    }

    #[test]
    fn a_resolution_beyond_the_bound_saturates_rather_than_wrapping() {
        let mut carry = Carry::new();
        assert_eq!(
            Em::from_units(UPPER)
                .unwrap()
                .resolve_inline(base(UPPER), &mut carry)
                .units(),
            UPPER,
            "the product of two bounded values over 720 is still far past the bound, and \
             saturation reports that rather than hiding a machine wrap"
        );
    }

    #[test]
    fn a_negative_amount_resolves_negatively() {
        let mut carry = Carry::new();
        assert_eq!(
            Em::HALF
                .neg_sat()
                .resolve_inline(base(1000), &mut carry)
                .units(),
            -500,
            "reduction deltas are negative, which is why both scalars are signed"
        );
    }

    #[test]
    fn the_resolution_answers_what_the_wide_computation_would() {
        let interesting = [
            0,
            1,
            2,
            359,
            719,
            720,
            721,
            1000,
            12_345,
            -1,
            -719,
            -720,
            -721,
            -12_345,
            UPPER,
            LOWER,
            1_073_741_760,
        ];
        for units in interesting {
            for em_length in interesting {
                for carried in [0, 1, 359, 719] {
                    let mut slot = carried;
                    let answered = resolve(units, em_length, &mut slot);
                    let wide = i64::from(units) * i64::from(em_length) + i64::from(carried);
                    assert_eq!(
                        i64::from(answered),
                        wide.div_euclid(i64::from(UNITS_PER_EM))
                            .clamp(i64::from(LOWER), i64::from(UPPER)),
                        "resolving {units} against an em of {em_length} carrying {carried} \
                         must equal the wide quotient, clamped"
                    );
                    assert_eq!(
                        i64::from(slot),
                        wide.rem_euclid(i64::from(UNITS_PER_EM)),
                        "and must leave the wide remainder behind, saturated or not"
                    );
                }
            }
        }
    }

    #[test]
    fn an_em_beyond_the_carry_table_is_not_aliased_onto_a_slotted_one() {
        // Fill every inline slot with an em of its own, then resolve against one more.
        let mut carry = Carry::new();
        for slot in 0..Carry::SIZES {
            let em = 1000_i32.saturating_add(i32::try_from(slot).unwrap());
            let _claimed = Em::THIRD.resolve_inline(base(em), &mut carry);
        }
        let unslotted = base(7);
        let _outside_the_table = Em::THIRD.resolve_inline(unslotted, &mut carry);

        let first = Em::THIRD.resolve_inline(base(1000), &mut carry).units();
        let second = Em::THIRD.resolve_inline(base(1000), &mut carry).units();
        assert_eq!(
            first + second,
            667,
            "an em with no slot of its own must not borrow another em's; the first third \
             against this em left 240 carried, so the next two complete 667"
        );
    }

    #[test]
    fn a_square_size_has_one_em_on_both_axes() {
        let scale = Scale::square(Advance::new(1000).unwrap()).expect("a positive em");
        assert_eq!(
            scale.inline_em(),
            scale.block_em(),
            "an ordinary size is the same on both axes and says so once"
        );
    }

    #[test]
    fn a_size_remembers_which_declared_size_it_is() {
        assert_eq!(
            sized(ScaleId::new(2), 1000).id(),
            ScaleId::new(2),
            "a scale says how big; a size also says which, which is what a report names"
        );
    }

    #[test]
    fn the_carry_holds_one_slot_per_em_per_axis_without_allocating() {
        assert_eq!(
            size_of::<Carry>(),
            Carry::SIZES * 2 * size_of::<Slot>(),
            "a fixed array of one em and its remainder per axis, and nothing else"
        );
        assert_eq!(
            size_of::<Slot>(),
            2 * size_of::<i32>(),
            "a slot is the em it belongs to and the remainder, with no discriminant: an \
             em of zero is the free slot, because a scale refuses one"
        );
    }

    #[test]
    fn a_fresh_carry_carries_nothing() {
        assert_eq!(
            Carry::new(),
            Carry::default(),
            "the neutral value is the one the constructor names"
        );
    }
}
