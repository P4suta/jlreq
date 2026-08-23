# ADR-0007: two length scalars, and the fixed-point unit is 1/720 em

- Status: accepted
- Date: 2026-08-05

## Context

[ADR 0005](0005-integer-layout-units.md) deferred the fixed-point denominator to M0 and
described a single length type in fractions of the ideographic em, with the caller
converting once at the boundary. Working the specification through, that single type turns
out to be unable to express Appendix B.

Table 1's cells are not amounts alone. Each carries a referent, written `be` or `af`, and
the legend states why: the distinction "is necessary because there are cases where a line
is composed with different sizes of characters, where it is necessary to disambiguate
which em size we are referring to." A line containing ruby, warichu (割注), or a
parenthetical set one size smaller has several ideographic ems in it at once. A type
defined as "a fraction of the ideographic em" does not name a unique quantity on such a
line, so it cannot represent a Table 1 cell.

Forcing the caller to express proportional Latin advances in units of some em is lossy in
the same place. A proportional glyph is not a fraction of anything; converting it into
1/720 em and the resulting position back out rounds twice, on data the caller already held
exactly.

The fractions the specification actually names are halves, thirds, quarters, fifths, and
eighths, whose least common multiple is 120. It names exactly two ruby scales: half the
base size, and one-third ruby (三分ルビ), whose inline extent is a third of the base.

## Decision

There are two scalar types and they never mix. `Em` is a quantity the writing system
states, in units of 1/720 of the ideographic em: 120 for the fractions JLReq names,
multiplied by 6 so that a quantity at either named ruby scale is still exact when stated
in base ems, which is what lets a conformance case be reviewed by a human. `Advance` is a
length in the caller's own unit — font units, 1/64 px, points, whatever the advances
already are — which jlreq adds, compares, and negates but never interprets. Returned
positions are in that same unit, so the caller converts nothing.

The two meet in one bridge, `Em::resolve`, which takes a `Scale` and a carried remainder.
`Scale` is one character size expressed in the caller's unit, and it is anisotropic: §3.3.3
gives one-third ruby a block extent of half the base em and an inline extent of a third, so
a single scalar per size cannot hold it. The bridge is offered once per axis rather than
once with an axis argument — `resolve_inline` yields an inline extent, `resolve_block` a
block extent — so it never produces an axis-free length that a later call site could put on
the wrong axis. Both go through one private computation, so there is still exactly one place
where an `Em` becomes a quantity in the caller's unit.

The carried remainder is per character size, and the exactness claim is stated that way. A
remainder produced against a 1000-unit em and spent against a 500-unit em is a different
absolute length, so one carry shared across a line of mixed sizes would not be exact — and a
line of mixed sizes is precisely the case this decision exists for. A run of resolutions *at
one size* therefore sums to the rounding of its exact total rather than to the sum of
roundings, and a line's total error is bounded by one unit per size the line contains rather
than by one per gap. The bookkeeping is a small fixed array indexed by the paragraph's own
scale table, which is why a text declares a bounded number of sizes; the bound is far above
the four the specification ever needs at once — base, ruby, warichu, and tate-chu-yoko.

The per-size claim is held by the signatures rather than by discipline at their call sites,
which is the correction this decision needed and did not have. A remainder is not a
parameter of anything. Both bridges and the conditional space's own resolution take a
`Size` — one character size together with its ordinal in the text's own scale table — and a
mutable carry. There is no public remainder type and no way to obtain one.

The carry keys on the *em length*, not on the ordinal. The ordinal is a proxy for the em,
and it is a caller-supplied proxy: `Size::new` has to be public, because the accessors that
ordinarily produce a `Size` live in `jlreq-class` and a seam type readable at one end and
not writable at the other is a seam with nothing on the far end
([ADR 0012](0012-outcome-and-detail-compatibility.md)). Keying on the proxy would have left
"a remainder produced against a 1000-unit em is never spent against a 500-unit em" true only
of callers who pair the two correctly, which is the discipline-at-call-sites failure
[ADR 0019](0019-one-fact-one-carrier.md) exists to remove. Keying on the em makes it hold by
construction for every `Size` a caller can build, and two declared sizes that share an em
length correctly share one remainder: the same absolute length is the same rounding. The
bound on the number of *em lengths per axis* belongs to the carry, and the text validates
against it.

That the constant is the right one is also checked rather than asserted. 720 was chosen so
that every quantity the specification names is exact in it, and `xtask attest` requires every
amount in every generated and captured table to be an exact multiple of 1/720 — so a future
appendix note naming a thirty-second would fail the build instead of rounding quietly.
(720 = 2^4 · 3^2 · 5, so a sixteenth *is* exact, at 45 units; the first power of two that is
not is a thirty-second, and the first small odd denominators that are not are a seventh and
an eleventh.)

Both scalars are `i32`, signed because reduction deltas and hanging punctuation are
naturally signed, and both are bounded by 2^30 − 1. That bound is the overflow argument:
two valid values sum to less than `i32::MAX`, so a single addition cannot wrap, and
saturation can only ever report a breach of the bound rather than hide a machine wrap.

No denominator makes distribution exact, because §3.8.3 divides equally across a
text-dependent number of sites and §3.3.6 divides by twice the ruby count. Exactness comes
from a remainder-preserving distribution primitive whose parts sum to the total, not from
the constant.

## Consequences

This supersedes ADR 0005's paragraph on lengths while keeping its substance: the
arithmetic is still integer, still bit-identical on every target, and still exactly
comparable in a conformance case. What changes is that only one of the two scalars is in
fractions of the em, and the rounding is ours and stated rather than the caller's and
implicit.

720 is a one-way door. Every generated table, every conformance case, and every published
expected value is expressed in it.
