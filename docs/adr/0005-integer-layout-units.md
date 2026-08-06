# ADR-0005: layout arithmetic is integer

- Status: accepted
- Superseded in part by [ADR 0007](0007-two-scalars-and-the-fixed-point-unit.md): the
  single length type in fractions of the ideographic em becomes two scalars that never
  mix, and the denominator this ADR defers is settled at 1/720 em. The integer decision
  itself stands unchanged.
- Date: 2026-08-05

## Context

Line composition is arithmetic on lengths: accumulate advances, subtract compressible
space, compare against the line measure. The obvious representation is `f32` or `f64`.

Floating point makes three things worse here.

Comparison against the line measure becomes approximate. "Does this character fit" is the
central question of line breaking, and answering it with a tolerance means the answer can
differ between two callers who supplied the same input, or between two targets.

Golden tests stop being exact. A conformance suite whose expected values need an epsilon
cannot serve as evidence for other implementations, because a disagreement inside the
epsilon is unresolvable.

Accumulated error is real. Japanese line adjustment distributes and reclaims fractional
space across every character in a line; a paragraph is a long chain of additions and
subtractions where rounding compounds.

There is also a hard constraint: [ADR 0001](0001-no-std-no-io-no-font-in-core.md) requires
the core to build for `thumbv7em-none-eabi`, which has no hardware floating-point unit in
its base variant.

TeX solved this in 1978 with the scaled point, and its output is bit-identical across four
decades of platforms as a direct result.

## Decision

All layout arithmetic in the core is integer. No `f32` or `f64` appears in `jlreq-unit`,
`jlreq-spec`, `jlreq-class`, `jlreq-spacing`, `jlreq-line`, `jlreq-inline`, or `jlreq` —
enforced by `just purity`, which scans the core sources and fails on a float type or a
float cast.

Lengths are a fixed-point type in fractions of the ideographic em. Callers converting from
a floating-point font pipeline convert once, at the boundary, where the rounding is
visible and theirs.

Overflow behavior is stated rather than inherited: the workspace lints deny
`clippy::arithmetic_side_effects`, so every arithmetic site declares whether it saturates,
checks, or is proven not to overflow.

## Consequences

The same input produces bit-identical output on every target, every optimization level,
and every Rust version. A conformance case can assert an exact number, which is what makes
the suite usable as evidence against another implementation
([ADR 0006](0006-conformance-suite-as-artifact.md)).

Callers with floating-point advances must convert. The precision they lose is bounded and
under their control, and it is less than the error they would accumulate over a paragraph
of floating-point composition.

Choosing the fixed-point denominator is a one-way door and belongs to M0, not M3. It must
divide the ideographic em evenly by the quarters, thirds, and halves that JLReq spacing
rules actually use, with headroom for a paragraph's worth of accumulated adjustment.
