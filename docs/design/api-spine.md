# The API spine

This is the contract. Every public type and function in the workspace appears here, in
dependency order, with the doc comment it will carry. Two implementers working from this
document independently should produce compatible code.

The **shape is frozen now**, at M0, before any layout code exists. The milestone markers
say when an item is *filled in*, not when it is *designed*: an item marked M4 has its
signature settled today and its body written later, because
[ADR 0012](../adr/0012-outcome-and-detail-compatibility.md) makes adding a filled-in item a
minor release only if the surrounding shape was right from the start.

Every signature elides `#[must_use]`, `#[derive(Debug)]`, and `#[non_exhaustive]` where the
rules in [ADR 0012](../adr/0012-outcome-and-detail-compatibility.md) make them mandatory.
They are mandatory; they are omitted here for readability and are not optional.

Class identifiers are written `cl-01` through `cl-30`, zero-padded, because that is what
JLReq writes — 302 occurrences of `cl-01` and none of `cl-1`. The repository's own prose
currently uses `cl-1` in four places; see [What must change elsewhere](#what-must-change-elsewhere).

### `Copy`, and by value or by reference

This is settled once, here, because it is mechanical rather than stylistic and because two
implementers left to decide it will decide it oppositely. `clippy.toml` sets
`trivial-copy-size-limit = 128` and `pass-by-value-size-limit = 256`, the workspace runs
`clippy::pedantic` with `RUSTFLAGS="-D warnings"`, and
[CONTRIBUTING.md](../../CONTRIBUTING.md) forbids `#[allow]`. A `&T` for a `Copy` `T` of 128
bytes or less is therefore a build error, not a preference.

**Every type built only from integers and shared references is `Copy` and is passed by
value.** That is `Em`, `Advance`, `Ratio`, `Scale`, `ScaleId`, `Size`, the four axis types,
`Direction`, `Side`, `InlineEdge`, `Frame`, `Role`, `Item`, `ByteOffset`, `ItemIndex`,
`AnnotationIndex`, `Construct`, `ConstructKind`, `ConstructRef`, `RunId`, `GroupId`,
`Runs<'a>`, `Segment<'a>`, `Interior<'a>`, `Straddle`, `Separation`, `BlockDemand`,
`InlineCursor`,
`RubyOverhang`, `RuleId`, `Address`, `Standing`, `Answer<T>`, `Provenance`, `Question`,
`Choice`, `Policy` (about forty bytes), `Text<'t>` and `Annotation<'a>` (three fat pointers
each), `Class`, `ClassSet`, `Member`, `Subject`, `Classified`, `Adjacency<'r>`,
`ConditionalSpace`, `Referent`, `Reduction`, `Expansion`, `Boundary`, `Breakable`,
`Placement`, `Candidate`, `CandidateIndex`, `Badness`, `Demerits`, `Preference`, `Fit`,
`Deepest`, `Trim`, `Part`, and `Paragraph<'r>`. Their methods take `self`, not `&self`.

**Everything else is passed by reference**, for one of two reasons. `Feasible<'r>`,
`Ladder`, `Adjustment`, `Composition`, `Line`, `Contribution<'a>`, `Lowered` and `Suite` own
an allocation and are not `Copy` at all. `Constructs<'c>` and `Ruby<'r>` are built only from
shared references but run well past 128 bytes, so a reference is what
`trivially_copy_pass_by_ref` permits and `pass_by_value` prefers. `Carry` is two arrays of
`Carry::SIZES` slots, one per axis, each slot an em length and its remainder — 512 bytes —
and is always taken as `&mut`, which the lint does not see. That is also why `InlineCursor`
does not own one: a 12-byte cursor is `Copy` and passed by value, and a cursor carrying 512
bytes of remainder would be neither that nor a single carrier of the remainder.

Two consequences worth naming. `Policy` is a value, so `policy: Policy` appears in about a
dozen signatures where a reference would read more naturally and would not compile. And a
`Text` is a value too, which is why every function that takes an ordinal takes the stream
beside it at no cost.

## Contents

- [The crate graph](#the-crate-graph)
- [`jlreq-unit`](#jlreq-unit)
- [`jlreq-spec`](#jlreq-spec)
- [`jlreq-class`](#jlreq-class)
- [`jlreq-spacing`](#jlreq-spacing)
- [`jlreq-line`](#jlreq-line)
- [`jlreq-inline`](#jlreq-inline)
- [`jlreq`](#jlreq)
- [`jlreq-conform`](#jlreq-conform)
- [The frozen API file](#the-frozen-api-file)
- [Mechanical gates](#mechanical-gates)
- [What must change elsewhere](#what-must-change-elsewhere)

## The crate graph

Two crates are added below the existing six, and `jlreq-line` and `jlreq-inline` stay
siblings ([ADR 0015](../adr/0015-the-crate-graph-and-the-inline-line-seam.md)).

| Crate | Depends on | Allocates | `no_std` |
| --- | --- | --- | --- |
| `jlreq-unit` | — | no | yes |
| `jlreq-spec` | `jlreq-unit` | no | yes |
| `jlreq-class` | `jlreq-unit`, `jlreq-spec` | no | yes |
| `jlreq-spacing` | `jlreq-unit`, `jlreq-spec`, `jlreq-class` | no | yes |
| `jlreq-line` | `jlreq-unit`, `jlreq-spec`, `jlreq-class`, `jlreq-spacing` | yes | yes |
| `jlreq-inline` | `jlreq-unit`, `jlreq-spec`, `jlreq-class` | yes | yes |
| `jlreq` | all of the above | yes | yes |
| `jlreq-conform` | `jlreq`, `jlreq-spec` | yes | no |

No crate anywhere in this table has an outside dependency, including `jlreq-conform`; see
[`jlreq-conform`](#jlreq-conform) for why the suite reads its own JSON.

`jlreq-unit` holds quantities, axes, and the item vocabulary. `jlreq-spec` holds the
specification-reference vocabulary — rule addresses, provenance, and the policy space — and
nothing else. They are separate because they are unrelated concepts, and because
`jlreq-conform` must reach the rule inventory to report coverage without pulling in the
whole facade. Merging them would produce a crate named after lengths that contains several
thousand generated JLReq headings.

`jlreq-spec` depends on `jlreq-unit` for one function and one type
([ADR 0020](../adr/0020-the-seam-carries-no-rule-address.md)). `Policy::remainder` is the
single derivation of a `RemainderRule` from a policy, which is what makes `distribute`'s
argument a transport rather than a second carrier
([ADR 0019](../adr/0019-one-fact-one-carrier.md)), and a `RemainderRule` is a quantity. The
edge runs this way and not the other because ADR 0019 states that `jlreq-unit` does not
depend on `jlreq-spec`, so `Segment` and `Separation` carry no `RuleId` and provenance
travels beside them in an `Answer<T>`. `jlreq-unit` therefore still depends on nothing, and
the manifest declares the edge in the commit that lands `Policy::remainder` with the
generated policy space, because a dependency nothing uses fails `cargo shear`.

`jlreq-class` needs both: `Em` because §3.1.6 makes an intrinsic advance a per-member
property (cl-03 alone carries a quarter em, a half em, and a full em across its four
members), and the policy space because §C.2 notes 1 through 3 make a relaxation a
*reclassification*, which must reach the classifier rather than the line breaker — a
relaxed `々` behaves as cl-19 against all six matrices, not only at the line head.

`jlreq-class` also owns [`Text`](#the-two-streams--m0) and `Annotation`, which is where the
crate table differs from the previous revision. `jlreq-unit` holds the item *vocabulary* —
`Item`, `ItemIndex`, `ByteOffset`, `Frame`, `Role` — but a stream's validity is a statement
about Appendix A: one item is one Appendix A key, and the frame is required on the five
classes of §3.1.2 ([ADR 0018](../adr/0018-an-item-is-one-occurrence.md)). A constructor that
cannot read the table it must check against is a constructor that documents its invariant
instead of holding it. Every crate that names a `Text` already depends on `jlreq-class`, so
this costs no edge and adds no crate.

`jlreq-inline` does **not** depend on `jlreq-line`, and the reason is §3.4.3. A warichu
(割注) that does not fit in what is left of the current line wraps onto the following one,
and the Japanese note calls two-line straddling 頻出 — frequent. Its interior's available
measure is therefore a sequence, not a number, and is unknown until the outer break is
chosen, so a construct layer holding neither the measure nor the outer search cannot compose
it. Every break selection in the workspace consequently happens in `jlreq-line`, over one
feasibility computation and one ladder, and the construct layer hands it a
[`Segment`](#the-seam--m0) instead.

There is equally **no** edge back from `jlreq-line` to `jlreq-inline`. Ruby overhang depends
on the space that survives line adjustment (§3.3.8 rule 3, and Appendix B's `hang` legend),
so `jlreq-line` resolves the overhang *allowance* and reports it per boundary;
`jlreq-inline` places annotations against an allowance it is told. Everything crossing the
seam — run identity, segments, separations, block demand, extents, the allowance — lives in
`jlreq-unit`, so neither crate names a type owned by the other, and `jlreq` orders the two:
`lower`, then `compose`, then `place`.

## `jlreq-unit`

Quantities, axes, and the item vocabulary. No specification knowledge, no tables, no state.

```text
src/
  lib.rs        re-exports; crate docs
  length.rs     Em, Advance, Scale, ScaleId, Size, Carry, Ratio, UNITS_PER_EM
  arith.rs      inherent arithmetic, InlineCursor, distribute
  axis.rs       InlineOffset, BlockOffset, InlineExtent, BlockExtent, InlineEdge, Side,
                Direction
  item.rs       Frame, Role, Item, ByteOffset, ItemIndex
  run.rs        Construct, ConstructKind, ConstructRef, RunId, GroupId, Runs
  seam.rs       Segment, Interior, Straddle, Separation, BlockDemand, RubyOverhang
```

### Lengths — M0

```rust
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

/// A quantity the writing system states, as a fraction of the ideographic em.
///
/// This is the unit of every table amount and every rule. It is *not* the unit of a
/// measured advance; see [`Advance`]. The two never mix, and [`Em::resolve_inline`] and
/// [`Em::resolve_block`] over one private computation are the only bridge.
///
/// JLReq: §B.1, ADR-0007
pub struct Em(i32);

impl Em {
    /// The largest magnitude an `Em` may hold: `2^30 - 1`, or 1_491_308 em.
    ///
    /// The bound is the overflow argument: two valid values sum to less than
    /// `i32::MAX`, so a single addition cannot wrap and saturation can only report a
    /// breach of this bound.
    pub const LIMIT: i32 = (1 << 30) - 1;

    /// Solid setting (ベタ組, beta gumi): no space at all. JLReq: §B.1 blank cell
    pub const ZERO: Self;
    /// One eighth em, the Japanese/Latin reduction floor. JLReq: §3.8.3 step 6
    pub const EIGHTH: Self;
    /// One fifth em, the alternative word-space reduction floor. JLReq: §D preamble
    pub const FIFTH: Self;
    /// A quarter em (四分アキ, shibu aki). JLReq: §B.1 `1/4`
    pub const QUARTER: Self;
    /// A third em, the default Western word space. JLReq: §3.2.2
    pub const THIRD: Self;
    /// A half em (二分アキ, nibu aki). JLReq: §B.1 `1/2`
    pub const HALF: Self;
    /// One full ideographic em, the amount required after a dividing punctuation mark
    /// (cl-04) ending a sentence. Table 1's legend has no token for it.
    /// JLReq: §3.1.6
    pub const FULL: Self;

    /// JLReq: n/a (arithmetic)
    pub const fn from_units(units: i32) -> Option<Self>;
    /// JLReq: n/a (arithmetic)
    pub const fn units(self) -> i32;
    /// Build from a ratio, rejecting anything 1/720 cannot state exactly.
    /// JLReq: n/a (arithmetic)
    pub const fn ratio(ratio: Ratio) -> Option<Self>;

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
    pub fn resolve_inline(self, size: Size, carry: &mut Carry) -> InlineExtent;
    /// The block-axis twin. §3.3.3 is the one rule that scales the two axes
    /// differently, which is why [`Scale`] is anisotropic and this is not the same
    /// function.
    ///
    /// JLReq: §3.3.3, §B.1, ADR-0007
    pub fn resolve_block(self, size: Size, carry: &mut Carry) -> BlockExtent;
}

/// A length in the caller's own unit.
///
/// kumihan adds, compares, and negates these; it never interprets one. Font units,
/// 1/64 px, points, and scaled points are equally valid: the unit is whatever the
/// caller's advances are already in, and returned positions are in the same unit.
///
/// This type is the unit of a [`Scale`] and the weight of [`distribute`], and appears
/// nowhere else in the public surface. It is the weight there because §3.8.3 reduces
/// spacing "in proportion to the character size", and a character size is a length in the
/// caller's own unit.
///
/// In particular there is no conversion between it and the four axis types: a pair of
/// them in either direction would let any value round-trip between the inline and block
/// axes in two well-typed steps, which no gate can see, so the pair does not exist
/// (ADR-0011). An axis type is built from and read as a plain integer in the caller's
/// unit.
///
/// JLReq: n/a (ADR-0002, ADR-0007)
pub struct Advance(i32);

impl Advance {
    pub const ZERO: Self;
    /// `2^30 - 1`. Shared by every length type in the workspace. A measure beyond this
    /// is refused rather than silently wrapped.
    pub const LIMIT: i32 = (1 << 30) - 1;
    /// JLReq: n/a (arithmetic)
    pub const fn new(value: i32) -> Option<Self>;
    /// JLReq: n/a (arithmetic)
    pub const fn get(self) -> i32;
}

/// An exact ratio. Serves every rule that states a proportion rather than an amount: the
/// emphasis-dot size of §3.3.9, the group-ruby split of §3.3.6, the warichu size of
/// §3.4.2, and [`Em::scaled`].
///
/// It deliberately does *not* serve the ruby size. §3.3.3 leaves that open — for headings
/// at twelve points or more the ruby is "generally smaller than half" with no ratio at
/// all — and the caller has measured the reading anyway, so the ruby em is the annotation
/// stream's declared [`Scale`] (ADR-0019).
///
/// JLReq: §3.3.6, §3.3.9, §3.4.2
pub struct Ratio { /* numerator: u16, denominator: NonZeroU16 */ }

impl Ratio {
    pub const HALF: Self;
    pub const THIRD: Self;
    pub const fn new(numerator: u16, denominator: NonZeroU16) -> Self;
    pub const fn numerator(self) -> u16;
    pub const fn denominator(self) -> NonZeroU16;
}

/// One character size, in the caller's unit.
///
/// Anisotropic on purpose: §3.3.3 gives one-third ruby (三分ルビ) a block extent of half
/// the base em and an inline extent of a third, so a single scalar per size cannot hold
/// it. For an ordinary square size use [`Scale::square`].
///
/// A paragraph declares one `Scale` per character size it contains — the base size, the
/// ruby size, the warichu (割注) size — and Appendix B's `be`/`af` referent selects which
/// one a fraction is a fraction of, so kumihan never computes "half of twelve points".
///
/// Both constructors refuse an em that is not strictly positive. [`Advance`] is signed
/// because a reduction delta and hanging punctuation are naturally signed, but a *size* is
/// not a delta: §2.1.2's character size is a positive length, a half em of a negative em is
/// a negative advance that flows into every extent on the line, and the conformance case
/// format already refuses a scale whose em is not positive. An input the types accept and
/// no rule permits is the dual of the unconstructible input ADR-0012's gate exists for.
///
/// JLReq: §B.1, §2.1.2, §3.3.3, §3.4.2
pub struct Scale { /* inline_em: Advance, block_em: Advance */ }

impl Scale {
    /// The ordinary case: one em, the same on both axes. `None` for an em that is not
    /// strictly positive.
    /// JLReq: §2.1.2
    pub const fn square(em: Advance) -> Option<Self>;
    /// A size whose two axes differ, which §3.3.3 needs and nothing else does. `None`
    /// when either em is not strictly positive.
    /// JLReq: §3.3.3
    pub const fn new(inline_em: Advance, block_em: Advance) -> Option<Self>;
    pub const fn inline_em(self) -> Advance;
    pub const fn block_em(self) -> Advance;
}

/// Index of a [`Scale`] in a stream's scale table. `ScaleId::BASE` is the first
/// declared size; a caller with one size writes it explicitly rather than omitting it.
pub struct ScaleId(u8);

impl ScaleId {
    /// The first declared size.
    pub const BASE: Self;
    pub const fn new(index: u8) -> Self;
    pub const fn index(self) -> u8;
}

/// One character size together with its ordinal in the stream that declared it.
///
/// The argument every resolution takes. A [`Scale`] alone says how big; a `Size` also says
/// *which* size, which is what a report, a trim record and a conformance case name it by.
///
/// Ordinarily obtained from a stream — `Text::size_of`, `Text::size`, and their annotation
/// twins. `Size::new` is public because those accessors live in `jlreq-class`, a separate
/// crate, and a seam type readable at one end and not writable at the other is a seam with
/// nothing on the far end (ADR-0012).
///
/// The ordinal is therefore caller-supplied, and the per-size exactness claim of ADR-0007
/// does *not* rest on it: [`Carry`] keys the rounding remainder on the em length the
/// resolution is against, which is the quantity the remainder is a remainder of. Pairing
/// one ordinal with two different scales is expressible and harmless, because the ordinal
/// is not what the arithmetic reads.
///
/// JLReq: §B.1, §3.3.3, ADR-0007, ADR-0019
pub struct Size { /* id: ScaleId, scale: Scale */ }

impl Size {
    /// Pair a scale with the ordinal the stream declaring it gave it.
    pub const fn new(id: ScaleId, scale: Scale) -> Self;
    pub const fn id(self) -> ScaleId;
    pub const fn scale(self) -> Scale;
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
/// supplies, since `Size::new` is public. Keying on the proxy would let a misstated
/// ordinal spend one size's remainder against another's em; keying on the fact makes that
/// unrepresentable for any `Size` a caller can build (ADR-0019). Two declared sizes that
/// share an em length share a slot, and that is correct rather than tolerated.
///
/// It is also per axis. §3.3.3 gives one-third ruby (三分ルビ) a block em of half the base
/// and an inline em of a third, so one [`Size`] names two different lengths; sharing one
/// slot between them would spend on the block axis a remainder produced against the inline
/// em, across the axis boundary ADR-0011 keeps closed.
///
/// There is no public remainder type and no way to obtain one. This is the only carrier
/// (ADR-0019), it is always taken as `&mut`, and every resolution reads and writes the
/// slot its em length and its axis name.
///
/// Fixed capacity, [`Carry::SIZES`] entries per axis, no allocation.
///
/// JLReq: §B.1, §3.3.3, ADR-0007
pub struct Carry { /* private */ }

impl Carry {
    /// The most em lengths one axis may carry a remainder for: 32. Well above the four
    /// character sizes the specification ever needs at once — base, ruby, warichu
    /// (割注), tate-chu-yoko (縦中横) — and bounded because this type keeps one remainder
    /// per em without allocating. `Text::new` validates a stream's scale table against it.
    /// A thirty-third em length on one axis resolves against a scratch remainder discarded
    /// on every call, which is stated rather than hidden: the signature has no error
    /// channel, and evicting a live remainder to make room would lose it just as quietly.
    pub const SIZES: usize = 32;
    pub const fn new() -> Self;
}

impl Default for Carry {
    /// Forced by `clippy::new_without_default`, which the workspace runs as an error.
    /// There is deliberately no `impl Default for Policy`; the difference is that a
    /// zeroed carry is the neutral value and a policy has no neutral value to be.
    fn default() -> Self;
}
```

### Arithmetic — M0

No `core::ops` trait is implemented for any type in this workspace. A bare `+` on a length
is therefore a compile error rather than a lint finding, no `#[allow]` is written anywhere,
and no shared configuration changes. This was measured: clippy's
`arithmetic-side-effects-allowed` resolves a crate-root-relative path *without* the crate
name, so one entry would silence the lint for every identically-named type in every crate.
It is not used. An `xtask ops` gate rejects any future `impl core::ops::*` on these types.

```rust
impl Em {
    /// JLReq: n/a (arithmetic)
    pub const fn add_sat(self, rhs: Self) -> Self;
    pub const fn sub_sat(self, rhs: Self) -> Self;
    pub const fn neg_sat(self) -> Self;
    pub const fn min(self, rhs: Self) -> Self;
    pub const fn max(self, rhs: Self) -> Self;
    pub const fn clamp_to(self, low: Self, high: Self) -> Self;
    pub const fn add_checked(self, rhs: Self) -> Option<Self>;
    pub const fn sub_checked(self, rhs: Self) -> Option<Self>;
    /// Scale by a ratio. `None` when the denominator does not divide the value **and**
    /// when the result would leave the bound: a scaled value past `LIMIT` has no
    /// representation, and returning it saturated would break the bound every other
    /// constructor enforces.
    /// JLReq: §3.3.3, §3.4.2
    pub const fn scaled(self, ratio: Ratio) -> Option<Self>;
}
// `Advance`, `InlineOffset`, `BlockOffset`, `InlineExtent`, and `BlockExtent` expose the
// identical inherent surface, each closed over its own type. There is no cross-axis
// addition, no `From`, and no shared trait: `docs/api-frozen.toml` names every one of
// them under `[[no_impl]]`.

/// Accumulates position along the inline axis without rounding drift.
///
/// The only type in the workspace that adds a length to a length in a loop. It does *not*
/// own a [`Carry`]: composition needs both a running position and the extents it feeds to
/// [`distribute`], so a cursor with a private remainder would be a second carrier of the
/// remainder for one em, and interleaving the two loses a unit — the defect ADR-0019
/// exists to remove, not one to reproduce inside the type that claims to have removed it.
/// One `Carry` is created per line and passed to every resolution on it, cursor and bridge
/// alike.
///
/// Bounded: once accumulation would exceed [`Advance::LIMIT`] the cursor records
/// saturation and [`InlineCursor::position`] answers `None`, so composition can report
/// the overflow with evidence rather than returning a wrong number.
///
/// JLReq: n/a (arithmetic)
pub struct InlineCursor { /* private */ }

impl InlineCursor {
    pub const fn new() -> Self;
    pub const fn advance(self, by: InlineExtent) -> Self;
    /// Move on by a writing-system fraction of a given size, spending that em's carried
    /// remainder. The signature is [`Em::resolve_inline`]'s, deliberately: the cursor is
    /// one more caller of the one bridge, not a second one. Takes a [`Size`] and not a
    /// [`Scale`], so the slot of [`Carry`] it touches is named by the argument rather than
    /// chosen here.
    pub const fn advance_em(self, by: Em, size: Size, carry: &mut Carry) -> Self;
    /// `None` once the accumulation has saturated.
    pub const fn position(self) -> Option<InlineOffset>;
}

/// Split `total` across `weights` so the parts sum to `total` exactly, whenever there is
/// at least one weight.
///
/// Two degenerate inputs are answered rather than refused, because the signature has no
/// error channel and each has one reading. A negative weight is no proportion at all —
/// §3.8.3's proportion is over character sizes, which are not negative — so it weighs
/// nothing. With **no** weights there is no site to place anything at, so the iterator is
/// empty and a non-zero total has nowhere to go; a caller holding space and no site has a
/// question this primitive cannot answer.
///
/// Serves every rule whose divisor depends on the text, and which therefore no choice of
/// denominator can make exact: "reduced equally" (§3.8.3), "added equally" (§3.8.4), the
/// group-ruby ratio (§3.3.6), and the proportional jukugo (熟語) expansion (§F.3.4).
///
/// A weight is an [`Advance`] because §3.8.3 says spacing is reduced "in proportion to the
/// character size", and a character size is a length in the caller's own unit. Only the
/// ratios matter, so the unit cancels; the sum is accumulated in `i64`, which a line's
/// worth of [`Advance::LIMIT`]-bounded weights cannot overflow. A narrower weight type
/// would truncate silently for callers whose em is larger than the type, which is the one
/// failure mode a distribution primitive must not have.
///
/// The remainder is a typographic decision JLReq does not make. It is a
/// [`Question`] in the policy space, and it arrives here as an argument only because
/// `jlreq-unit` does not depend on `jlreq-spec`; one function over [`Policy`] derives it
/// and every call site in the workspace uses that one, so the policy is still the single
/// carrier (ADR-0019).
///
/// JLReq: §3.8.3, §3.8.4, §3.3.6, §F.3.4
pub fn distribute(total: InlineExtent, weights: &[Advance], remainder: RemainderRule)
    -> Distribution<'_>;

/// Where the units that do not divide evenly go. JLReq states no rule; both readings are
/// permitted and both have conformance cases. Selected through [`Policy`].
///
/// JLReq: n/a (`decision:remainder`)
pub enum RemainderRule { /// Earliest sites in inline order.
                         Leading,
                         /// Latest sites in inline order.
                         Trailing }
```

### Axes and direction — M0

Each of the four is a distinct type over an `i32` in the caller's unit, with `ZERO`, `new`,
`units`, the inherent arithmetic surface above, and **nothing else public**. `ZERO` is not a
convenience: `new` returns `Option`, the workspace denies `unwrap_used` and `expect_used`,
and `#[allow]` is forbidden, so without it no crate above `jlreq-unit` could obtain a zero
extent at all. There is no
conversion between any two of them and none between any of them and [`Advance`]; the
`[[no_impl]]` table of [`docs/api-frozen.toml`](#the-frozen-api-file) makes that
mechanical.

`new` and `units` are themselves the residue, and pretending otherwise would be worse than
the leak. `BlockExtent::new(inline.units())` is a cross-axis assignment in two well-typed
steps, and no arrangement of types removes it: a value the caller supplies must get in, and a
position the caller draws must get out. So the untyped channel is narrowed instead. Calling
`new` or `units` on any of the four, or `Advance::get`, is permitted inside `jlreq-unit`'s
own `axis` and `length` modules and, everywhere else in the workspace, only inside an item
listed in `docs/scalar-sites.toml` — code-owner-guarded, one entry per site with its crate,
its item and the reason. The `ops` gate enforces it.

The four hold their integer in a *private* field, not a `pub(crate)` one, and the crate
reaches the channel through `pub(crate) const fn of(i32) -> Self` and
`pub(crate) const fn raw(self) -> i32`, which the `[[scalar_channel]]` method list names
beside `new` and `units`. A `pub(crate)` field would have been a second channel that names
no method, so the gate could not see `InlineExtent(raw)` or `value.0` at all — and those are
the forms a cross-axis assignment inside `jlreq-unit` would actually take. For the same
reason the closed arithmetic macro is *expanded* in `axis` and `length` rather than in
`arith`, so that its `Self(units)` stays at home. The list is short by construction: the
inherent arithmetic is closed over each type and every public output is already typed, so
what remains is `Badness::of`'s ratio, the bridge to the conformance case format, and the
two items in `jlreq-unit`'s own `arith` that turn a computed count back into a typed value —
`InlineCursor::position` and `Distribution::next`. This is [ADR 0011](../adr/0011-typed-axes-and-direction-as-a-datum.md)'s
mechanism, and the reason it is a list rather than a claim.

```rust
/// A position along the axis a line advances on.
///
/// Whether that is left-to-right, right-to-left, or top-to-bottom is the caller's
/// renderer's business (ADR-0004). There is no conversion to [`BlockOffset`] and no
/// arithmetic accepting one.
///
/// JLReq: §2.3.2
pub struct InlineOffset(i32);
/// A position along the axis lines stack on. JLReq: §2.3.2
pub struct BlockOffset(i32);
/// An extent along the inline axis: a line measure, an item's advance. JLReq: §3.8.1
pub struct InlineExtent(i32);
/// An extent along the block axis: how far ruby or warichu juts. JLReq: §4.5.1
pub struct BlockExtent(i32);

impl InlineExtent {
    pub const ZERO: Self;
    /// `None` beyond [`Advance::LIMIT`].
    pub const fn new(units: i32) -> Option<Self>;
    pub const fn units(self) -> i32;
    // pub(crate) const fn of(units: i32) -> Self;  — the same construction, clamped
    // pub(crate) const fn raw(self) -> i32;        — the same read, named for the gate
}
// The other three are identical.

/// The two ends of the inline axis. Appendix B's "line head" is inline-start and its
/// "line end" is inline-end. JLReq: §B.1
pub enum InlineEdge { Start, End }

/// The two sides of the block axis.
///
/// §3.3.4's ruby side is block-start in both directions — "right in vertical, above in
/// horizontal" is one side stated twice — and §4.5.1's first-line and last-line escape
/// rules are its exact dual. A correct implementation produces both of JLReq's sentences
/// from this one value; a conformance case requires it.
///
/// JLReq: §3.3.4, §4.5.1, §4.2.3
pub enum Side { BlockStart, BlockEnd }

/// The direction a line advances and stacks.
///
/// Exactly three rules read this, each marked direction-conditional in the generated rule
/// inventory, and a gate asserts that the set of rules consulting it equals that set:
/// §3.1.3 (ideographic numerals with `、` and `・` are set solid in vertical writing),
/// §3.2.5 (tate-chu-yoko 縦中横 exists only in vertical writing), and §3.3.5 (katatsuki
/// 肩付き ruby alignment is forbidden in horizontal writing). Everything else JLReq
/// states twice is axis mapping and is expressed with [`Side`] and [`InlineEdge`].
///
/// JLReq: §2.3.1, §2.3.2, ADR-0011
pub enum Direction {
    /// Horizontal writing (横組, yokogumi). JLReq: §2.3.2
    Horizontal,
    /// Vertical writing (縦組, tategumi). JLReq: §2.3.2
    Vertical,
}
```

### The item vocabulary — M0

```rust
/// What the caller's supplied advance covers: the character frame (字幅).
///
/// One field carries two rules the specification states separately, because they are the
/// same distinction. Appendix A's Remarks column disambiguates 473 of its 1133 keys on
/// this axis (§3.2.4 puts full-width and fixed-space Western characters in cl-19, §3.2.6
/// puts proportional ones in cl-27 and half-width numerals in cl-24). And §3.1.2 states
/// that the advance of commas (cl-07), full stops (cl-06), opening brackets (cl-01),
/// closing brackets (cl-02) and middle dots (cl-05) is half-width, with Table 1's amount
/// being what "makes them appear as if they were intrinsically full-width".
///
/// So for those five classes the frame decides which way the conditional space runs. A
/// closing bracket declared [`Frame::HalfEm`] has the Table 1 amount *added*; the same
/// bracket declared [`Frame::FullEm`] — the advance a modern font reports — already
/// contains it, and it is *trimmed*. Both are correct and they are the same geometry
/// reached from opposite directions; a library that assumed one would overshoot by half
/// an em at the commonest adjacency in Japanese text.
///
/// The trim is not silent. Composition normalizes to the specification's geometry and
/// reports every unit it took from a supplied advance, with the rule that states it, in
/// [`Line::trims`]. The consequence is the property the pair of worked conformance cases
/// asserts: the same text on either frame produces identical placements, identical
/// trailing space, and an identical extent (ADR-0017).
///
/// The default is [`Frame::Unstated`], not a guess. (`jlreq-unit` does not depend on
/// `jlreq-class`, so the doc comment on the type cannot link to [`Classified`] and does
/// not try: an unresolved intra-doc link fails `just doc` under `RUSTDOCFLAGS=-D warnings`.)
///
/// JLReq: §3.1.2, §3.2.4, §3.2.6, §A Remarks
pub enum Frame {
    /// Not stated. Multi-class keys resolve to their candidates rather than to a class.
    Unstated,
    /// The full ideographic em (全角, full-width).
    FullEm,
    /// Half an em (半角, half-width).
    HalfEm,
    /// A third of an em (三分角). JLReq: §A.25 U+002F
    ThirdEm,
    /// A quarter em (四分角). JLReq: §A.3 U+2010
    QuarterEm,
    /// A per-glyph advance (プロポーショナル). JLReq: §3.2.6
    Proportional,
}

/// The syntactic job the document gives this occurrence.
///
/// Needed by six code points and no others; leaving it unstated is safe everywhere else,
/// and where it is not, `jlreq::diagnose` names the item and the section.
///
/// JLReq: §3.1.3, §B.2#12, §C.2#11, §A.24
pub enum Role {
    Unstated,
    /// `U+002E` or `U+30FB` separating a fraction. JLReq: §3.1.3, §A.24
    DecimalPoint,
    /// `U+002C` or `U+0020` grouping digits (位取り). JLReq: §A.24
    DigitGroupSeparator,
    /// A component of an SI unit symbol; both sides set solid. JLReq: §B.2#12, §3.9.2
    UnitSymbol,
    /// A Western character used as a quantity symbol. JLReq: §C.2#11, §E.2#10
    QuantitySymbol,
    /// A full stop or comma terminating a sentence. JLReq: §3.1.1
    SentenceTerminator,
    /// A dividing punctuation mark (cl-04) inside a sentence rather than ending one;
    /// §3.1.6's Note gives it either solid setting or a quarter em, a caller choice.
    /// JLReq: §3.1.6
    SentenceMedial,
}

/// Which construct run this occurrence belongs to.
///
/// Run identity *is* the same-run/different-run predicate of §B.2 notes 9 through 11 and
/// §C.2 notes 6 through 8 and 13, so `jlreq-spacing` compares two of these for equality
/// without knowing that equality means "the same ruby group". `group` is the one further
/// level §C.2 note 8 needs: a break is allowed between two base characters of one
/// jukugo-ruby complex but not between the ruby characters attached to one base
/// character. No note anywhere needs a second level.
///
/// JLReq: §A.20–§A.23, §A.30, §B.2#9–#11, §C.2#6–#8, §C.2#13
pub struct Construct { /* private */ }

impl Construct {
    pub const fn new(kind: ConstructKind, run: RunId, group: Option<GroupId>) -> Self;
    pub const fn kind(self) -> ConstructKind;
    pub const fn run(self) -> RunId;
    pub const fn group(self) -> Option<GroupId>;
}

pub enum ConstructKind {
    /// cl-20, characters as reference marks (合印). JLReq: §A.20, §4.2.3
    ReferenceMark,
    /// cl-21, ornamented character complex — a base character with its superscripts and
    /// subscripts, which §3.7.1 makes indivisible and unexpandable. JLReq: §A.21, §3.7.1
    Ornamented,
    /// cl-22, simple-ruby complex — mono-ruby *and* group-ruby, per §A.22's
    /// "ruby other than jukugo-ruby". JLReq: §A.22, §3.3.5, §3.3.6
    NonJukugoRuby,
    /// cl-23, jukugo-ruby (熟語ルビ) complex. JLReq: §A.23, §3.3.7
    JukugoRuby,
    /// cl-30, characters in tate-chu-yoko. JLReq: §A.30, §3.2.5
    TateChuYoko,
    /// The interior of a warichu; carries no class of its own, unlike the cl-28 and
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
/// JLReq: §3.7.4
pub enum FormulaSetting { InLine, IndependentLine }

pub struct RunId(NonZeroU16);
pub struct GroupId(NonZeroU16);

impl RunId {
    pub const fn new(id: NonZeroU16) -> Self;
    pub const fn get(self) -> NonZeroU16;
}
// `GroupId` is identical.

/// Which construct run each item of one [`Text`] belongs to.
///
/// Run identity is here and **not** on [`Item`], because an item is what the caller
/// measured and a run is what lowering computed; two carriers of one fact are two things
/// a caller can desynchronize (ADR-0015).
///
/// The constructor validates rather than trusts: every identity must name one contiguous
/// span, and no two kinds may share one. A caller with its own construct model can
/// therefore build this directly and skip `jlreq-inline` entirely — a real capability,
/// and the reason uniqueness is checked rather than promised by whoever allocated it.
///
/// JLReq: §B.2#9–#11, §C.2#6–#8, §C.2#13
pub struct Runs<'a> { /* private */ }

impl<'a> Runs<'a> {
    /// One slot per item of the text this overlays.
    pub fn new(slots: &'a [Option<Construct>]) -> Result<Self, RunsError>;
    /// Text with no constructs. Total, so every signature taking `Runs` has an answer
    /// for plain text and there is no second code path for it.
    pub const fn none() -> Self;
    pub fn of(self, item: ItemIndex) -> Option<Construct>;
    pub fn len(self) -> usize;
    /// Forced by `clippy::len_without_is_empty`, which is warn-by-default and therefore an
    /// error under `RUSTFLAGS="-D warnings"`.
    pub fn is_empty(self) -> bool;
}

pub enum RunsError {
    /// One [`RunId`] appears at two non-adjacent positions.
    RunNotContiguous { run: RunId, at: ItemIndex },
    /// One [`RunId`] is used by two different [`ConstructKind`]s.
    RunKindConflict { run: RunId, at: ItemIndex },
    /// A [`GroupId`] spans two runs.
    GroupCrossesRun { group: GroupId, at: ItemIndex },
}

/// One occurrence of one Appendix A key, as the caller already holds it.
///
/// `start` is a byte offset into the stream this item belongs to; the extent is implied by
/// the next item's `start`. Constructed from [`Item::new`] and configured by `with_*`,
/// never as a struct literal (ADR-0012).
///
/// **The granularity is exact and it is checked** (ADR-0018). One item is one Appendix A
/// key, so `classify` is total over items and an adjacency between two items is an
/// adjacency between two keys, which is what Appendices B through E are indexed by. Two
/// mismatches would otherwise be silent and both are refused by `Text::new`:
///
/// - A key split across two items. Appendix A keys twenty-five entries on an ordered pair,
///   and a shaper may emit either half as its own glyph. `<02E5, 02E9>` is a cl-27 falling
///   tone contour whose first code point is *also* listed alone, so splitting it yields two
///   plausible cl-27 answers; `<31F7, 309A>` is a cl-11 small kana whose second code point
///   is listed nowhere, so splitting it yields cl-11 followed by an unlisted reading. The
///   caller merges the glyphs into one item whose advance is their sum, which loses nothing:
///   no cell of any matrix is indexed inside a key.
/// - An item covering more than one key, **unless** it declares [`Frame::Proportional`] and
///   every key in it is listed in cl-27. That exception is the shaper's own output and
///   nothing else — §3.2.6 puts proportional Western characters in cl-27, Table 1 sets
///   cl-27 against cl-27 solid, and §C.2 note 12 requires a caller-supplied hyphen before a
///   Western word may be divided at all — so a Latin ligature contains no amount and no
///   break for the merge to have destroyed. §3.2.1's own example of Western text in
///   Japanese, the word `editor`, is six items or one, and both are well formed.
///
/// The three facts that always matter are constructor arguments, not builder steps. An
/// omitted advance is a zero-width character and a silently short line, and an omitted
/// size is the wrong size on every annotated line — the two loudest instances of the
/// failure this library exists to prevent, and neither has an "unstated" answer that
/// could be reported instead. The role is genuinely optional and does have one. The frame
/// has one for the cl-19 against cl-27 axis and does *not* have one for the five classes of
/// §3.1.2, where it names a geometry rather than a class, so `Text::new` requires it there.
///
/// JLReq: §A, §3.1.2, §3.2.6, §C.2#12, ADR-0002, ADR-0018
pub struct Item { /* private */ }

impl Item {
    pub const fn new(start: ByteOffset, advance: InlineExtent, scale: ScaleId) -> Self;
    pub const fn with_frame(self, frame: Frame) -> Self;
    pub const fn with_role(self, role: Role) -> Self;
    pub const fn start(self) -> ByteOffset;
    pub const fn advance(self) -> InlineExtent;
    pub const fn frame(self) -> Frame;
    pub const fn role(self) -> Role;
    pub const fn scale(self) -> ScaleId;
}

/// A byte offset into one stream. Distinct from [`ItemIndex`] so the two index spaces
/// cannot be confused at a call site.
///
/// Deliberately *not* split per stream the way the ordinals are. A byte offset is only ever
/// dereferenced through the stream that owns the item carrying it, and the two places a
/// bare one appears in the surface — a break [`Candidate`] and [`Line::bytes`] — are
/// running text by definition, because annotation streams are not broken into lines
/// (ADR-0016).
pub struct ByteOffset(u32);

impl ByteOffset {
    pub const fn new(bytes: u32) -> Self;
    pub const fn get(self) -> u32;
}

/// An ordinal into one running-text stream's items.
///
/// Annotation streams are indexed by `AnnotationIndex` instead, so a base range and an
/// annotation range cannot be swapped at a call site or inside a struct: the invariant is a
/// compile error rather than a comment (ADR-0016). Depth is exactly one — every construct
/// that owns a stream attaches to running text, and none sits inside another's — so two
/// ordinal types are enough and no stream identifier is threaded anywhere.
pub struct ItemIndex(u32);

impl ItemIndex {
    pub const fn new(index: u32) -> Self;
    pub const fn get(self) -> u32;
}

// The five newtypes above and below carry `new` and `get` for the same reason
// `BlockDemand` carries a constructor: each appears in an input position of a public
// function — `Item::new`, `Runs::of`, `Construct::new`, `BlockDemand::new` — and each is
// read back, so ADR-0012's own gate would reject them without a named constructor. The
// getters are named `get`, not `units`, because these are ordinals rather than lengths;
// the `[[scalar_channel]]` type list is what keeps that name out of the axis check.

/// Which declared construct one [`RunId`] came from.
///
/// [`lower`] allocates the identities, so the caller never saw them; this is the map back,
/// and it is the caller's own coordinates — the construct kind and the position in the
/// slice it passed. Every error and every placed annotation names a construct this way, so
/// a report reads "the ruby you passed third" rather than an ordinal the caller cannot
/// resolve (ADR-0015).
///
/// A caller that built [`Runs`] itself owns the identities already and does not need this.
pub struct ConstructRef { /* kind: ConstructKind, ordinal: u16 */ }

impl ConstructRef {
    pub const fn new(kind: ConstructKind, ordinal: u16) -> Self;
    pub const fn kind(self) -> ConstructKind;
    /// The position in the slice the caller passed for that kind.
    pub const fn ordinal(self) -> u16;
}

/// How far a run of items needs beyond the line on each side of the block axis.
///
/// §4.5.1: the kihon-hanmen (基本版面) line gap is *not* changed to accommodate ruby;
/// on the first or last line of the area the jutting part is placed outside it. Only the
/// page layer knows where that edge is, so the line layer reports this and the caller
/// decides.
///
/// JLReq: §4.5.1, §2.4.2, §2.5.1
pub struct BlockDemand { /* private */ }

impl BlockDemand {
    /// `jlreq-inline` builds these and `Line::block_demand` returns them, so both a
    /// constructor and accessors are public: a seam type readable at one end and not
    /// writable at the other is a seam with nothing on the far end (ADR-0012).
    pub const fn new(items: Range<ItemIndex>, start: BlockExtent, end: BlockExtent)
        -> Self;
    pub const fn items(self) -> Range<ItemIndex>;
    /// Toward the block-start side: the ruby side of §3.3.4 and §3.3.9.
    pub const fn start(self) -> BlockExtent;
    pub const fn end(self) -> BlockExtent;
}

/// How far ruby may extend beyond its base at one boundary.
///
/// Appendix B's legend defines two structurally different permissions and this type keeps
/// them apart. `1/2 be hang` and its siblings permit extension *over the space* and say
/// ruby "shall not be extended over the other character", capped by whatever the space is
/// after line adjustment. `ruby hang` sits on a solid cell — there is no space — and
/// permits extension over the adjacent character itself.
///
/// Resolved by `jlreq-line` after adjustment and reported per boundary, because the cap
/// is not known until then (§3.3.8 rule 3).
///
/// JLReq: §B.1, §3.3.8
pub enum RubyOverhang {
    None,
    /// Over the inter-character space only, up to `limit` in ruby ems, and never past
    /// the space. JLReq: §B.1 `1/2 be hang`, `1/4 af hang`
    OverSpace { limit: Em },
    /// Over the adjacent character body, up to `limit` in ruby ems. JLReq: §B.1
    /// `ruby hang`, §B.2#7
    OverCharacter { limit: Em },
}
```

### The seam — M0

Four types cross from `jlreq-inline` to `jlreq-line`, and nothing else does. They live here
so neither crate names a type the other owns
([ADR 0015](../adr/0015-the-crate-graph-and-the-inline-line-seam.md)): [`Runs`] above, and
[`Segment`], [`Separation`] and [`BlockDemand`] here.

None of them carries the rule that produced it
([ADR 0020](../adr/0020-the-seam-carries-no-rule-address.md)). A `rule: RuleId` field would
need the edge `jlreq-unit -> jlreq-spec`, which ADR 0019 forbids and which would close a
cycle against `Policy::remainder`; and it would be a second provenance mechanism in a
workspace that has one. `jlreq-inline` produces `Answer<Segment<'_>>` and
`Answer<Separation>` instead — both `Copy`, both crates already depend on `jlreq-spec`, and
the `Provenance` inside carries up to three rules and the standing of the chain, which is
strictly more than one `RuleId`.

```rust
/// A span of items the line layer does not lay out as ordinary inline text.
///
/// One concept, four of JLReq's constructs, and the line layer meets none of their names.
/// A segment carries its own size, because three of the four are set smaller than the
/// text around them.
///
/// JLReq: §3.2.5, §3.4.2, §3.4.3, §3.7.2, §3.7.3
pub struct Segment<'a> { /* private */ }

impl<'a> Segment<'a> {
    pub const fn new(items: Range<ItemIndex>, scale: ScaleId, interior: Interior<'a>)
        -> Self;
    pub const fn items(self) -> Range<ItemIndex>;
    pub const fn scale(self) -> ScaleId;
    pub const fn interior(self) -> Interior<'a>;
}

/// How a segment's interior relates to the line containing it.
pub enum Interior<'a> {
    /// Laid out on an axis this line does not own, and occupying `extent` along the
    /// inline axis. §3.2.5 sets tate-chu-yoko (縦中横) left to right and then centers
    /// "the whole string" on the vertical line, so the outer line sees one advance and
    /// a block-axis jut, not a nested writing mode. JLReq: §3.2.5
    Opaque { extent: InlineExtent },
    /// One sub-line whose inter-character spacing is adjusted so the span occupies
    /// exactly `extent`. §3.7.3's jidori (字取り), including its rule that spacing is
    /// not added where a break is prohibited, and its rule that a single character is
    /// set at the inline start of the block. JLReq: §3.7.3
    Filled { extent: InlineExtent },
    /// `parts` sub-lines as near equal in length as possible, none longer than an
    /// earlier one, split where the break rules permit. §3.4.2's warichu (割注).
    /// `straddle` is the only place it is ever [`Straddle::Permitted`], because §3.4.3
    /// is the only section that permits it. JLReq: §3.4.2, §3.4.3
    Balanced { parts: NonZeroU8, straddle: Straddle },
    /// Sub-lines split at exactly these positions, each starting at the segment's inline
    /// start, the segment as long as the longest of them. §3.7.2's furiwake (振分け),
    /// whose splits are declared by the document rather than searched for.
    /// JLReq: §3.7.2
    Declared(&'a [ItemIndex]),
}

/// Whether a segment may continue onto the following line.
///
/// §3.4.3 permits it for warichu and the Japanese note records two-line straddling as
/// 頻出, frequent. §3.7.2 forbids it for furiwake in one sentence: "One furiwake block
/// should not be extended across multiple base text lines." §3.2.5 and §3.7.3 are within
/// one line by construction.
///
/// JLReq: §3.4.3, §3.7.2
pub enum Straddle { Forbidden, Permitted }

/// The least inline space a construct forces at a base-text boundary.
///
/// §3.3.8 rule 1 forbids ruby from overhanging an adjacent ideographic character, so a
/// base character carrying more ruby than it is wide pushes its neighbors apart before
/// composition begins. That is natural advance rather than an adjustment opportunity, and
/// conflating the two composes every such line short — §3.3.1's note concludes that such
/// a line "needs some line adjustment processing" rather than that it offers some.
///
/// JLReq: §3.3.8, §3.3.1
pub struct Separation { /* private */ }

impl Separation {
    pub const fn new(after: ItemIndex, least: InlineExtent) -> Self;
    pub const fn after(self) -> ItemIndex;
    pub const fn least(self) -> InlineExtent;
}
```

## `jlreq-spec`

The specification-reference vocabulary. Generated; no lengths, no classes, no tables.

```text
src/
  lib.rs        re-exports; crate docs
  rule.rs       RuleId, Address, Standing, RULES  (generated)
  answer.rs     Answer<T>, Provenance
  policy.rs     Question, Choice, Policy, QUESTIONS  (generated)
```

This crate's two generated tables sit beside the vocabulary that indexes them rather than
under a `generated/` directory, which the three crates below do use. A rule address, its
inventory and the identifier that reads it are one module's worth of subject, and splitting
them would put a `pub(crate)` table one `use` away from its only reader for no gain.
`docs/design/generation.md` states the same, so the pipeline document and this file map
describe one layout.

### Rule addressing — M0

```rust
/// A stable identifier for one normative statement of JLReq.
///
/// The address is the specification's own, so a failure report is readable by someone who
/// has never seen this code (ADR-0013). Generated from the rule inventory.
///
/// JLReq: n/a (addressing)
pub struct RuleId(u16);

impl RuleId {
    /// Every rule kumihan implements. The coverage gate subtracts from this.
    pub const ALL: &'static [Self];
    /// Named constants, one per inventoried rule.
    pub const LINE_START_PROHIBITION: Self;   // 3.1.7
    pub const LINE_END_PROHIBITION: Self;     // 3.1.8
    pub const PUNCTUATION_FRAME: Self;        // 3.1.2
    pub const MIDDLE_DOT_SUM: Self;           // B.2#3
    pub const INSEPARABLE_PAIRS: Self;        // C.2#5
    pub const WESTERN_HYPHENATION: Self;      // C.2#12
    // ...

    /// The canonical rendering: `3.1.9`, `B.2#3`, `B.1@cl-05,cl-05`.
    pub const fn address(self) -> Address;
    /// The sentence, quoted from the specification.
    pub const fn statement(self) -> &'static str;
    /// Whether this rule reads the writing direction (ADR-0011). Exactly three do.
    pub const fn is_direction_conditional(self) -> bool;
    /// What kind of claim this rule is.
    pub const fn standing(self) -> Standing;
    pub fn parse(address: &str) -> Option<Self>;
}

/// A parsed specification address. Byte-identical in the tables, in doc comments, and in
/// the conformance case files.
///
/// Grammar: `section := digit+ ('.' digit+)* | [A-G] ('.' digit+)*`,
/// `address := section ('#' note)? | section '@' cell`.
/// The `#` is kumihan's separator for JLReq's "note N", which the published document
/// gives no machine-readable identifier; that is recorded rather than glossed over. A cell
/// coordinate is a class or one of the two line edges, spelled `line-head` and `line-end`
/// — hyphenated, byte-identically in the captured transcription, in the inventory, in a doc
/// comment and in a case file, so nothing anywhere translates between two forms of it.
///
/// The row and the column are two coordinate vocabularies and not one. A matrix carries one
/// line-head *row* and one line-end *column*, which is the frozen reason
/// `jlreq_spacing::Before` and `jlreq_spacing::After` are two types, so
/// `B.1@cl-02,line-head` and `B.1@line-end,cl-05` address cells no matrix has and are not
/// addresses.
pub struct Address(/* private */);

impl Address {
    /// Parse the canonical rendering. `None` when `text` is not one. This is the only way
    /// to build one, which is what makes the canonical form the only form; `Class::enumeration`
    /// in `jlreq-class` is a `const fn` that must build one, so it is public and `const`.
    pub const fn parse(text: &str) -> Option<Self>;
}

impl fmt::Display for Address {
    /// The canonical rendering. A type that is "byte-identical in the tables, in doc
    /// comments, and in the case files" needs a way to produce those bytes.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result;
}

// `docs/design/address-corpus.tsv` is the corpus both parsers of this grammar read — this
// one, and `xtask`'s, which cannot depend on this crate. A spelling one accepts and the
// other refuses fails a test rather than reaching a published case file.

/// What kind of claim a rule makes.
///
/// The last two exist because the specification contradicts itself in places and leaves
/// holes in others. A library that quietly filled them would publish invention as
/// requirement (ADR-0013).
pub enum Standing {
    /// Normative specification text.
    Normative,
    /// JLReq permits several answers here; the choice is a [`Question`].
    Alternative,
    /// JLReq says nothing. This is kumihan's published reading, recorded in
    /// `docs/decisions/`. Examples: an unlisted code point (§3.9.2), emphasis dots
    /// (圏点) having no class or table row (§3.3.9), warichu line adjustment (§3.8.3).
    Unstated,
    /// The specification says two incompatible things. Both are recorded; the case
    /// carries both readings. Examples: §D.2 note 5 against notes 1–3 on a priority
    /// ordinal; §3.1.3's Note reading "vertical" in English against 横組 in Japanese;
    /// §3.8.3 numbering Appendix D's tables one higher than Appendix D does.
    Adjudicated,
}
```

### Answers — M0

```rust
/// A value together with why it is that value.
///
/// One shape for every layer. A conformance case can assert "this half em came from
/// §B.2 note 5", so an implementation that gets the right number from the wrong sentence
/// fails — which matters because several answers are reachable by two rules with
/// different policy sensitivity.
///
/// JLReq: n/a (ADR-0013)
pub struct Answer<T> { /* value: T, why: Provenance */ }

impl<T: Copy> Answer<T> {
    /// Public because `jlreq-class`, `jlreq-spacing`, `jlreq-line` and `jlreq-inline` all
    /// produce answers and none of them is this crate. A type four crates must build and
    /// cannot is an unconstructible input in the sense ADR-0012's gate rejects.
    pub const fn new(value: T, why: Provenance) -> Self;
    pub const fn value(self) -> T;
    pub const fn why(self) -> Provenance;
}

/// Why an answer is what it is. Fixed capacity, no allocation, so the no-alloc crates
/// can carry it.
///
/// The chain is bounded by the specification at two steps: disambiguate, then at most one
/// reclassification. A single slot would lose the first — `%` is chosen between cl-13 and
/// cl-27 by frame and *then* moved to cl-19 by §C.2's alternative.
pub struct Provenance { /* rules: [Option<RuleId>; 3], standing: Standing */ }

impl Provenance {
    /// One rule, the ordinary case.
    pub const fn of(rule: RuleId, standing: Standing) -> Self;
    /// A rule chained onto an existing provenance: the disambiguation, then at most one
    /// reclassification. `None` when the chain is already full, which the specification
    /// makes unreachable and which is therefore reported rather than truncated.
    pub const fn then(self, rule: RuleId, standing: Standing) -> Option<Self>;
    pub fn rules(self) -> impl Iterator<Item = RuleId>;
    pub const fn standing(self) -> Standing;
    /// Frozen projection (ADR-0012): whether the specification decides this.
    pub const fn is_specified(self) -> bool;
}
```

### Policy — M0

Zero cargo features. The feature-matrix cost is quadratic in features per core crate, and
there are roughly forty places where JLReq permits alternatives; as features that is
uncomputable, and it would also be wrong, because a policy is a property of a document
rather than of a build — one process may set a quotation to JIS conventions inside a book
set to JLReq conventions.

```rust
/// A place where JLReq permits more than one answer. Generated.
pub struct Question(u16);

impl Question {
    pub const ALL: &'static [Self];
    /// `ALL.len()`. Generated, and the width of [`Policy`]'s representation.
    pub const COUNT: usize;

    /// The four strictness levels. JLReq: §C.3
    pub const KINSOKU_LEVEL: Self;
    /// Which of Tables 3, 4 and 5 governs reduction. JLReq: §D
    pub const REDUCTION_TABLE: Self;
    /// Spacing after cl-02, cl-06 and cl-07 at the line end. JLReq: §B.2#2, §3.1.9
    pub const LINE_END_PUNCTUATION: Self;
    /// Spacing before cl-01 at the line head. JLReq: §B.2#17, §3.1.5
    pub const LINE_HEAD_OPENING_BRACKET: Self;
    /// How far ruby may overhang adjacent kana. The English of §B.2 note 7 names only
    /// katakana; the Japanese names cl-16, cl-10 and cl-11, and §3.3.8 rule 2 adds
    /// cl-15. This project follows the Japanese and records the divergence.
    /// JLReq: §B.2#7, §3.3.8
    pub const RUBY_OVERHANG_KANA: Self;
    /// Whether ruby may overhang the paragraph first-line indent. JLReq: §B.2#8
    pub const RUBY_OVERHANG_INDENT: Self;
    /// Ruby alignment: nakatsuki (中付き) or katatsuki (肩付き). The document's default;
    /// [`Ruby::with_alignment`] overrides it for one ruby, which is the precedence rule
    /// ADR-0019 states once and applies everywhere. `Policy::JLREQ` follows §3.3.5's
    /// recommendation against katatsuki in horizontal writing; a caller who overrides that
    /// is honored and told, because "should not be adopted" is a recommendation over a
    /// construct that is perfectly well defined there. JLReq: §3.3.5
    pub const RUBY_ALIGNMENT: Self;
    // There is deliberately no `RUBY_SIZE`. The ruby em is the annotation stream's own
    // declared `Scale`, because ADR-0002 makes a measured quantity the caller's and
    // §3.3.3 does not close the set — for headings at twelve points or more it says the
    // ruby "is generally smaller than half the size of the base characters" with no ratio
    // given at all, which no enumeration states and a declared scale states exactly
    // (ADR-0019).
    /// Group-ruby distribution when ruby is shorter than base. JLReq: §3.3.6
    pub const GROUP_RUBY_DISTRIBUTION: Self;
    /// Jukugo-ruby layout when a base needs three or more ruby characters.
    /// JLReq: §3.3.7, §F
    pub const JUKUGO_RUBY_LAYOUT: Self;
    /// What to do when `々` would fall at a line head — including §B.2 note 14's third
    /// answer, which replaces the character rather than moving it.
    /// JLReq: §B.2#14, §3.1.7
    pub const ITERATION_MARK_AT_LINE_HEAD: Self;
    /// Hanging punctuation (ぶら下げ, burasage). JLReq: §3.8.2, §2.5.1
    pub const HANGING_PUNCTUATION: Self;
    /// Whether a break is allowed between cl-24 and cl-27. JLReq: §C.2#10
    pub const GROUPED_NUMERAL_BEFORE_WESTERN: Self;
    /// Dividing punctuation inside a sentence: solid, or a quarter em. JLReq: §3.1.6
    pub const SENTENCE_MEDIAL_DIVIDING_MARK: Self;
    /// The ceiling on Japanese/Latin expansion. §E.1's Japanese legend permits a third
    /// em where the English gives only a half; §3.8.4 (b) permits both, and a further
    /// reading treats the space as rigid. JLReq: §E.1, §3.8.4
    pub const JAPANESE_LATIN_EXPANSION_CEILING: Self;
    /// The expansion priority order, which §3.8.4 attributes to JIS X 4051 rather than
    /// stating as JLReq's own. JLReq: §3.8.4, §E
    pub const EXPANSION_ORDER: Self;
    /// Which order [`Preference`] compares the [`Demerits`] components in.
    ///
    /// JLReq states no paragraph-level objective. §C.3's closing paragraph describes
    /// what its four *levels* achieve — "the very strict rule is for the best appearance
    /// at the line head, while the strict rule is best to avoid inter-character spacing
    /// adjustment" — which is guidance on choosing a level, not a rule for ranking two
    /// candidate paragraphs. The components and the order over them are therefore
    /// kumihan's, published in `docs/decisions/` with [`Standing::Unstated`], and the two
    /// named orders each have a conformance case.
    /// JLReq: §C.3 (silence), `decision:adjustment-preference`
    pub const ADJUSTMENT_PREFERENCE: Self;
    /// Where an indivisible remainder goes when a total is distributed. JLReq: n/a
    pub const REMAINDER: Self;
    /// The class of a code point Appendix A does not list. JLReq: §3.9.2
    pub const UNLISTED_CODE_POINT: Self;
    /// The class of an occurrence Appendix A lists under several classes that no
    /// caller-supplied axis separates. §3.9.2's own conceded example is a Latin spelling
    /// parenthesized inside Japanese — "エディター（editor）は……" — whose brackets it
    /// says "Japanese design is better" for. That is a preference and not a rule, so it
    /// is the [`Choice`] marked preferred rather than a constant.
    /// This is the sibling [`Classified::Irreducible`] and [`resolve`] apply.
    /// JLReq: §3.9.2
    pub const AMBIGUOUS_CONTEXT: Self;
    /// Which of §C.3's matrix-relaxation reading and §C.2's reclassification reading
    /// applies. The specification states the same three relaxations both ways and they
    /// are not equivalent. JLReq: §C.3, §C.2#1–#3
    pub const RELAXATION_MECHANISM: Self;
    // ... roughly forty in all, every one generated with its section.

    /// The permitted answers.
    pub const fn permits(self) -> &'static [Choice];
    /// The section that permits the alternative.
    pub const fn rule(self) -> RuleId;
    /// The stable dotted path used in a conformance case file.
    pub const fn path(self) -> &'static str;
}

/// One permitted answer.
///
/// A `Choice` carries the [`Question`] it answers, and [`Policy::with`] reads the
/// question out of it, so setting a question to a choice belonging to a different
/// question is not an expression that can be written.
pub struct Choice { /* question: Question, index: u8 */ }

impl Choice {
    pub const fn question(self) -> Question;
    /// The section that states this alternative.
    pub const fn rule(self) -> RuleId;
    /// e.g. "JIS X 4051: ruby shall not extend over katakana".
    pub const fn statement(self) -> &'static str;
    /// Whether JLReq calls this one "preferred". JLReq: §B.2#1, #2, #4, #6, #7, #8, #17
    pub const fn is_preferred(self) -> bool;
    /// The stable name used in a conformance case file.
    pub const fn name(self) -> &'static str;
}

/// The permitted alternative in force at every question.
///
/// Total by construction: there is no unset question, so no default hides in an
/// evaluator. Opaque, so adding a question is not a breaking change.
///
/// JLReq: §B.2, §C.2, §C.3, §D, §E.2
pub struct Policy { /* choices: [u8; Question::COUNT] */ }

impl Policy {
    /// JLReq's own preference wherever it states one: Strict kinsoku (§C.3 level 3,
    /// which JLReq labels "Default, general publications") and reduction Table 3
    /// ("the method adopted by this document").
    pub const JLREQ: Self;
    /// The JIS X 4051 reading wherever JLReq records a divergence. This is JLReq's
    /// account of JIS, not JIS conformance — note that §B.2 note 5's divergence is a
    /// different class lattice (cl-07 as a subset of cl-02), not a spacing choice.
    pub const JIS_READING: Self;
    /// Book practice: reduction Table 5, §3.1.5 pattern 3, hanging punctuation.
    /// JLReq: §D, §3.1.5, §2.5.1
    pub const BOOK: Self;
    /// Magazine practice: Loose kinsoku. JLReq: §C.3 level 2
    pub const MAGAZINE: Self;
    /// Newspaper practice: Very loose kinsoku. JLReq: §C.3 level 1
    pub const NEWSPAPER: Self;

    /// Set one question. Returns `Err` when the result would be a combination JLReq makes
    /// contradictory — a Very strict level alongside a §C.2 alternate rule, which §C.3
    /// defines Very strict as excluding.
    ///
    /// This is a `Result` rather than a separate `validate`, and the reason is ADR-0010's:
    /// a contradictory policy has no representation, so no entry point has to check for
    /// one and none can forget to. There is deliberately no way to build a policy that
    /// `compose` would have to reject.
    /// JLReq: §C.3
    pub const fn with(self, choice: Choice) -> Result<Self, PolicyConflict>;
    pub const fn get(self, question: Question) -> Choice;
    /// Every question, its answer, and the section that permits it. No other
    /// implementation of Japanese layout can report this.
    pub fn explain(self) -> impl Iterator<Item = (Question, Choice)>;
    pub fn diff(self, base: Self) -> impl Iterator<Item = (Question, Choice)>;
    /// The remainder rule in force, for [`distribute`], which lives below the policy
    /// space and takes it as an argument. This is the one derivation, so the policy stays
    /// the single carrier (ADR-0019).
    pub const fn remainder(self) -> RemainderRule;
}
```

`Policy::JLREQ` is a citable factual claim, checked field by field against the quotes by a
conformance case. It is not a default: there is no `impl Default for Policy`, so a caller
names a preset and the choice appears in their source where a reviewer sees it.

## `jlreq-class`

Appendix A. Allocation-free; binary search over sorted static tables.

```text
src/
  lib.rs                re-exports; crate docs
  class.rs              Class, ClassSet
  member.rs             Member, members(), fold_compatibility()
  text.rs               Text, Annotation, AnnotationIndex, TextError
  classify.rs           Classified, Subject, Reclassification, classify(), resolve()
  usage.rs              Usage, usage()
  generated/
    appendix_a.rs       the 1133 keys           (generated, M0)
    ideograph.rs        the cl-19 predicate     (generated, M0)
    folding.rs          Wide/Narrow mapping     (generated, M0)
    script.rs           the small-kana fallback (generated, M0)
```

### Classes — M0

```rust
/// A JLReq character class, cl-01 through cl-30.
///
/// The one exhaustive public enum in this workspace whose cardinality is ours to state:
/// §3.9.2 closes the set, and a caller legitimately matches all thirty. A catch-all arm
/// over character classes is exactly where a silently wrong default hides, so this type
/// is deliberately not `#[non_exhaustive]` (ADR-0012).
///
/// The line edges are *not* members. Appendix B gives them one row and one column, not a
/// symmetric axis value; see [`Before`] and [`After`].
///
/// JLReq: §3.9.2, §A
pub enum Class {
    /// cl-01, opening brackets (始め括弧類). JLReq: §A.1
    OpeningBracket,
    /// cl-02, closing brackets (終わり括弧類). JLReq: §A.2
    ClosingBracket,
    /// cl-03, hyphens (ハイフン類). JLReq: §A.3
    Hyphen,
    /// cl-04, dividing punctuation marks (区切り約物). JLReq: §A.4
    DividingPunctuation,
    /// cl-05, middle dots (中点類). JLReq: §A.5
    MiddleDot,
    /// cl-06, full stops (句点類). JLReq: §A.6
    FullStop,
    /// cl-07, commas (読点類). JLReq: §A.7
    Comma,
    /// cl-08, inseparable characters (分離禁止文字). JLReq: §A.8
    Inseparable,
    /// cl-09, iteration marks (繰返し記号). JLReq: §A.9
    IterationMark,
    /// cl-10, the prolonged sound mark (長音記号). JLReq: §A.10
    ProlongedSoundMark,
    /// cl-11, small kana (小書きの仮名). JLReq: §A.11
    SmallKana,
    /// cl-12, prefixed abbreviations (前置省略記号). JLReq: §A.12
    PrefixedAbbreviation,
    /// cl-13, postfixed abbreviations (後置省略記号). JLReq: §A.13
    PostfixedAbbreviation,
    /// cl-14, the full-width ideographic space (和字間隔). JLReq: §A.14
    IdeographicSpace,
    /// cl-15, hiragana (平仮名). JLReq: §A.15
    Hiragana,
    /// cl-16, katakana (片仮名). JLReq: §A.16
    Katakana,
    /// cl-17, math symbols (等号類). JLReq: §A.17
    MathSymbol,
    /// cl-18, math operators (演算記号). JLReq: §A.18
    MathOperator,
    /// cl-19, ideographic characters (漢字等). Contains 66 Cyrillic and 49 Greek
    /// letters; the name is JLReq's, and it is not a description. JLReq: §A.19
    Ideographic,
    /// cl-20, characters as reference marks (合印中の文字). JLReq: §A.20
    AsReferenceMark,
    /// cl-21, characters in an ornamented complex. JLReq: §A.21
    InOrnamentedComplex,
    /// cl-22, characters in a non-jukugo ruby complex. JLReq: §A.22
    InNonJukugoRubyComplex,
    /// cl-23, characters in a jukugo-ruby complex. JLReq: §A.23
    InJukugoRubyComplex,
    /// cl-24, grouped numerals (連数字中の文字). JLReq: §A.24
    InGroupedNumeral,
    /// cl-25, unit symbols (単位記号中の文字). JLReq: §A.25
    InUnitSymbol,
    /// cl-26, the Western word space (欧文間隔). JLReq: §A.26
    WesternWordSpace,
    /// cl-27, Western characters (欧文用文字). JLReq: §A.27
    Western,
    /// cl-28, warichu opening brackets (割注始め括弧類). JLReq: §A.28
    WarichuOpeningBracket,
    /// cl-29, warichu closing brackets (割注終わり括弧類). JLReq: §A.29
    WarichuClosingBracket,
    /// cl-30, characters in tate-chu-yoko (縦中横中の文字). JLReq: §A.30
    InTateChuYoko,
}

impl Class {
    pub const ALL: [Self; 30];
    /// `1` through `30`. JLReq: §3.9.2
    pub const fn number(self) -> u8;
    /// The identifier JLReq uses in every rule sentence: `cl-01` … `cl-30`.
    pub const fn id(self) -> &'static str;
    /// JLReq's English name, generated from §3.9.2 rather than from an Appendix A
    /// heading — §A.19's heading names the class's own complement.
    pub const fn name_en(self) -> &'static str;
    /// JLReq's Japanese name, from §3.9.2. JLReq: §3.9.2
    pub const fn name_ja(self) -> &'static str;
    /// The Appendix A section enumerating this class, if it enumerates anything.
    /// Five classes enumerate nothing: their section text reads in full "Any character
    /// may participate in …". JLReq: §A.20–§A.23, §A.30
    pub const fn enumeration(self) -> Option<Address>;
    pub const fn from_number(n: u8) -> Option<Self>;
}

/// A set of classes, as a bitmask. Allocation-free and order-deterministic, which is why
/// it is not a `BTreeSet`.
pub struct ClassSet(u32);
```

### The Appendix A key — M0

```rust
/// The key Appendix A is indexed by.
///
/// Twenty-five entries key on an *ordered pair* of code points, and cl-27 lists
/// `<02E5, 02E9>` and `<02E9, 02E5>` as two distinct members, so this is a sequence and
/// not a set. A `char`-keyed lookup cannot express Appendix A.
///
/// `MAX_LEN` is generated from the table, with a compile-time assertion, so a
/// specification revision adding a three-code-point member is a build failure rather than
/// a silent truncation.
///
/// JLReq: §A
pub struct Member { /* private */ }

impl Member {
    pub const MAX_LEN: usize;
    pub const fn single(c: char) -> Self;
    pub const fn pair(first: char, second: char) -> Self;
}

/// Longest-match scan over Appendix A's key structure, yielding each member with its
/// byte range in the original text.
///
/// This is not text segmentation (ADR-0003): it is the appendix's own key shape, which no
/// other library knows. ICU4X grapheme clusters would fold the fourteen kana pairs
/// correctly and the tone-bar pairs incorrectly, so it cannot be delegated.
///
/// JLReq: §A
pub fn members(text: &str) -> Members<'_>;

/// Fold a compatibility code point onto the code point Appendix A keys, reporting the
/// frame that code point itself asserts.
///
/// Appendix A's preamble states that it lists `U+0028` while real Japanese text uses
/// `U+FF08`, so a library that did not fold would give wrong classes on ordinary text,
/// silently. Only the Wide and Narrow decomposition mapping is used: full compatibility
/// folding would fold `U+2160`, a genuine cl-19 member, onto `I`.
///
/// A caller who passed `U+FF08` has thereby stated the frame is full-width; if they also
/// declared [`Frame::Proportional`] that is a diagnostic, not a silent choice.
///
/// JLReq: §A preamble
pub fn fold_compatibility(c: char) -> Option<(char, Frame)>;
```

### The two streams — M0

`Text` lives here rather than in `jlreq-unit` because its validity is a statement about
Appendix A, and a constructor that cannot read the table it checks against is a constructor
that documents its invariant instead of holding it
([ADR 0018](../adr/0018-an-item-is-one-occurrence.md)).

```rust
/// Text, its items, and its scale table: the single carrier of what the caller knows
/// about **one running-text stream**.
///
/// A stream is one string in reading order. A paragraph is one; each annotation — each
/// ruby run's reading, each interlinear reference mark — is an [`Annotation`] (ADR-0016).
/// Tate-chu-yoko, warichu, furiwake and the ornamented complex are *not* annotations:
/// their characters are running text and live in the stream they read in.
///
/// Deliberately not called a *run*: JLReq spends that word on construct instances — "two
/// adjacent characters of the same ornamented character complex (cl-21) run" — and
/// [`RunId`] is that sense. Nothing else in this workspace may take the word.
///
/// Construction validates six things, and the last three are why this type is here. Byte
/// offsets are strictly increasing, land on character boundaries, and stay in range, so no
/// downstream slice can panic — this is a `no_std` library that may run in an interrupt
/// handler. Every item names a declared scale, and the table is non-empty and no longer
/// than [`Carry::SIZES`]. Every item is exactly one Appendix A key, with the one exception
/// [`Item`] states. And every item whose key Appendix A names under any of the five classes
/// of §3.1.2 declares a frame, because there the frame decides a geometry and an unstated
/// geometry has no answer to report.
///
/// JLReq: §A, §3.1.2, ADR-0002, ADR-0016, ADR-0018
pub struct Text<'t> { /* private */ }

impl<'t> Text<'t> {
    pub fn new(text: &'t str, items: &'t [Item], scales: &'t [Scale])
        -> Result<Self, TextError>;
    pub fn as_str(self) -> &'t str;
    pub fn items(self) -> &'t [Item];
    pub fn scales(self) -> &'t [Scale];
    /// The size of one item: its [`Scale`] together with the ordinal [`Carry`] keys on.
    /// The only source of a [`Size`], which is what makes the per-size exactness claim of
    /// ADR-0007 hold by construction rather than by discipline.
    pub fn size_of(self, item: ItemIndex) -> Size;
    pub fn size(self, id: ScaleId) -> Option<Size>;
    /// The cluster text of one item.
    pub fn cluster(self, item: ItemIndex) -> &'t str;
}

/// One annotation stream: the same shape as a [`Text`], indexed by a different ordinal.
///
/// Ruby readings and interlinear reference marks are annotations. The type exists so that
/// a base range and an annotation range cannot be swapped — in a call, or inside a
/// [`RubyRun`] — which the previous revision left to field order and a comment (ADR-0016).
///
/// Validated by exactly the same routine as [`Text`], because annotation characters are
/// classified too: ruby text has its own classes and its own boundaries. There is
/// deliberately no conversion in either direction, since one would reinstate the confusion
/// the two types exist to prevent.
///
/// JLReq: §3.3.1–§3.3.8, §4.2.3, ADR-0016, ADR-0018
pub struct Annotation<'a> { /* private */ }

impl<'a> Annotation<'a> {
    pub fn new(text: &'a str, items: &'a [Item], scales: &'a [Scale])
        -> Result<Self, TextError>;
    pub fn as_str(self) -> &'a str;
    pub fn items(self) -> &'a [Item];
    pub fn scales(self) -> &'a [Scale];
    /// The ruby em, and the only statement of it: §3.3.8's overhang allowances and
    /// §3.3.6's distribution both read this, so they cannot disagree about what one is
    /// (ADR-0019).
    pub fn size_of(self, item: AnnotationIndex) -> Size;
    pub fn size(self, id: ScaleId) -> Option<Size>;
    pub fn cluster(self, item: AnnotationIndex) -> &'a str;
}

/// An ordinal into one [`Annotation`]'s items. See [`ItemIndex`], which it is deliberately
/// not.
pub struct AnnotationIndex(u32);

pub enum TextError {
    /// Offsets are not strictly increasing.
    OffsetsNotMonotonic { at: ItemIndex },
    /// An offset does not land on a character boundary.
    OffsetNotOnBoundary { at: ItemIndex },
    /// An offset lies outside the text.
    OffsetOutOfRange { at: ItemIndex },
    /// An item names a scale the table does not have.
    UnknownScale { at: ItemIndex },
    /// The scale table is empty, or longer than [`Carry::SIZES`].
    ScaleCount { declared: usize },
    /// An Appendix A key begins inside this item and ends in the next. Merge them: the
    /// pair is one occurrence and no matrix is indexed inside one. JLReq: §A
    MemberCrossesItem { at: ItemIndex, key: Member },
    /// This item covers several Appendix A keys and is not a Western ligature — that is,
    /// it does not declare [`Frame::Proportional`] with every key in cl-27. JLReq: §A,
    /// §3.2.6
    ItemCoversSeveralMembers { at: ItemIndex, keys: u16 },
    /// The frame is unstated on an item Appendix A names under one of the five classes
    /// whose advance §3.1.2 states as half-width. There is no answer to report instead:
    /// on the ideographic frame the conditional space is inside the supplied advance and
    /// on every other it is added, and a default would put a half-em guess at the
    /// commonest adjacency in Japanese. JLReq: §3.1.2
    FrameRequired { at: ItemIndex, class: Class },
}
```

### Classification — M0

```rust
/// The answer to "what class is this occurrence".
///
/// JLReq: §3.9.2, §A
pub enum Classified {
    /// One class survives. [`Answer::why`] names the rule that decided it.
    One(Answer<Class>),
    /// Appendix A names several and the supplied facts do not separate them. The axes
    /// that would are named, as a set rather than singly: §3.9.2's own irreducible
    /// example needs frame *and* role, and `U+0031` needs frame *and* construct.
    Several { candidates: ClassSet, needs: AxisSet },
    /// Appendix A names several and nothing can separate them. §3.9.2 concedes the case
    /// — "エディター（editor）は……" — and states a preference rather than a rule, so
    /// [`Question::AMBIGUOUS_CONTEXT`] decides it and [`resolve`] applies that choice.
    Irreducible { candidates: ClassSet, why: Provenance },
    /// The member is in no Appendix A table — most of Unicode. §3.9.2 records that
    /// JIS X 4051 leaves this implementation-defined and that JLReq inherits it, so the
    /// answer is a published reading marked [`Standing::Unstated`].
    Unlisted,
}

impl Classified {
    /// Frozen projection (ADR-0012): whether the supplied facts decided this. `true` for
    /// every variant but [`Classified::One`], and a new variant recording a further
    /// reason the facts did not decide keeps the answer `true`.
    pub const fn is_ambiguous(self) -> bool;
}

/// Which caller-supplied axes would separate the surviving candidates.
///
/// Exactly three, and there is no fourth. §3.7.4's in-line and independent-line settings
/// look like a fourth and are not: they change the *spacing* between cl-17 or cl-18 and
/// its neighbors, never which class a member is in, so they belong to
/// [`ConstructKind::MathFormula`] and to an override predicate, not here.
pub struct AxisSet(u8);   // Frame, Role, Construct

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
pub enum Subject { Class(Class), Member(Member), Pair(Member, Member) }

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
/// [`Question::RELAXATION_MECHANISM`]; this project's reading is recorded, because §C.3
/// defines its strictest level by reference to the §C.2 notes, which implies the level
/// selector drives them.
///
/// JLReq: §C.2#1–#3, §B.2#14–#16, §E.2#1–#3, §C.3
pub struct Reclassification { /* subject: Subject, to: Class, when: Choice, rule: RuleId */ }

/// Resolve the class of one item.
///
/// Total over the items of a `Text`, because [`Text::new`] has already refused every
/// stream whose items are not one Appendix A key each (ADR-0018). There is no
/// "misaligned" answer, because there is no misaligned input.
///
/// JLReq: §3.9.2, §A, §3.2.4, §3.2.6, §C.2#1–#3
pub fn classify(text: Text<'_>, index: ItemIndex, policy: Policy) -> Classified;

/// The annotation twin. One implementation, two ordinal types: ruby text is classified by
/// the same tables and the same axes, and §3.3 gives it boundaries of its own.
///
/// JLReq: §3.9.2, §A, §3.3.1
pub fn classify_annotation(annotation: Annotation<'_>, index: AnnotationIndex,
                           policy: Policy) -> Classified;

/// The total variant, for callers that must have an answer.
///
/// Applies [`Question::UNLISTED_CODE_POINT`] to an unlisted member and
/// [`Question::AMBIGUOUS_CONTEXT`] to a residual ambiguity, and is defined as `classify`
/// followed by exactly that one step — so there is one classification implementation,
/// not two. The answer's provenance records which of the two applied, so a caller can
/// tell a decided class from a policy tie-break.
///
/// JLReq: §3.9.2
pub fn resolve(text: Text<'_>, index: ItemIndex, policy: Policy) -> Answer<Class>;

/// Whether the writing system uses this member in one direction only.
///
/// A validity fact, never a class selector: for all twelve code points carrying a
/// writing-mode Remark the class ambiguity is resolved by frame or role, so the direction
/// never selects a class. `jlreq::diagnose` checks it; composition does not.
///
/// JLReq: §3.1.1, §A Remarks
pub fn usage(member: Member) -> Usage;

pub enum Usage {
    Both,
    /// e.g. `U+002E` as a full stop — §3.1.1's three horizontal conventions.
    HorizontalOnly,
    /// e.g. `U+301D` 〝, which §3.1.1 says is "exclusively used for vertical writing
    /// mode and not to be used in horizontal writing mode".
    VerticalOnly,
    /// Horizontal, except that §3.1.1 permits `‘ ’ “ ”` in vertical writing when
    /// Western characters are rotated. The restriction is conditional on a rotation
    /// policy the caller owns.
    HorizontalOrRotatedWestern,
}
```

## `jlreq-spacing`

Appendices B, C, D and E: all six matrices, the roughly forty appendix notes as data, and
one evaluator. Allocation-free; the tables are `static` arrays and a boundary is `Copy`.

The crate keeps its name and broadens in scope. The six tables are one coupled rule
system — §E.1's legend defines a blank by reference to Table 2's answer, and adopting
reduction Table 5 mutates a Table 6 cell — so a crate boundary between them would run
through the middle of one evaluator.

```text
src/
  lib.rs                re-exports; crate docs
  axis.rs               Before, After
  space.rs              ConditionalSpace, Referent, Reduction, Expansion,
                        ReductionStage, ExpansionStage
  boundary.rs           Boundary, Breakable, Placement, Spacing, Delegation
  evaluate.rs           Adjacency, boundary(), rules_fired()
  generated/
    table1.rs .. table6.rs    the matrices  (transcribed → generated, M1/M2)
    notes.rs                  the overrides (generated, M1)
```

### Table axes — M1

The three appendices have three shapes and the types say so, so an out-of-range query is a
compile error rather than a phantom cell. Appendix B's legend gives Table 1 a last row
labeled "line head" and a last column labeled "line end" — one row and one column, not a
symmetric axis. Appendix C's legend names no line-edge axis. Appendix E states outright
that "there are no cells involving line head or line end".

```rust
/// A preceding position: a class, or the line head. JLReq: §B.1
pub enum Before { Class(Class), LineHead }
/// A trailing position: a class, or the line end. There is deliberately no
/// `After::LineHead`. JLReq: §B.1
pub enum After { Class(Class), LineEnd }

// Shapes:  Table 1  31 × 31   (§B)
//          Table 2  30 × 30   (§C)
//          Tables 3, 4, 5  31 × 31 each   (§D)
//          Table 6  30 × 30   (§E)
```

### The conditional space — M1/M2

```rust
/// One conditional space: one neighbor's contribution to the space at a boundary.
///
/// This and not the cell is the unit of spacing data (ADR-0014). §B.2 note 3 makes the
/// space between two middle dots "the sum of a quarter em of the preceding middle dots
/// and a quarter em of the trailing middle dots", and §D.2 note 3 then gives those two
/// components different reduction priorities in the same table. A cell holding one
/// number cannot state that.
///
/// JLReq: §B.1, §B.2#3, §B.2#5, §D.2#3
pub struct ConditionalSpace { /* private */ }

impl ConditionalSpace {
    /// The amount, as a fraction of the referent's em. Not confined to Table 1's
    /// tokens: §3.1.6 requires a full em after a sentence-final cl-04. JLReq: §B.1
    pub const fn amount(self) -> Em;
    /// Whose em, and equivalently which character this space accompanies. Appendix B
    /// writes these `be` and `af`. JLReq: §B.1
    pub const fn referent(self) -> Referent;
    pub const fn reduction(self) -> Reduction;
    pub const fn expansion(self) -> Expansion;
    pub const fn rule(self) -> RuleId;
    /// Resolve to the caller's unit against the two neighbors' sizes. Selects the
    /// referent's [`Size`] and calls [`Em::resolve_inline`], which is the workspace's only
    /// bridge from a writing-system fraction to a caller-unit length — there is
    /// deliberately no second path that would round differently and make a case's
    /// boundary answer disagree with its placements.
    ///
    /// The remainder belongs to the referent's size and is never named here: the [`Size`]
    /// the referent selects carries the ordinal, and [`Carry`] is keyed by it, so the
    /// question "which size does this remainder belong to" has no wrong answer available
    /// (ADR-0007, ADR-0019).
    pub fn resolve(self, before: Size, after: Size, carry: &mut Carry) -> InlineExtent;
}

/// Which of the two adjacent characters' ems an amount is a fraction of.
///
/// Appendix B writes these `be` and `af`, and its legend explains why they must be
/// distinguished: "there are cases where a line is composed with different sizes of
/// characters, where it is necessary to disambiguate which em size we are referring to."
/// Every note that assigns a conditional space assigns owner and referent together —
/// "the conditional half em space accompanying the preceding comma" — so this is one
/// concept and one field.
///
/// JLReq: §B.1
pub enum Referent { Preceding, Trailing }

/// JLReq: §D.1, §3.1.9
pub enum Reduction {
    /// A bare `1/2` or `1/4` in Appendix D: fixed, not reducible.
    Rigid,
    /// `1/2–0`, `1/2–1/4`, `1/4–1/8`: continuously reducible to a floor.
    Range { floor: Em, stage: ReductionStage },
    /// `1/2=0`: the full amount or the floor, nothing between. §3.1.9 says twice that
    /// at the line end "the possibilities are only half em spacing or solid. Other
    /// spacing, such as quarter em spacing should not be used." A single continuous
    /// notion of shrink emits the value the specification forbids.
    Discrete { floor: Em, stage: ReductionStage },
}

/// JLReq: §E.1, §3.8.4
pub enum Expansion {
    None,
    /// `1/4–1/2` and `1/4`: expandable to a ceiling at a stage.
    Range { ceiling: Em, stage: ExpansionStage },
    /// §3.8.4 step (d): no upper limit, and §E's fourth step re-levels across all
    /// stages rather than filling its own. A kind, not a magnitude (ADR-0010).
    Residual,
}

/// A priority stage in one of Appendix D's three reduction tables. Six steps (§3.8.3).
///
/// Distinct from [`ExpansionStage`] because the two ladders are two orderings of two
/// different things and §3.8.2 orders the ladders themselves absolutely. One shared
/// ordinal type would let "stage 2" mean two things in one report and in one published
/// case field (ADR-0014).
///
/// Stage 1 is the Western word space and lies outside the tables — Appendix D says it
/// covers "the second and subsequent stages".
///
/// JLReq: §D.1, §3.8.3
pub struct ReductionStage(u8);

/// A priority stage in Appendix E. Four steps (§3.8.4), the last unbounded.
/// JLReq: §E.1, §3.8.4
pub struct ExpansionStage(u8);
```

### The boundary — M1/M2

```rust
/// Everything the six tables say about one adjacency.
///
/// JLReq: §B, §C, §D, §E
pub struct Boundary { /* private */ }

impl Boundary {
    /// The conditional spaces here, in order. At most two (ADR-0014), which `xtask attest`
    /// checks against the captured tables rather than trusting.
    pub fn spaces(self) -> impl Iterator<Item = ConditionalSpace>;
    /// Whether a line may end here.
    pub const fn breakable(self) -> Answer<Breakable>;
    /// Whether this adjacency may occur at all.
    pub const fn placement(self) -> Answer<Placement>;
    /// How far ruby may extend here, before line adjustment caps it.
    pub const fn ruby_overhang(self) -> RubyOverhang;
    /// Where the same-run answer is a procedure rather than a value.
    pub const fn delegation(self) -> Option<Delegation>;
    /// Frozen projection (ADR-0012).
    pub const fn is_breakable(self) -> bool;
    /// Frozen projection (ADR-0012).
    pub const fn is_permitted(self) -> bool;
}

/// Whether a line may end between two items.
///
/// A hard constraint: no `Ord`, no arithmetic, and no conversion to a number, so no
/// expression turns a prohibition into a cost (ADR-0010).
///
/// JLReq: §C.1, §3.1.7, §3.1.8
pub enum Breakable { Yes, No { rule: RuleId } }

/// Whether an adjacency may occur.
///
/// The tables' `×`. The English legend is vague; the Japanese is decisive and says the
/// placement is prohibited by 行頭禁則, 行末禁則, or another rule — so this is the kinsoku
/// prohibition restated at a line edge. It is policy-dependent, and it is an outcome the
/// composer must avoid, not an assertion that the caller's text is malformed.
///
/// JLReq: §B.1, §C.1, §D.1, §E.1 legends
pub enum Placement { Permitted, Forbidden { rule: RuleId } }

/// A same-run answer that is a procedure rather than a value.
///
/// §B.2 notes 9 through 11 say to set two adjacent characters of one complex "according
/// to the method explained in §3.7.1 / §3.3.5 / §3.3.6 / §3.3.7". The boundary names the
/// procedure and stops there; `jlreq-inline::place` runs it. The variant exists so the
/// table states what the specification states instead of inventing a number, and so the
/// three delegation targets are reachable — §B.2 note 9's is §3.7.1, which is why the
/// ornamented character complex has a declaration of its own.
///
/// JLReq: §B.2#9, §B.2#10, §B.2#11, §3.7.1
pub struct Delegation { pub rule: RuleId }
```

### The evaluator — M1

One function and one ordered override list. The evaluator holds no specification knowledge:
every fact is in the tables or the overrides, which is what
[ARCHITECTURE.md](../../ARCHITECTURE.md) requires and what lets a conformance case address
a rule.

```rust
/// Everything an override's predicate can ask about an adjacency.
///
/// Constructed from two items of a [`Text`] and the [`Runs`] overlaying it, never by hand,
/// so it cannot disagree with the text it came from and there is exactly one carrier of
/// run identity. [`Runs::none`] is total, so plain text needs no second path.
pub struct Adjacency<'r> { /* private */ }

impl<'r> Adjacency<'r> {
    pub fn between(text: Text<'r>, runs: Runs<'r>, before: ItemIndex,
                   direction: Direction) -> Self;
    pub fn at_line_head(text: Text<'r>, runs: Runs<'r>, first: ItemIndex,
                        direction: Direction) -> Self;
    pub fn at_line_end(text: Text<'r>, runs: Runs<'r>, last: ItemIndex,
                       direction: Direction) -> Self;
}

/// The predicate forms an override may take. Closed, and derived from the notes rather
/// than assumed: a note that no form covers is a build failure in the generator, so the
/// claim that this set is complete is checked rather than asserted.
pub enum Predicate {
    /// §C.2 note 5: only identical marks are inseparable.
    SameMember,
    /// §C.2 note 5's five ordered adjacencies; §C.3's ellipsis pair.
    MemberPair(Member, Member),
    /// §B.2#9–#11, §C.2#6–#8, §C.2#13.
    SameRun(ConstructKind),
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
    /// §3.7.4 states four spacings for cl-17 and cl-18 against cl-21, cl-24 and cl-27:
    /// two for a formula in running text and two for one set on a line of its own. The
    /// class pair is the same in all four, so the setting has to be a predicate.
    InFormula(FormulaSetting),
    /// A policy question is set a particular way; also how §E.1's cross-table coupling
    /// is expressed — adopting reduction Table 5 makes a Table 6 quarter em rigid.
    PolicyIs(Question, Choice),
    /// §C.3's level relaxations, whose subject is a class *or* a member *or* an ordered
    /// pair. Uses [`jlreq_class::Subject`], which reclassification keys on as well.
    Relaxes(Subject),
}

/// Everything about one boundary, in one call.
/// JLReq: §B, §C, §D, §E
pub fn boundary(a: Adjacency<'_>, policy: Policy) -> Boundary;
//
// Two notes that look like they need a form of their own and do not, recorded here so
// nobody adds one. §C.2 note 4 permits the full em after a sentence-final cl-04 to be
// written as a cl-14 ideographic space character instead: when the document already
// contains that character the space is already there, so the note is an `AtEdge`-free
// override of §3.1.6 predicated on `HasClass(Trailing, {cl-14})`, and adding the amount
// as well would double it. §3.1.10 item 12's "a unit of furiwake is handled as one
// object" is `SameRun(Furiwake)`, identical in form to the five other same-run
// indivisibility notes.

/// Which rules fired at one boundary. Drives the exercised-coverage gate.
///
/// Every layer that can fire a rule exposes this shape, and the facade unions them:
/// [`Classified`] and [`Answer`] carry theirs as provenance, [`Composition::rules_fired`]
/// reports the adjustment ladder's, and [`Contribution::rules_fired`] reports the
/// constructs'. No trace object is threaded through the no-alloc crates.
pub fn rules_fired(a: Adjacency<'_>, policy: Policy) -> impl Iterator<Item = RuleId>;
```

## `jlreq-line`

Line composition. Uses `alloc`.

```text
src/
  lib.rs        re-exports; crate docs
  feasible.rs   Candidate, CandidateIndex, Feasible, FeasibleBreak
  ladder.rs     Ladder, Site, Adjustment, adjust()
  objective.rs  Badness, Demerits, Preference, Fit, Deepest
  compose.rs    Paragraph, Composition, Line, Trim, Violation, Search, compose()
  segment.rs    the nested composition of a Segment  (§3.2.5, §3.4.2, §3.7.2, §3.7.3)
  align.rs      Alignment, align()    (§3.5.3, §3.7.3)
  generated/
    figures.rs  the arrangements §3.4.3 and §3.7.2 publish only as images
                (captured → generated, M4)
```

### Break candidates — M1

```rust
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
pub enum Candidate {
    /// A plain opportunity at a byte offset.
    At(ByteOffset),
    /// §C.2 note 12: "In order to break a line in the middle of a Western word, it needs
    /// to be divided into two syllables first. Then a line can be broken between the two
    /// by adding HYPHEN at the line end." Taking this break inserts a glyph and
    /// lengthens the line, so the caller supplies its advance.
    /// JLReq: §C.2#12, §3.2.6
    Discretionary { at: ByteOffset, pre_break: InlineExtent },
}

/// An ordinal into the caller's own candidate slice.
///
/// ADR-0003 says kumihan may only remove opportunities, never add one. That is held by
/// this type rather than asserted in prose: a feasible break stores the ordinal of the
/// candidate it came from, so a break that is not one of the caller's candidates has no
/// representation and the subset property needs no test.
pub struct CandidateIndex(u32);

/// Breaks that kinsoku permits.
///
/// No public constructor, on this type or on [`FeasibleBreak`]: only
/// [`Feasible::compute`] can build one, so the optimizer cannot be handed a prohibited
/// break even by a caller who wants to (ADR-0010). Both are listed under
/// `[[no_public_constructor]]`, because naming only the set would have left a later
/// constructor on the break free to defeat the gate.
pub struct Feasible<'r> { /* private */ }

impl<'r> Feasible<'r> {
    /// [`Runs`] is a parameter rather than a field of the items, so the same-run
    /// refusals of §C.2 notes 6 through 8 and 13 are decided here, in the crate that
    /// owns break refusal, and appear in [`Feasible::rejected`] with their rule like
    /// every other refusal (ADR-0015).
    ///
    /// The `×` of Tables 1 through 6 is refused here too, and is not a separate
    /// mechanism. Its Japanese legend says the placement is prohibited by 行頭禁則,
    /// 行末禁則 or another rule, which is the kinsoku prohibition restated at a line edge,
    /// so a candidate whose resulting line edge would produce one is rejected with that
    /// cell as its rule. [`ViolationKind::ForbiddenPlacement`] is what remains for the
    /// case where *no* feasible break exists and composition must still emit lines
    /// (ADR-0010): one concept, two appearances, and which one a case exercises is
    /// decided by whether an alternative existed.
    /// JLReq: §B.1, §C.1, §D.1, §E.1 legends
    pub fn compute(text: Text<'r>, runs: Runs<'r>, candidates: &'r [Candidate],
                   policy: Policy, direction: Direction) -> Self;
    pub fn breaks(&self) -> &[FeasibleBreak];
    /// Candidates that were refused, each with the rule that refused it. A caller can
    /// see why its opportunity disappeared instead of guessing.
    pub fn rejected(&self) -> &[(CandidateIndex, RuleId)];
}

pub struct FeasibleBreak {
    /* candidate: CandidateIndex, at: ItemIndex, pre_break: InlineExtent,
       why: Provenance */
}
```

### The adjustment ladder — M1/M2

```rust
/// A line's flexibility, as an ordered ladder rather than a stretch/shrink pair.
///
/// This is the single most common way to get Japanese line adjustment wrong. TeX has one
/// proportional glue; JLReq has ordered stages — six for reduction (§3.8.3, Appendix D)
/// and four for expansion (§3.8.4, Appendix E) — drained in order and *equally within a
/// stage*, where "equally" means equal as a fraction of each site's own em, not equal in
/// absolute units. On a line mixing base-size and ruby-size runs those differ, and no test
/// of uniform-size text reveals it.
///
/// That reading is an adjudication, recorded as such: §3.8.3's steps 4 through 6 say
/// "reduced equally in proportion to the character size", but its step 1 says "the same
/// width reduction is applied to all spaces on the target line at the same time" in
/// English against 文字サイズ比で均等に in Japanese. This project follows the Japanese, and
/// the divergence is a recorded defect with both readings in a conformance case.
///
/// **Hanging punctuation (ぶら下げ) is a stage of this ladder**, between the reduction
/// stages and the expansion stages, and not a repair applied after a break is chosen. That
/// is where the specification puts it. §2.5.1 says it "is only necessary for full stops
/// (cl-06) and commas (cl-07) when they would otherwise need to be wrapped to the line
/// head" and adds that "if possible the full stops or commas are placed at the line end",
/// so a line that fits without hanging does not hang; §3.8.2's note says it is used "in
/// order to avoid the addition of inter character spacing", so a line that would otherwise
/// expand hangs first. Putting it here rather than in [`Feasible::compute`] is what makes
/// conformance.md's cross-search agreement gate satisfiable rather than aspirational: the
/// greedy and the optimal search share one ladder and one fit, so they cannot disagree
/// about when a character hangs.
///
/// JLReq: §3.8.2, §3.8.3, §3.8.4, §2.5.1, §D, §E
pub struct Ladder { /* private */ }

/// One adjustable site: one [`ConditionalSpace`] at one boundary.
pub struct Site { /* private */ }

/// What was done to a line. Deterministic and replayable.
pub struct Adjustment { /* private */ }

impl Adjustment {
    /// The realized amount at each site, so ruby overhang can be capped by what
    /// survived (§3.3.8 rule 3) rather than by the nominal amount.
    pub fn per_site(&self) -> &[InlineExtent];
    pub fn reduced(&self) -> &[InlineExtent];
    pub fn expanded(&self) -> &[InlineExtent];
    /// §E: "When the 4th step is needed, evenly add space to equalize the spacing of
    /// 1st, 2nd, 3rd and 4th steps." A re-leveling, not another bucket.
    pub const fn releveled(&self) -> bool;
}
```

### The objective — M3

```rust
/// How badly stretched or squeezed one line is: TeX's quantity, in exact integers.
///
/// The optimizer's only tuning knob, and the only quantity of the objective a caller
/// constructs. It is bounded at [`Badness::WORST`], a value ordinary lines reach, so it
/// is a cap and not a sentinel — a line that *cannot* be set is [`Fit::Infeasible`] and
/// never a large badness (ADR-0010).
///
/// JLReq: n/a (adjustment quality)
pub struct Badness(u32);

impl Badness {
    pub const ZERO: Self;
    /// `10_000`, TeX's cap.
    pub const WORST: Self;
    /// `Badness` is the one quantity of the objective with a numeric constructor and a
    /// numeric accessor, and that is deliberate rather than an omission from
    /// `[[no_impl]]`. It is an input, it is bounded, its cap is a value ordinary lines
    /// reach, and no prohibition is ever expressed in it — what the denylist protects is
    /// the type that answers "may a line end here" (ADR-0010).
    pub const fn new(value: u32) -> Self;   // clamped to WORST
    pub const fn get(self) -> u32;

    /// `min(WORST, floor(100 × (residual/flex)^3))`, computed with TeX's shift schedule
    /// so no cube overflows. Exact and bit-identical on every target; a conformance case
    /// tabulates the boundary values.
    ///
    /// This is one of the handful of items in `docs/scalar-sites.toml`, because a ratio of
    /// two inline extents is a raw quantity and the axis types have no division
    /// (ADR-0011).
    ///
    /// Total, and the zero-flex case is defined rather than left to divide: a rigid line
    /// with no residual is [`Badness::ZERO`], and a rigid line with a residual is
    /// [`Badness::WORST`]. The second value is never reached in practice because
    /// [`Fit`] classifies that line infeasible first, and it is defined anyway so the
    /// function has no precondition and no division by zero — which under
    /// `clippy::arithmetic_side_effects` would be a build error the moment it was
    /// written.
    pub const fn of(residual: InlineExtent, flex: InlineExtent) -> Self;
}

/// How good a feasible line is.
///
/// Components add independently and saturating, and are compared by [`Preference`]. There
/// is no value meaning "impossible": infeasibility is [`Fit::Infeasible`], which carries
/// evidence (ADR-0010).
///
/// This is an output only. It has no literal form and appears in no input position, so
/// the knob a caller turns ([`Badness`]) and the verdict the library returns cannot be
/// confused for one another.
///
/// Reduction and expansion depth are separate components because §3.8.2 orders the two
/// ladders absolutely — "only when there is no spacing that can be reduced is line
/// adjustment by inter-character spacing expansion applied" — and merging them would let
/// a little expansion outrank more reduction.
///
/// JLReq: §3.8.2, §3.8.3, §3.8.4, §C.3 closing paragraph
pub struct Demerits {
    /// Widow adjustment (§3.5.4) and other structural penalties.
    pub structural: u32,
    /// Lines that reached the unbounded last expansion stage.
    pub last_resort: u32,
    /// Summed expansion stage depth.
    pub expansion_depth: u32,
    /// Summed reduction stage depth.
    pub reduction_depth: u32,
    /// Summed [`Badness`].
    pub badness: u32,
    /// Hanging punctuation used, as the last tiebreak.
    pub hanging: u32,
}

impl Demerits {
    pub const ZERO: Self;
    pub const fn add_sat(self, rhs: Self) -> Self;
}

/// How two `Demerits` compare: a permutation of the six components, applied
/// lexicographically.
///
/// `Demerits` deliberately implements neither `PartialOrd` nor `Ord`, because a derived
/// order would advertise as the specification's a permutation the specification only
/// partly states.
///
/// **One relation is normative and every permutation holds it fixed.** §3.8.2: "Normally
/// line adjustment by inter-character spacing reduction is preferred. Only when there is
/// no spacing that can be reduced is line adjustment by inter-character spacing expansion
/// applied." §3.1.12's worked example applies exactly that to a choice between two breaks:
/// the opening bracket at the line end is ideally avoided by reclaiming a full em so the
/// next line's first character moves up (追い込み), and only because that reduction is
/// impossible is the bracket pushed down and the line expanded (追い出し). Ranking
/// `expansion_depth` before `reduction_depth` reproduces both sentences, so no choice of
/// [`Question::ADJUSTMENT_PREFERENCE`] reorders that pair.
///
/// **Where the other four sit is a silence.** §C.3's closing paragraph — "the very strict
/// rule is for the best appearance at the line head, while the strict rule is best to
/// avoid inter-character spacing adjustment" — is guidance on choosing a *level*, not a
/// rule for ranking two candidate paragraphs. Their placement is published in
/// `docs/decisions/adjustment-preference.toml` with [`Standing::Unstated`], and a
/// conformance case pins each of the two permutations:
///
/// - `least-adjustment`, the [`Policy::JLREQ`] value and the declaration order of the
///   struct above: `structural`, `last_resort`, `expansion_depth`, `reduction_depth`,
///   `badness`, `hanging`. It minimizes how deep into the ladders any line goes.
/// - `even-texture`: `structural`, `last_resort`, `badness`, `expansion_depth`,
///   `reduction_depth`, `hanging`. It minimizes how uneven the lines look, tolerating
///   deeper but more uniform adjustment.
///
/// JLReq: §3.8.2, §3.1.12, §C.3 (silence), `decision:adjustment-preference`
pub struct Preference { /* private */ }

impl Preference {
    pub fn from_policy(policy: Policy) -> Self;
    pub fn compare(self, a: Demerits, b: Demerits) -> core::cmp::Ordering;
}

pub enum Fit {
    Feasible { demerits: Demerits, adjustment: Adjustment },
    /// Carries why and by how much, which an infinity discards.
    Infeasible { shortfall: InlineExtent, deepest: Deepest, blocking: Option<RuleId> },
}

impl Fit {
    /// Frozen projection (ADR-0012).
    pub const fn is_feasible(self) -> bool;
}

/// How far the adjustment got before giving up, and on which ladder. §3.8.2 orders the
/// two absolutely, so "stage 3" without the ladder is two different facts.
/// JLReq: §3.8.2, §D, §E
pub enum Deepest { Reduction(ReductionStage), Expansion(ExpansionStage) }
```

### Composition — M1, extended M3

```rust
/// What to compose, and against what.
pub struct Paragraph<'r> { /* private */ }

impl<'r> Paragraph<'r> {
    /// Candidates are a required argument, not a builder step: ADR-0003 makes them an
    /// input, and omitting them would leave the library either breakless or inventing
    /// breaks. `measure` is a parameter name and not an item name, which is why the
    /// `[[forbidden]]` name guard does not fire on it (ADR-0012).
    pub fn new(text: Text<'r>, candidates: &'r [Candidate], measure: InlineExtent,
               direction: Direction) -> Self;
    /// §3.5.1's paragraph line head indent.
    pub fn with_first_line_indent(self, amount: InlineExtent) -> Self;
    /// §3.5.2's line head and line end indents.
    pub fn with_indents(self, head: InlineExtent, end: InlineExtent) -> Self;
    /// §3.5.4's widow threshold.
    pub fn with_widow_threshold(self, characters: u16) -> Self;
    /// Everything the constructs contribute (ADR-0015). Omitting it composes plain text:
    /// the neutral value is [`Runs::none`] with no segments, no separations and no block
    /// demand, so this is a builder step rather than an argument and an M1 adopter is not
    /// broken when M4 fills it in.
    pub fn with_contribution(self, contribution: &'r Contribution<'r>) -> Self;
}

/// How breaks are chosen.
///
/// There is no companion "what to compose to". §3.8.1's Note records that Japanese
/// composition has no concept corresponding to ragged right, so justification is not one
/// choice among several at the paragraph level; the two processes that are not
/// justification have their own entry points — [`align`] for §3.5.3 and §3.7.3, and
/// [`Interior`] for the four nested constructs.
pub enum Search {
    /// Take the last feasible break on each line that the ladder can fit. M1.
    ///
    /// §3.1.12 ⑤ needs no lookahead and no mechanism of its own: taking the *last*
    /// feasible break is the pull-up (追い込み), taking an earlier one is the push-down
    /// (追い出し), and preferring the first is §3.8.2's "only when there is no spacing
    /// that can be reduced" applied greedily. [`Preference`] reaches the same answer by
    /// comparison, which is why the two searches agree.
    FirstFit,
    /// Minimize total demerits over the paragraph, discarding any line worse than
    /// `tolerance`. M3 — a new variant of a `#[non_exhaustive]` enum, so M1 adopters are
    /// not broken (ADR-0012).
    Optimal { tolerance: Badness },
}

/// Compose. One entry point for greedy and optimal, sharing one feasibility computation,
/// one ladder, and one fit, so there is never a second implementation to keep in sync —
/// including for the nested composition of every [`Segment`], which runs here and not in
/// the construct layer, because §3.4.3 makes a warichu's available measure a sequence
/// that only the outer search knows (ADR-0015).
///
/// Returns `Err` only for input that is not well formed. A paragraph that cannot be
/// composed within the rules returns lines together with [`Composition::violations`],
/// because every real adopter must render something and the alternative is that each of
/// them writes an emergency breaker outside JLReq and outside our record.
///
/// JLReq: §3.8, §3.2.5, §3.4.2, §3.4.3, §3.7.2, §3.7.3, §C, §D, §E
pub fn compose(paragraph: Paragraph<'_>, policy: Policy, search: Search)
    -> Result<Composition, ComposeError>;

pub struct Composition { /* private */ }

impl Composition {
    pub fn lines(&self) -> &[Line];
    pub fn demerits(&self) -> Demerits;
    /// Rules the composition could not satisfy. Empty for a conforming result.
    pub fn violations(&self) -> &[Violation];
    /// §B.2 note 14 (c) replaces `々` with the character it repeats — the one rule in the
    /// composition layer that mutates the character stream. A caller who enabled it must
    /// re-shape and compose again; the neutral value is the empty slice, so this was
    /// added without breaking anyone (ADR-0012).
    ///
    /// The lines returned alongside a rewrite are the composition of the text **as
    /// supplied**, with the `々` still at the line head, and they are not the final
    /// layout. Composing the replacement here would mean inventing its advance, which
    /// ADR-0002 forbids. So this is a two-pass contract, and it is not silent: the same
    /// composition carries a [`Violation`] for the line-head `々`, and
    /// [`Composition::is_conforming`] answers `false` while a rewrite is outstanding, so a
    /// caller that ignores this slice cannot mistake the first pass for a result.
    /// JLReq: §B.2#14
    pub fn rewrites(&self) -> &[Rewrite];
    /// Every rule this composition applied, including the adjustment ladder's, which no
    /// per-boundary answer carries. The exercised-coverage gate unions this with the
    /// other layers'.
    pub fn rules_fired(&self) -> impl Iterator<Item = RuleId> + '_;
    /// Frozen projection (ADR-0012).
    pub fn is_conforming(&self) -> bool;
}

pub struct Line { /* private */ }

impl Line {
    /// The base stream's items. Annotation streams are not on any line's ranges: a line
    /// covers the text a reader reads on it (ADR-0016).
    pub fn items(&self) -> Range<ItemIndex>;
    pub fn bytes(&self) -> Range<ByteOffset>;
    /// The caller's own glyph-box origins, **always on this line's inline axis and
    /// relative to this line's own origin**: add the advance you supplied to one of these
    /// and you have your own box. One entry per item of [`Line::items`], with no
    /// exceptions and no second coordinate system.
    ///
    /// A trimmed item's box may run past [`Line::extent`], covering the blank half of a
    /// punctuation em, and an item whose trim came off its leading side — an opening
    /// bracket set solid at the line head — receives an origin *before* the line's start.
    /// Both are correct, both are stated rather than clamped, and [`Line::trims`] is what
    /// makes them reconstructible (ADR-0017).
    ///
    /// Items inside a [`Segment`] are here too, and they are here honestly rather than by
    /// convention. Three of the four interiors run along this line's inline axis, so their
    /// items have ordinary inline origins and the sub-line they landed in supplies a
    /// block-axis offset ([`Part`]). The fourth, §3.2.5's tate-chu-yoko, sets its run "from
    /// left to right" and then centers "the whole string" on the vertical line — so every
    /// item of it shares the segment's inline position, which is what appears here, and
    /// what distinguishes them is where they sit *across* the line, which is
    /// [`Part::across`] on the block axis. Putting an interior's own axis into this slice
    /// would have made one type mean two axes, which is precisely what ADR-0011 exists to
    /// prevent, and it would have made the published case format ambiguous for every case
    /// containing a segment.
    /// JLReq: §3.2.5, §3.4.2, §3.7.2, §3.7.3
    pub fn placements(&self) -> &[InlineOffset];
    /// From the line-head origin to the line end, in normalized geometry: including
    /// [`Line::trailing`] and excluding anything [`Line::hanging`] placed outside the
    /// measure. This is the quantity compared against the measure, and it is what a
    /// conformance case's `extent` means (ADR-0017).
    pub fn extent(&self) -> InlineExtent;
    /// The realized conditional space at the line end, whether or not it lives inside
    /// the last item's supplied advance. Defining it this way is what makes the two
    /// frames of §3.1.2 produce byte-identical expectations (ADR-0017).
    /// JLReq: §3.1.9, §B.2#2
    pub fn trailing(&self) -> InlineExtent;
    /// Every unit composition took out of a caller-supplied advance, with the rule that
    /// states it. Sparse: only items whose declared frame already contained a
    /// conditional space appear. A renderer needs nothing here to draw; a renderer that
    /// wants the normalized cell for a highlight or an underline reconstructs it from
    /// these (ADR-0002, ADR-0017).
    /// JLReq: §3.1.2
    pub fn trims(&self) -> &[Trim];
    /// The sub-lines of every [`Segment`] that touches this line.
    ///
    /// A segment straddling a main-line break (§3.4.3) appears on both lines, each
    /// carrying the parts that landed there, which is the whole output half of the
    /// straddle: the arrangement JLReq publishes only in Figures 148 and 149 is stated
    /// here as data and captured as such (ADR-0009, ADR-0015).
    /// JLReq: §3.2.5, §3.4.2, §3.4.3, §3.7.2, §3.7.3
    pub fn parts(&self) -> &[Part<'_>];
    pub fn adjustment(&self) -> &Adjustment;
    /// Ruby overhang allowances, capped by the space that survived adjustment.
    /// `jlreq-inline` places annotations against these rather than predicting them.
    ///
    /// Indexed by boundary ordinal within the line, so a line of `n` items has `n + 1`
    /// entries: index 0 is the line head, index `k` is after item `k - 1`, index `n` is
    /// the line end. The line-head entry is where §B.2 note 8's permission to overhang
    /// the paragraph's first-line indent lives — the indent is not a character and has
    /// no class, so a per-item index would have had nowhere to put it.
    /// JLReq: §3.3.8, §B.1, §B.2#8
    pub fn overhang(&self) -> &[RubyOverhang];
    /// §3.1.12 ⑤ as it happened on this line: the stated ideal response to a line-end
    /// prohibition reclaims a full em here so the next line's first character moves up
    /// (追い込み, oikomi), and only if that is impossible does one push down and expand
    /// (追い出し, oidashi).
    ///
    /// This is a report and not a mechanism, which is the whole of the decision. Pulling
    /// up is *taking the later of two feasible breaks and paying reduction*; pushing down
    /// is taking the earlier one and paying expansion; both breaks are in the feasible set
    /// already and the choice between them is [`Preference`]'s, which ranks
    /// `expansion_depth` before `reduction_depth` because §3.8.2 says so. A separate
    /// lookahead would have been a second implementation of that comparison, drifting
    /// from it.
    /// JLReq: §3.1.12, §3.8.2
    pub fn pull_up(&self) -> Option<PullUp>;
    /// A character placed outside the measure. §2.5.1 groups hanging punctuation
    /// (ぶら下げ) with tate-chu-yoko and warichu as "items jutting out of the
    /// kihonhanmen", so it is placement rather than spacing and a renderer must know to
    /// draw past the measure.
    ///
    /// Decided in the [`Ladder`], between reduction and expansion, and therefore shared by
    /// both searches. §2.5.1 says hanging "is only necessary … when they would otherwise
    /// need to be wrapped to the line head" and that "if possible the full stops or commas
    /// are placed at the line end", so a line that fits does not hang.
    /// JLReq: §3.8.2, §2.5.1
    pub fn hanging(&self) -> Option<Hanging>;
    /// Carried through opaquely and reported; never acted on, because §4.5.1 says the
    /// line gap is not changed for ruby and only the page layer knows the area edge.
    pub fn block_demand(&self) -> &[BlockDemand];
    /// Whether this line is exempt from expansion. §3.8.1's Note: the last line still
    /// takes reduction.
    pub fn is_last(&self) -> bool;
}

/// One sub-line of one [`Segment`], on this line.
///
/// [`Part::inline`] and [`Part::block`] are the sub-line's origin relative to the line's
/// own origin. Its items' inline origins are in [`Line::placements`] like every other
/// item's, in the line's own axis and never in another.
///
/// [`Part::across`] is the one thing that is not an inline quantity, and it is typed as
/// what it is. It is non-empty only for an [`Interior::Opaque`] segment — §3.2.5's
/// tate-chu-yoko, "set from left to right using solid setting" and then centered on the
/// vertical line — where the interior items are spread *across* the line rather than along
/// it. That spread is the block axis, so it is a slice of block offsets, signed and
/// straddling zero because the string is centered, and the caller maps block onto screen
/// with its own handedness exactly as it does for every other block quantity (ADR-0011).
///
/// JLReq: §3.2.5, §3.4.2, §3.4.3, §3.7.2, §3.7.3
pub struct Part<'l> { /* private */ }

impl<'l> Part<'l> {
    /// Which [`Segment`] of the contribution this is a sub-line of.
    pub fn segment(self) -> u32;
    /// Which sub-line, in reading order. A segment straddling a main-line break (§3.4.3)
    /// contributes parts to two lines and the ordinals continue across them.
    pub fn index(self) -> u8;
    pub fn items(self) -> Range<ItemIndex>;
    pub fn inline(self) -> InlineOffset;
    pub fn block(self) -> BlockOffset;
    pub fn extent(self) -> InlineExtent;
    /// One block offset per item of [`Part::items`], for an opaque interior only; empty
    /// otherwise. JLReq: §3.2.5
    pub fn across(self) -> &'l [BlockOffset];
}

/// One conditional space that was already inside a caller-supplied advance and has been
/// taken out of it.
///
/// `referent` is Appendix B's own vocabulary doing double duty, and it is the side: a
/// space owned by the *preceding* character sits after that character's glyph, a space
/// owned by the *trailing* one sits before it. A closing bracket is trimmed at its end
/// and an opening bracket at its start, which is why a trimmed opening bracket at the
/// line head is placed at a negative offset (ADR-0017).
///
/// JLReq: §3.1.2, §B.1
pub struct Trim { pub at: ItemIndex, pub amount: InlineExtent,
                  pub referent: Referent, pub rule: RuleId }

pub struct Violation { pub line: u32, pub at: ItemIndex, pub rule: RuleId,
                       pub kind: ViolationKind }
pub enum ViolationKind { Overfull(InlineExtent), ExpansionExhausted,
                         NoFeasibleBreak, ForbiddenPlacement }

pub enum ComposeError {
    /// A length exceeded the range invariant.
    OutOfRange { at: ItemIndex },
    /// A candidate lies outside the text.
    CandidateOutOfRange { at: ByteOffset },
}
```

### Single line alignment — M1

§3.8.1's Note states this is a distinct process, not a mode of line adjustment: "There is
another adjustment processing, besides line adjustment, called single line alignment."
Omitting it would leave a normative process unimplemented.

```rust
/// Align a run shorter than the target length. Used for headings and poems.
///
/// All four methods share one spacing computation and differ only in where the residual
/// goes; only [`Alignment::EvenSpacing`] consumes the §3.8 expansion opportunities.
///
/// JLReq: §3.5.3
pub enum Alignment { Centered, LineHead, LineEnd, EvenSpacing }

/// JLReq: §3.5.3, §3.7.3
pub fn align(text: Text<'_>, runs: Runs<'_>, target: InlineExtent,
             alignment: Alignment, policy: Policy, direction: Direction)
    -> Result<Line, ComposeError>;
```

§3.7.3's jidori (字取り) is [`Alignment::EvenSpacing`] against a target the caller states,
and it appears in two places for a reason rather than by duplication: a whole line set to a
length is this function, and a *span inside* a line set to a length is
[`Interior::Filled`], which runs the same computation from inside composition. Its two
stated exceptions — spacing is not added where a break is prohibited, and a single
character is set at the inline start of the block — are ordinary boundary facts and one
rule, not a second code path.

## `jlreq-inline`

Every construct the specification defines over running text or beside it. Uses `alloc`.
Lowers each of them into the four things `jlreq-line` speaks — run identity, segments,
separations, and block demand — so `jlreq-line` can be read end to end without meeting the
word "ruby", and places annotations afterwards against an allowance it is told.

```text
src/
  lib.rs        re-exports; crate docs
  lower.rs      Constructs, Lowered, Contribution, lower()
  place.rs      Attachment, Attachments, place()
  ruby.rs       Ruby, RubyStyle, RubyAlignment, RubyRun
  tcy.rs        TateChuYoko
  emphasis.rs   EmphasisDots
  warichu.rs    Warichu, WarichuDelimiters
  block.rs      Furiwake, Jidori          (§3.7.2, §3.7.3)
  mark.rs       ReferenceMark             (§4.2.3)
  ornament.rs   Ornamented, Formula       (§3.7.1, §3.7.4)
```

### Ruby — M4

```rust
/// Ruby (ルビ): a smaller reading set beside a base.
///
/// One type, not three. §3.3.7 states that a jukugo compound whose every base carries two
/// or fewer ruby characters *is* composed as mono-ruby, and §3.3.1's note says the two
/// then produce identical geometry and differ only in line-adjustment behavior. Three
/// types would have to duplicate that relationship; the shape of `runs` expresses it.
///
/// The reading is a second stream: its own string, its own items, its own size, its own
/// classes (ADR-0016). It is not a range of the annotated text, because a base character
/// and the ruby attached to it are not an adjacency any cell of Table 1 is indexed by,
/// and because the caller's break candidates were computed over the document, which does
/// not contain the reading interleaved into it.
///
/// JLReq: §3.3.1–§3.3.8, §F
pub struct Ruby<'r> { /* private */ }

impl<'r> Ruby<'r> {
    /// Takes **both** streams, which is what lets it validate both ranges. The previous
    /// revision took only the reading, so a base range could not be checked against
    /// anything and the invariant lived in prose; a swapped pair is now additionally a
    /// compile error, because a base range is a `Range<ItemIndex>` and an annotation range
    /// is a `Range<AnnotationIndex>` (ADR-0016).
    ///
    /// Validated: `base` lies inside `text`, every run's base lies inside `base`, every
    /// run's annotation lies inside `annotation`, the runs cover both in order without
    /// overlap, and the count matches what `style` requires.
    pub fn new(text: Text<'r>, base: Range<ItemIndex>, annotation: Annotation<'r>,
               runs: &'r [RubyRun], style: RubyStyle) -> Result<Self, RubyError>;
    /// Overrides [`Question::RUBY_ALIGNMENT`] for this ruby, which is the precedence rule
    /// of ADR-0019: the policy is the document's default and a per-construct statement
    /// wins for that construct.
    ///
    /// Not a `Result`, and not a `Direction` parameter. §3.3.5 says katatsuki (肩付き)
    /// "should not be adopted" for horizontal writing — a recommendation about a construct
    /// that is perfectly well defined there, unlike §3.2.5's tate-chu-yoko, which JLReq
    /// does not define horizontally at all. Refusing it at construction would publish a
    /// prohibition the specification does not state. [`Policy::JLREQ`] follows the
    /// recommendation, [`lower`] is where the direction is read, and a caller who
    /// overrides it is honored and told by `jlreq::diagnose` (ADR-0011).
    /// JLReq: §3.3.5
    pub fn with_alignment(self, alignment: RubyAlignment) -> Self;
    // There is deliberately no `with_size`. The ruby em is `annotation`'s own declared
    // `Scale`, read through `Annotation::size_of`, and it is the only statement of it —
    // §3.3.8's overhang allowances and §3.3.6's distribution both read that one value, so
    // they cannot disagree about what a ruby em is (ADR-0019).
}

pub enum RubyError {
    /// A base range lies outside the annotated stream.
    BaseOutOfRange { at: ItemIndex },
    /// An annotation range lies outside the reading.
    AnnotationOutOfRange { at: AnnotationIndex },
    /// The runs do not cover their ranges in order without overlap.
    RunsNotContiguous { at: usize },
    /// [`RubyStyle::MonoRuby`] and [`RubyStyle::JukugoRuby`] need one run per base item;
    /// [`RubyStyle::GroupRuby`] needs exactly one. JLReq: §3.3.5, §3.3.6, §3.3.7
    RunCount { expected: usize, found: usize },
}

pub enum RubyStyle {
    /// One run per base character, so two adjacent annotated bases are *different*
    /// cl-22 runs — which is what gives them §E.2 note 6's quarter-em expansion
    /// opportunity, with no special case. §3.3.1's note names the pairs: in Figure 107
    /// "the inter-character spacing between 鬼 and 門, or, 方 and 角 can be expanded".
    ///
    /// That expansion opportunity is not the same fact as the quarter em §3.3.1's other
    /// note reports between 凝 and 視. That one is *natural advance*: 凝 carries three
    /// ruby characters, §3.3.8 rule 1 forbids ruby from overhanging an adjacent cl-19
    /// character, so the bases are forced apart before composition begins — which is why
    /// the note concludes that such a line "needs some line adjustment processing"
    /// rather than that it offers some. [`lower`] emits it as extent; conflating the two
    /// composes every mono-ruby line short. JLReq: §3.3.5, §3.3.1, §3.3.8
    MonoRuby,
    /// One run over the whole base: internally unbreakable and unexpandable, from the
    /// same same-run predicate. JLReq: §3.3.6
    GroupRuby,
    /// One run per base character, but the compound is one object that may split
    /// between base characters and not inside one base character's ruby.
    /// JLReq: §3.3.7, §C.2#8
    JukugoRuby,
}

pub enum RubyAlignment {
    /// 中付き: inline-axis center alignment. Permitted in both directions.
    Nakatsuki,
    /// 肩付き: inline-start alignment. §3.3.5 says it "should not be adopted" in
    /// horizontal writing, so this is a caller choice
    /// ([`Question::RUBY_ALIGNMENT`]) whose `Policy::JLREQ` value follows the
    /// recommendation, not a hard error. JLReq: §3.3.5
    Katatsuki,
}

// There is deliberately no ruby-size type. §3.3.3 names half the base size as the
// principle and one-third ruby (三分ルビ) as a variant, and then says that for headings at
// twelve points or more the ruby "is generally smaller than half the size of the base
// characters" with no ratio given — so the set is not closed and no enumeration states it.
// The caller shaped the reading at some size and measured it there, and ADR-0002 makes that
// measurement the carrier: the ruby em is `Annotation::size_of`, full stop (ADR-0019).
// §3.3.3's anisotropy needs no type of its own either, because `Scale` has been
// anisotropic since ADR-0007 for exactly this reason: one-third ruby is one rule that
// §3.3.3 writes out twice because it speaks in physical axes — vertical gives it "the half
// of the base character in width and the one third in height", horizontal "half of the base
// characters in height and one third in width", and both are an inline third and a block
// half. A conformance case over two declared scales requires both sentences.

/// One run of ruby characters against the base characters it reads.
///
/// `base` indexes the annotated stream and `annotation` the ruby's own, and the two are
/// *different types*, so the pairing §3.3.7 and §C.2 note 8 turn on cannot be written
/// backwards: a break is permitted between two base characters of a jukugo complex and
/// never inside one base character's reading (ADR-0016).
///
/// A caller supplies these, so it can build them. The previous revision left this type
/// with private fields and no constructor, which made ruby undeclarable — the failure
/// ADR-0012's constructor check exists to catch.
///
/// JLReq: §3.3.3, §3.3.7, §C.2#8
pub struct RubyRun { /* private */ }

impl RubyRun {
    pub const fn new(base: Range<ItemIndex>, annotation: Range<AnnotationIndex>) -> Self;
    pub const fn base(self) -> Range<ItemIndex>;
    pub const fn annotation(self) -> Range<AnnotationIndex>;
}
```

### The other constructs — M4

```rust
/// Tate-chu-yoko (縦中横): a short run set across a vertical line.
///
/// Vertical writing only — JLReq defines no horizontal counterpart, so this is the one
/// construct whose *availability* depends on the direction (§3.2.5, §A.30). Once formed
/// it composes through the cl-30 row and column of all six tables like any other class,
/// so nothing downstream branches.
///
/// Note the handedness a nested-writing-mode model gets backwards: §3.2.5 sets the run
/// left to right and then centers it on the line, while vertical block progression runs
/// right to left. This is not a nested mode. It lowers to a [`Segment`] with
/// [`Interior::Opaque`]: the outer line sees one inline extent — one em at the run's own
/// size, which is what makes §3.2.5's "the inter-character spacing between cl-15, cl-16
/// or cl-19 and tate-chu-yoko is set solid" behave like any other full-width character —
/// and a block-axis jut, which §2.5.1 groups with warichu and hanging punctuation as
/// items jutting out of the kihon-hanmen.
///
/// JLReq: §3.2.5, §A.30, §C.2#13, §2.5.1
pub struct TateChuYoko { /* private */ }

impl TateChuYoko {
    pub fn new(items: Range<ItemIndex>, scale: ScaleId, direction: Direction)
        -> Result<Self, NotAvailable>;
}

/// Emphasis dots (圏点).
///
/// The one construct that carries no stream. §3.3.9 fixes both facts that would otherwise
/// be the caller's: the symbol is chosen once for the whole run, and "the character size
/// of emphasis dots is the half size of the base characters". One mark repeated at a
/// stated size is not a character string, which is also why JLReq assigns emphasis dots no
/// character class and no row in any of Tables 1 through 6 — §3.9.2's cl-21 note merely
/// observes that the JIS term 親文字群 covers ruby, ornament characters *and* emphasis
/// dots, which is not an assignment. kumihan does not invent a class: a dot run
/// contributes block demand and nothing else, and the hole is published as a conformance
/// case with [`Standing::Unstated`].
///
/// It does carry the mark's **advance**, and that is not an inconsistency with the
/// paragraph above. §3.3.9 says "the center of emphasis dots is aligned with that of the
/// base characters", and centering needs a width. §3.3.9 fixes the *size* and leaves the
/// symbol to the caller — "there are many symbols that could be specified", with SESAME
/// DOT and BULLET named only as what is used in general — and those two are not the same
/// width: one is a full-width Japanese glyph and the other is whatever a Latin font makes
/// it. Assuming one em at the dot scale would be the library computing a position from a
/// width it was never told, which ADR-0002 forbids and which the `[[forbidden]]` name
/// guard cannot see, because nothing here is called `measure` (ADR-0019). So the caller
/// supplies it, at the size §3.3.9 fixes, and nothing is assumed.
///
/// The side is not a parameter either. §3.3.9 places dots "to the right of the base
/// characters in vertical writing mode, or above them in horizontal writing mode", which
/// is block-start stated twice — the same one rule as §3.3.4's ruby side, and a
/// conformance case requires both of its sentences to come from the single value.
///
/// By convention dots are not attached to cl-01, cl-02, cl-06 or cl-07; `jlreq::diagnose`
/// reports a run that does.
///
/// JLReq: §3.3.9
pub struct EmphasisDots { /* private */ }

impl EmphasisDots {
    /// `advance` is the mark's own inline advance at the dot size, which §3.3.9 fixes at
    /// half the base. The caller shaped it and measured it (ADR-0002).
    pub fn new(base: Range<ItemIndex>, symbol: Member, advance: InlineExtent) -> Self;
}

/// Warichu (割注): an inline cutting note, two lines of small characters inside one line.
///
/// Lowered to a [`Segment`] with [`Interior::Balanced`] and [`Straddle::Permitted`], so
/// its interior is composed by `jlreq-line` inside the outer search. That is not a
/// stylistic preference: §3.4.3 wraps a warichu that does not fit onto the following main
/// line, and the Japanese note calls two-line straddling 頻出 — frequent — so the
/// interior's available measure is a sequence the outer search discovers, and no crate
/// below that search can compose it (ADR-0015).
///
/// Because the interior is a real composition, its line head and line end are real line
/// edges, so §B.2 note 13's word-space collapse and §B.2 notes 14 through 16's line-head
/// rules apply there with no special case.
///
/// JLReq: §3.4.1–§3.4.3
pub struct Warichu { /* private */ }

impl Warichu {
    pub fn new(items: Range<ItemIndex>, scale: ScaleId,
               delimiters: WarichuDelimiters) -> Self;
}

/// §3.4.2's note: bracket-delimited, or delimited by a specified amount of spacing.
pub enum WarichuDelimiters { Brackets { open: ItemIndex, close: ItemIndex },
                             Spacing(InlineExtent) }

/// Furiwake (振分け): several phrases set as stacked sub-lines inside one line.
///
/// Lowered to a [`Segment`] with [`Interior::Declared`], because §3.7.2's splits are the
/// document's — "when there are line break marks in the furiwake-gyou, the line is broken
/// in the indicated places" — and with [`Straddle::Forbidden`], because §3.7.2 says in
/// one sentence that a furiwake block "should not be extended across multiple base text
/// lines". §3.1.10 item 12 additionally makes "a unit of furiwake" one object, which is
/// the ordinary same-run indivisibility.
///
/// JLReq: §3.7.2, §3.1.10
pub struct Furiwake<'f> { /* private */ }

impl<'f> Furiwake<'f> {
    pub fn new(items: Range<ItemIndex>, scale: ScaleId, splits: &'f [ItemIndex]) -> Self;
}

/// Jidori (字取り): a run set to an explicitly specified length.
///
/// Lowered to a [`Segment`] with [`Interior::Filled`]. Shares its computation with
/// [`align`], because a whole line set to a length and a span inside a line set to a
/// length are the same operation at two scopes.
///
/// JLReq: §3.7.3
pub struct Jidori { /* private */ }

impl Jidori {
    pub fn new(items: Range<ItemIndex>, length: InlineExtent) -> Self;
}

/// A reference mark (合印), cl-20.
///
/// §4.2.3 gives two styles and they differ in exactly the way ADR-0016 turns on. In one
/// the mark is set in the line just after the target word, so its characters are running
/// text and it names a range of the stream it sits in. In the other it is set in the line
/// gap beside the target, so its characters are an annotation stream of their own.
///
/// §3.1.10 item 11 forbids a break before the mark and between its characters, which is
/// the ordinary same-run indivisibility, and §B.2 note 9 delegates the same-run case to
/// §3.7.1.
///
/// JLReq: §4.2.3, §4.2.2, §A.20, §3.1.10, §B.2#9
pub struct ReferenceMark<'m> { /* private */ }

impl<'m> ReferenceMark<'m> {
    /// Set in the line after the target word. JLReq: §4.2.3
    pub fn in_line(items: Range<ItemIndex>) -> Self;
    /// Set in the line gap beside the target word: its characters are an annotation
    /// stream, indexed by `AnnotationIndex` like every other. JLReq: §4.2.3
    pub fn interlinear(at: ItemIndex, annotation: Annotation<'m>) -> Self;
}

/// An ornamented character complex (cl-21): a base character with its superscripts and
/// subscripts (添え字).
///
/// The sub- and superscript characters are running text set at a smaller size, so this
/// names a range and carries no stream; the caller has already declared the smaller size
/// on those items. §3.7.1 states two rules and both are ordinary same-run facts: no break
/// inside the complex, and no inter-character spacing inside it used for line adjustment.
///
/// JLReq: §3.7.1, §A.21, §C.2#6, §3.1.10
pub struct Ornamented { /* private */ }

impl Ornamented {
    pub fn new(items: Range<ItemIndex>) -> Self;
}

/// A math or chemical formula.
///
/// cl-17 and cl-18 are the members' own classes and need no declaration, but §3.7.4
/// states four different spacings for the same class pairs depending on whether the
/// formula runs in the text or is set on a line of its own. That setting is a property of
/// the formula, so it is declared here and reaches the evaluator as
/// [`Predicate::InFormula`].
///
/// JLReq: §3.7.4
pub struct Formula { /* private */ }

impl Formula {
    pub fn new(items: Range<ItemIndex>, setting: FormulaSetting) -> Self;
}

/// A construct the specification does not define in this direction.
///
/// §3.2.5's tate-chu-yoko is the only one, and the earlier revision listed a second in
/// error: §3.3.5 says katatsuki "should not be adopted" in horizontal writing, which is a
/// recommendation about a construct that is well defined there, so it is a policy question
/// and a diagnostic rather than a refusal (ADR-0011).
pub struct NotAvailable { pub rule: RuleId, pub direction: Direction }
```

### Lowering and placement — M4

```rust
/// Everything the constructs contribute, in the vocabulary the line layer speaks.
///
/// Exactly four things cross the seam, and [`Paragraph::with_contribution`] is what
/// consumes them (ADR-0015). Accessors rather than public fields, so a field added later
/// is detail (ADR-0012).
pub struct Contribution<'a> { /* private */ }

impl<'a> Contribution<'a> {
    /// Per-item run identity, so the same-run predicates need no construct knowledge.
    /// The only carrier of that fact: it is deliberately not on [`Item`].
    pub fn runs(&self) -> Runs<'a>;
    /// Spans the line layer does not lay out as ordinary inline text.
    pub fn segments(&self) -> &'a [Segment<'a>];
    /// Least spacing a construct forces at a base-text boundary (§3.3.8 rule 1).
    pub fn separations(&self) -> &'a [Separation];
    /// Block-axis demand per item range, carried through and reported, never acted on.
    pub fn block_demand(&self) -> &'a [BlockDemand];
    /// Which declared construct one identity came from: the kind, and the position in the
    /// slice the caller passed. Total over every identity [`lower`] allocated.
    ///
    /// Without this a caller cannot attribute a placed annotation or an overlap error to
    /// the ruby it asked for, because it never saw the identities. The alternative —
    /// making the caller allocate them — would have created a second identity space to
    /// keep in sync with the slices it already holds, which is the defect ADR-0015 opens
    /// by refusing (ADR-0015, ADR-0019).
    pub fn construct_of(&self, run: RunId) -> ConstructRef;
    /// Every rule the construct layer applied — the whole of §3.3 lives here and reaches
    /// no boundary answer, so without this the exercised-coverage gate could never close.
    pub fn rules_fired(&self) -> impl Iterator<Item = RuleId> + '_;
}

/// Lower the declared constructs. Does **not** remove break candidates: a candidate
/// inside an indivisible construct is refused by [`Feasible::compute`] through the
/// ordinary same-run predicates, in the crate that owns break refusal, so the refusal
/// appears in [`Feasible::rejected`] with its rule like every other (ADR-0015).
///
/// This is the one place the ruby alignment question is resolved, and therefore the one
/// hand-written item in the workspace outside a construct constructor that may name a
/// variant of [`Direction`]: §3.3.5's recommendation against katatsuki is direction-
/// conditional, and `docs/direction-sites.toml` lists this item for it (ADR-0011). The
/// per-construct alignment overrides the policy's, and the answer records which applied.
pub fn lower<'a>(constructs: &Constructs<'_>, policy: Policy, direction: Direction,
                 out: &'a mut Lowered) -> Result<Contribution<'a>, LowerError>;

/// Place the annotations of one composed line against the allowances it reports.
///
/// The second half of the split that resolves the overhang fixpoint: `jlreq-line` owns
/// both halves it needs and reports the allowance per boundary (§3.3.8 rule 3, and
/// Appendix B's `hang` legend), and this places against an allowance it is told. There is
/// no edge back, and every parameter is a `jlreq-unit` type, so this crate names nothing
/// `jlreq-line` owns.
///
/// JLReq: §3.3.4–§3.3.8, §3.3.9, §4.2.3, §4.5.1
pub fn place<'a>(constructs: &Constructs<'_>, contribution: &Contribution<'_>,
                 items: Range<ItemIndex>, placements: &[InlineOffset],
                 overhang: &[RubyOverhang], policy: Policy, out: &'a mut Lowered)
    -> Attachments<'a>;

/// One annotation, placed against its base. The caller draws it with the size and the
/// offsets given.
///
/// Named `Attachment` and not `Annotation`, because [`Annotation`] is the input stream: a
/// stream of ruby characters and one placed mark are different things and one name for
/// both would be the confusion ADR-0016 is about, one level up.
pub struct Attachment { /* private */ }

impl Attachment {
    /// Which declared construct this came from, in the caller's own coordinates. Resolved
    /// through [`Contribution::construct_of`], so a caller can attribute every mark it is
    /// handed to the ruby, dot run or reference mark it asked for (ADR-0015).
    pub fn construct(self) -> ConstructRef;
    pub fn run(self) -> RunId;
    /// The size to draw at: for ruby, the annotation stream's own declared size, which is
    /// the single carrier of the ruby em (ADR-0019).
    pub fn size(self) -> Size;
    /// Block-start for ruby (§3.3.4) and emphasis dots (§3.3.9), which are one rule each
    /// stated twice in physical axes.
    pub fn side(self) -> Side;
    pub fn inline(self) -> InlineOffset;
    pub fn block(self) -> BlockOffset;
    /// The annotation stream's item, or `None` for an emphasis dot, which repeats one
    /// member rather than placing a stream (ADR-0016).
    pub fn item(self) -> Option<AnnotationIndex>;
    pub fn symbol(self) -> Option<Member>;
}
```

## `jlreq`

The facade. Re-exports the layers, owns nothing but composition of them plus one thing no
layer can own alone.

```text
src/
  lib.rs        re-exports; crate docs
  diagnose.rs   Diagnostic, diagnose()
```

```rust
/// Report what a caller's input says that is unlikely to be what they meant.
///
/// This is the answer to the question the whole API design turns on: what can an
/// integration get wrong silently? Every finding uses only data the caller supplied, and
/// no font.
///
/// Takes the constructs rather than the bare text, and the constructs carry the text
/// ([`Constructs::over`]), so the parameter count does not grow. Plain text passes
/// `&Constructs::over(text)`. The previous revision took only the text, which made three
/// of its own variants unreachable — a diagnostic about emphasis dots cannot fire without
/// seeing the emphasis dots — and adding a parameter after M1 would have been a breaking
/// change for the sake of a defect visible at M0.
///
/// JLReq: various; each diagnostic names its section.
pub fn diagnose(constructs: &Constructs<'_>, policy: Policy, direction: Direction)
    -> Diagnostics<'_>;

pub enum Diagnostic {
    /// An item's frame is unstated and its member is in more than one class. The most
    /// likely integration bug by a wide margin: unstated frames classify Latin text as
    /// cl-19, which is freely breakable, so Western words break at arbitrary letters.
    /// JLReq: §3.2.4, §3.2.6
    FrameUnstated { at: ItemIndex, candidates: ClassSet },
    /// The declared frame and the supplied advance disagree — a `Frame::FullEm` item
    /// whose advance is 0.42 em. Reported, never acted on: ADR-0002 makes the caller's
    /// advance authoritative.
    FrameContradictsAdvance { at: ItemIndex, declared: Frame },
    /// The code point the caller wrote asserts a frame of its own that contradicts the
    /// declared one — `U+FF08`, which is full-width by construction, declared
    /// [`Frame::Proportional`]. This is the diagnostic [`fold_compatibility`] promises.
    /// JLReq: §A preamble
    FoldingContradictsFrame { at: ItemIndex, declared: Frame, implied: Frame },
    /// A member `U+30FB` or `U+002E` whose role is unstated where the role changes the
    /// class or the spacing. Six code points, named individually.
    /// JLReq: §3.1.3, §B.2#12, §A.24
    RoleUnstated { at: ItemIndex, rule: RuleId },
    /// The member is not customary in this direction — `〝` used horizontally.
    /// JLReq: §3.1.1
    UnusualInDirection { at: ItemIndex, usage: Usage },
    /// Emphasis dots on cl-01, cl-02, cl-06 or cl-07, against §3.3.9's convention.
    /// Reachable because [`diagnose`] takes the constructs. JLReq: §3.3.9
    EmphasisOnPunctuation { at: ItemIndex, construct: ConstructRef },
    /// Katatsuki (肩付き) ruby alignment in horizontal writing, which §3.3.5 says "should
    /// not be adopted" — a recommendation, so the caller's choice is honored and
    /// reported rather than refused (ADR-0011, ADR-0019). JLReq: §3.3.5
    AlignmentDiscouraged { at: ItemIndex, construct: ConstructRef, rule: RuleId },
    /// The answer relies on a published reading rather than on the specification.
    Unstated { at: ItemIndex, decision: RuleId },
}
```

## `jlreq-conform`

See [conformance.md](conformance.md) for the case format, the trait, and the gates. The
crate is `std`, depends on `jlreq` and `jlreq-spec`, and has **no outside dependencies**:
it carries its own JSON reader and writer over the subset the committed schema uses.

That is a decision, not an omission, and it has three reasons. The suite is the deliverable
ADR 0006 says is worth more than the implementation, and a browser engineer running it
should not acquire a proc-macro chain to do so. The workspace's `deny.toml` sets
`bans.multiple-versions = "deny"` with empty skip and skip-tree lists, so a single
transitive duplicate anywhere becomes a standing tax on the crates that *are* published,
paid for a build-time convenience. And the parser is unusually safe to own here, because
ADR 0005 already guarantees that every number in a case is an integer inside 2^53 — the one
part of JSON that is genuinely hard is the part this format does not contain. The schema
stays committed, so nobody else has to use our reader.

## Supporting types

Named above and defined here, so nothing in this document is left to guess.

```rust
// jlreq-unit
/// The iterator [`distribute`] returns. Its items sum to the total exactly.
pub struct Distribution<'w> { /* Iterator<Item = InlineExtent> */ }

// jlreq-spec
/// Two choices JLReq makes mutually exclusive — for example
/// [`Question::KINSOKU_LEVEL`] set to Very strict, which §C.3 defines as applying no
/// §C.2 alternate rule, alongside a §C.2 alternate. Returned by [`Policy::with`], so the
/// contradictory policy is never built rather than built and checked. JLReq: §C.3
pub struct PolicyConflict { pub questions: [Question; 2], pub rule: RuleId }

// jlreq-class
/// The iterator [`members`] returns.
pub struct Members<'t> { /* Iterator<Item = (Range<ByteOffset>, Member)> */ }

// jlreq-line
/// §3.1.12 ⑤'s repair, as applied: `amount` was reclaimed in this line so the next line's
/// first item moved up. Reported by [`Line::pull_up`]. JLReq: §3.1.12
pub struct PullUp { pub amount: InlineExtent, pub pulls: ItemIndex, pub rule: RuleId }

/// A character placed outside the measure. §2.5.1 groups hanging punctuation (ぶら下げ,
/// burasage) with tate-chu-yoko and warichu as "items jutting out of the kihonhanmen",
/// so this is placement rather than spacing and a renderer must draw past the measure.
/// Only cl-06 and cl-07 are ever hung. JLReq: §3.8.2, §2.5.1
pub struct Hanging { pub item: ItemIndex, pub beyond: InlineExtent, pub rule: RuleId }

/// A required edit to the character stream. §B.2 note 14 (c) replaces `々` with the
/// character it repeats, which is the only rule in the composition layer that mutates
/// text. The caller applies it, re-shapes, and composes again — a two-pass contract,
/// because ADR-0002 makes the advances the caller's. JLReq: §B.2#14
pub struct Rewrite { pub at: ItemIndex, pub replace_with: Member, pub rule: RuleId }

// jlreq-inline
/// The constructs a caller declares over one [`Text`].
///
/// Built from [`Constructs::over`] and configured by `with_*`, with no public fields, so
/// declaring a construct kind that arrives in a later milestone is a minor release
/// (ADR-0012). The neutral value declares nothing, which is what plain text passes.
///
/// Every construct kind the specification defines has an entry here, because a kind with
/// no entry is a rule with no conformance case, and ADR 0013's coverage gate cannot close
/// over one.
pub struct Constructs<'c> { /* private */ }

impl<'c> Constructs<'c> {
    pub fn over(text: Text<'c>) -> Self;
    /// The stream these are declared over. [`diagnose`] and [`place`] read it here rather
    /// than taking it again beside them.
    pub fn text(&self) -> Text<'c>;
    pub fn with_ruby(self, ruby: &'c [Ruby<'c>]) -> Self;
    pub fn with_emphasis(self, emphasis: &'c [EmphasisDots]) -> Self;
    pub fn with_tate_chu_yoko(self, tcy: &'c [TateChuYoko]) -> Self;
    pub fn with_warichu(self, warichu: &'c [Warichu]) -> Self;
    pub fn with_furiwake(self, furiwake: &'c [Furiwake<'c>]) -> Self;
    pub fn with_jidori(self, jidori: &'c [Jidori]) -> Self;
    pub fn with_reference_marks(self, marks: &'c [ReferenceMark<'c>]) -> Self;
    pub fn with_ornaments(self, ornaments: &'c [Ornamented]) -> Self;
    pub fn with_formulae(self, formulae: &'c [Formula]) -> Self;
}

/// Reusable scratch space for [`lower`] and [`place`], so a caller composing many
/// paragraphs allocates once.
pub struct Lowered { /* private */ }

/// The iterator [`place`] returns.
pub struct Attachments<'a> { /* Iterator<Item = Attachment> */ }

pub enum LowerError {
    /// Two constructs claim overlapping items in a way the specification does not
    /// nest — §3.7.1 permits cl-21 inside cl-30 in vertical writing, and little else.
    /// Named in the caller's own coordinates, not by identities [`lower`] invented.
    OverlappingConstructs { a: ConstructRef, b: ConstructRef },
    /// A construct names an item outside the text.
    OutOfRange { at: ItemIndex, construct: ConstructRef },
    /// The construct is not defined in this direction. §3.2.5's tate-chu-yoko is the only
    /// one; §3.3.5's katatsuki is a recommendation and is honored with a diagnostic
    /// (ADR-0011).
    NotAvailable(NotAvailable),
}

// jlreq
/// The iterator [`diagnose`] returns.
pub struct Diagnostics<'r> { /* Iterator<Item = Diagnostic> */ }
```

## The frozen API file

`docs/api-frozen.toml` is written at M0 with its full contents, before the types it governs
exist. A gate authored against types that already compile is a gate written to pass them,
which inverts its purpose
([ADR 0012](../adr/0012-outcome-and-detail-compatibility.md)). It is owned by the code
owners, so every one of these lists is relaxed only by review.

```toml
# Output enums that are open, each with the total accessor whose answer set is frozen.
[[frozen]]
type = "jlreq_spacing::Boundary"
projections = ["is_breakable", "is_permitted"]
[[frozen]]
type = "jlreq_line::Fit"
projections = ["is_feasible"]
[[frozen]]
type = "jlreq_line::Composition"
projections = ["is_conforming"]
[[frozen]]
type = "jlreq_class::Classified"
projections = ["is_ambiguous"]
[[frozen]]
type = "jlreq_spec::Provenance"
projections = ["is_specified"]
[[frozen]]
type = "jlreq_line::Line"
projections = ["is_last"]

# Public types allowed to be exhaustive, each closed by the specification rather than
# by us.
[[exempt]]
type = "jlreq_class::Class"
why = "3.9.2 closes the set at thirty; a catch-all arm over character classes is where a silently wrong default hides"
[[exempt]]
type = "jlreq_spacing::Before"
why = "B.1 gives Table 1 one line-head row and nothing else"
[[exempt]]
type = "jlreq_spacing::After"
why = "B.1 gives Table 1 one line-end column and nothing else"
[[exempt]]
type = "jlreq_spacing::Referent"
why = "B.1's referent vocabulary is exactly `be` and `af`; a space has two owners"
[[exempt]]
type = "jlreq_spacing::Breakable"
why = "C.1's legend has one token for permitted and one for prohibited"
[[exempt]]
type = "jlreq_spacing::Placement"
why = "the `x` of B.1, C.1, D.1 and E.1 is one token"
[[exempt]]
type = "jlreq_unit::Direction"
why = "2.3.1 defines two writing modes; a caller mapping them onto screen axes must handle both or draw a mirrored page"
[[exempt]]
type = "jlreq_unit::Side"
why = "an axis has two ends"
[[exempt]]
type = "jlreq_unit::InlineEdge"
why = "an axis has two ends"
[[exempt]]
type = "jlreq_spec::Standing"
why = "a fifth kind of claim would be an outcome, not a detail"

# Choice sets that may never grow, for the same reason.
[[closed_choices]]
question = "kinsoku.level"
count = 4
why = "C.3 enumerates four levels"
[[closed_choices]]
question = "adjustment.reduction_table"
count = 3
why = "D publishes Tables 3, 4 and 5"
[[closed_choices]]
question = "adjustment.remainder"
count = 2
why = "jlreq_unit::RemainderRule enumerates the two readings of JLReq's silence as Rust variants, Leading and Trailing, and Policy::remainder is the one mapping from this question onto them; a third generated choice would have no variant to map to and no conformance case"

# Traits no listed type may ever implement or derive, matched on the trait's name so an
# import cannot evade the check.
[[no_impl]]
types = ["Em", "Advance", "InlineOffset", "BlockOffset", "InlineExtent", "BlockExtent",
         "Badness", "Demerits", "Breakable", "Placement", "Fit"]
traits = ["Add", "Sub", "Mul", "Div", "Rem", "Neg", "AddAssign", "SubAssign",
          "MulAssign", "DivAssign", "Ord", "PartialOrd", "From", "Into", "TryFrom",
          "TryInto", "Deref", "DerefMut", "AsRef", "AsMut", "Borrow", "BorrowMut"]
why = "ADR-0010: no expression may turn a prohibition into a number. ADR-0011: two conversions in either direction let a value round-trip between the axes in two well-typed steps."

# Types with no public constructor beyond a named factory.
[[no_public_constructor]]
type = "jlreq_line::Feasible"
factory = "compute"
why = "ADR-0010: a forbidden break is absent from the search space, not expensive within it"
[[no_public_constructor]]
type = "jlreq_line::FeasibleBreak"
factory = ""
why = "ADR-0010: naming only the set would leave a later `FeasibleBreak::new` free to hand the optimizer a prohibited break while the gate still passed"

# Raw-integer construction and read-back on the axis types, permitted only here. Every
# other site is a gate failure. See docs/scalar-sites.toml for the allowlist itself.
[[scalar_channel]]
types = ["InlineOffset", "BlockOffset", "InlineExtent", "BlockExtent", "Advance"]
methods = ["new", "units", "get", "of", "raw"]
home = "jlreq-unit::axis, jlreq-unit::length"
allowlist = "docs/scalar-sites.toml"
why = "ADR-0011: `BlockExtent::new(inline.units())` is a cross-axis assignment in two well-typed steps, and no arrangement of types removes the pair. The channel is narrowed to a reviewed list instead of being denied. `of` and `raw` are the crate-visible half of the same pair: the four axis types hold their integer in a private field rather than a `pub(crate)` one, so that `InlineExtent(raw)` and `value.0` are not a second channel this method list cannot reach."

# Shapes that may never appear in the public surface at all.
[[forbidden]]
crates = ["jlreq-class", "jlreq-spacing", "jlreq-line", "jlreq-inline", "jlreq"]
item_names = ["measure", "width", "height", "metrics", "advance_of", "glyph_advance",
              "font", "load_font"]
matches = "declared public item identifiers only: fn, struct, enum, trait, const, static, mod, type. Never parameter names, field names, or keys in a data format."
why = "ADR-0002: the library never asks how wide a character is. A name-level guard, and it is named as one; the substantive control is that no core crate may gain a dependency. `Paragraph::new(.., measure, ..)` is a parameter and does not match."
[[forbidden]]
crates = ["jlreq-class"]
signature = "(char) -> Class"
why = "ADR-0008: no total function from a code point to a class exists. 473 of 1133 enumerated keys are multi-class and five classes enumerate nothing. This is the first thing an adopter in a hurry reaches for."
```

Two definitions the gate needs, pinned here rather than settled by whatever the code turns
out to do ([ADR 0012](../adr/0012-outcome-and-detail-compatibility.md)). A public type is in
an **input position** when it appears in the parameter list of any public function in the
workspace other than as the receiver, including inside a reference, a slice, a range, an
`Option` or a `Result`. A **named constructor** is an associated function returning `Self`,
`Result<Self, _>`, or `Option<Self>`. Applying that predicate to this document is what found
`RubyRun`, `BlockDemand`, `Answer` and `Provenance`, each of which now has one.

## Mechanical gates

New `xtask` subcommands, all `std`-only so `xtask`'s empty dependency table survives.

| Gate | What it enforces |
| --- | --- |
| `purity` (extended) | The exact crate adjacency of the table above, not merely core membership; and that every seam type appears in a producer's and a consumer's signature, so a seam with nothing on the far end fails rather than passing. Plus bare float *literals*, closing a measured hole: `let r = 0.5; r * 2.0;` passes clippy, rustc under `-D warnings`, and today's gate. The scan must strip block comments and string literals first — today's `code_only` strips only `//` — and must not fire on `self.0` tuple access or `{:.3}` format precision. |
| `ops` | The `[[no_impl]]`, `[[no_public_constructor]]` and `[[scalar_channel]]` tables of `docs/api-frozen.toml`. Traits are matched on the *name* in `impl <Trait> for <Type>` and in `#[derive(...)]`, so `use core::ops::Add;` does not evade it, and over an explicit type list so a new length type must be added to the file. The scalar half rejects any call of `new`, `units`, `get`, `of` or `raw` on an axis type or on `Advance` outside `jlreq-unit`'s own `axis` and `length` modules and outside an item listed in `docs/scalar-sites.toml` — which is what makes ADR-0011's axis separation a control rather than a claim, since the two functions are a round-trip pair through `i32` that no arrangement of types removes. |
| `placeholder` | No `todo!`, `unimplemented!`, `#[allow(`, or `#[expect(` in core sources. Measured: a body of `todo!()` produces zero diagnostics today. |
| `api` | Every public type is `#[non_exhaustive]` unless `[[exempt]]` lists it; every `[[frozen]]` projection still exists; every `#[non_exhaustive]` type in an input position has a named constructor, under the two definitions pinned above; no `[[forbidden]]` shape appears, matching declared item identifiers only. |
| `spec-links` | Every public item carries a `JLReq:` line; every address resolves; every rule cited by an item has a conformance case. |
| `direction` | Union of the rules named in `docs/direction-sites.toml` and the rules carrying a `Predicate::InDirection` row equals the set the inventory marks direction-conditional — today §3.1.3, §3.2.5, §3.3.5. A variant of `Direction` may appear in hand-written core sources only inside an allowlisted item, and in generated sources only in that predicate. Naming the *type* is unrestricted, because passing a value through a signature is not a branch. |
| `generate --check` | Byte-identical regeneration. See [generation.md](generation.md). |
| `attest` | Double entry, provenance, cross-table invariants, recorded defects. See [generation.md](generation.md). |
| `conform --check` | Schema, unique ids, declared rule coverage. See [conformance.md](conformance.md). |

Four invariants are held by the shape of the API rather than by a gate, and each is stated
here so none is mistaken for unguarded.

ADR 0003's promise that kumihan only ever *removes* break opportunities is held by
`CandidateIndex`: a feasible break stores the ordinal of the candidate it came from, so a
break that is not one of the caller's has no representation.

ADR 0016's stream scoping is held by two ordinal types: a base range and an annotation range
have different types, so the swap the previous revision described in prose is a compile
error.

ADR 0018's item contract is held by `Text::new`, which is why `Text` lives in `jlreq-class`
and not in `jlreq-unit`: a misaligned stream, or one with an unstated frame on a §3.1.2
class, has no value to reach any entry point with.

ADR 0010's policy consistency is held by `Policy::with` returning a result: a policy §C.3
makes self-contradictory is never built, so no entry point has to check for one and none can
forget to.

What ADR 0011's axis separation is held by is stated honestly rather than in that list: no
*typed* conversion exists and `[[no_impl]]` keeps it absent, but `new` and `units` are a
round-trip pair through `i32` that no arrangement of types removes, so the untyped channel is
narrowed by `docs/scalar-sites.toml` and the `ops` gate. A gate rather than a shape, named as
one.

## What must change elsewhere

Noted here, changed in the implementation phase — this phase writes documents only.

1. **`Cargo.toml`**: two new members, `crates/jlreq-unit` and `crates/jlreq-spec`.
2. **`Justfile`**: `core_crates` gains both; the `msrv` recipe gains two `--path` lines (it
   enumerates paths and does not glob); `check` and `ci` gain the new gates.
3. **`xtask/src/main.rs`**: `CORE_CRATES` becomes the adjacency table; the new subcommands
   land here. Also, the flat list should be replaced by a list derived from
   `Cargo.toml` members with an explicit non-core denylist, so the gate fails *closed*
   when a crate is added — today it silently skips an unlisted crate.
4. **`release-plz.toml`**: a `[[package]]` block per new crate with
   `version_group = "kumihan"`, and both added to `jlreq`'s `changelog_include`.
5. **`ARCHITECTURE.md`**: the crate boundary table
   ([ADR 0015](../adr/0015-the-crate-graph-and-the-inline-line-seam.md)); invariant 4's
   wording, which currently implies no direction-conditional composition logic at all
   where there are three enumerated rules
   ([ADR 0011](../adr/0011-typed-axes-and-direction-as-a-datum.md)).
6. **`CONTRIBUTING.md`**: the "generated, not transcribed" rule, amended per
   [ADR 0009](../adr/0009-generated-data-and-attested-transcription.md); and the `cl-1`
   spelling in the commit example.
7. **`crates/jlreq-class/src/lib.rs`**: the claim that a class is a property of a code
   point, corrected per
   [ADR 0008](../adr/0008-classification-is-a-function-of-an-occurrence.md); `cl-1`,
   `cl-2`, `cl-7` to `cl-01`, `cl-02`, `cl-07`.
8. **`crates/jlreq-spacing/src/lib.rs`**: "a function of the two adjacent classes and
   nothing else", which run identity, role, direction and policy all refute.
9. **`crates/jlreq-inline/src/lib.rs`**: "resolved before the line layer decides where a
   line ends rather than after", which §3.3.8 rule 3 reverses for overhang.
10. **`README.md`**: `cl-1` to `cl-01`; the crate table.
11. **`ROADMAP.md`**, in three places rather than one. M0 currently reads "determine the
    JLReq class (cl-1 … cl-30) for a code point", which is the exact total function
    [ADR 0008](../adr/0008-classification-is-a-function-of-an-occurrence.md) proves does not
    exist and `[[forbidden]]` now bans — and ROADMAP.md is the document a new contributor
    reads first, so it currently instructs them to build the one thing the design forbids.
    M3's "with hanging punctuation as an adjustment option" is right and should say it is a
    ladder stage between reduction and expansion. And M4 grows: furiwake (§3.7.2), jidori
    (§3.7.3), reference marks (§4.2.3), the ornamented complex (§3.7.1) and formulae
    (§3.7.4) all lower through the same seam as the four constructs already listed, and
    [ADR 0013](../adr/0013-rules-are-addressed-by-specification-address.md)'s coverage gate
    cannot close at M4 with five normative processes unimplemented. The growth is real and
    is cheap precisely because they share one mechanism.
12. **`CONTRIBUTING.md`**, second entry: the "core stays pure" bullet enumerates the five
    core crates by name and gains `jlreq-unit` and `jlreq-spec`, exactly as
    [ADR 0001](../adr/0001-no-std-no-io-no-font-in-core.md)'s identical list does. Also a
    line stating that with integer arithmetic a per-OS difference in `just test-ci` is a
    bug and never a tolerance.
13. **`docs/adr/0005-integer-layout-units.md`**: a Superseded-in-part note pointing at
    [ADR 0007](../adr/0007-two-scalars-and-the-fixed-point-unit.md). Likewise
    **`docs/adr/0001-no-std-no-io-no-font-in-core.md`**, whose list of core crates grows
    by two.
14. **`docs/api-frozen.toml`**, **`docs/direction-sites.toml`** and
    **`docs/scalar-sites.toml`**: all three new, each written in the same commit as the
    first `xtask` gate that reads it, and each added to `CODEOWNERS` in that commit — the
    code-owner guard is what makes them controls rather than documentation.
15. **`docs/decisions/`**: the published readings this design commits to, one file each,
    including `adjustment-preference.toml` (where the four non-ladder demerit components
    sit, which JLReq does not state — the ladder pair is §3.8.2's and is not a choice),
    `ambiguous-context.toml` (§3.9.2's conceded case), and
    `compatibility-ideographs.toml`.

No change is needed to `typos.toml` or `clippy.toml`. Both were probed: every romanized
term this design uses — warichu, jukugo, katatsuki, nakatsuki, kenten, hanmen, kihon,
furigana, tategumi, yokogumi, zenkaku, hankaku — passes `typos` with the current
configuration, and the domain proper nouns this design adds are either already in
`doc-valid-idents` or are not camel-case and do not trip `doc_markdown`. Unicode property
names such as `East_Asian_Width` are written in backticks, because they are identifiers
rather than proper nouns. `clippy.toml`'s trailing `".."` sentinel must never be removed:
it extends clippy's default list rather than replacing it.
