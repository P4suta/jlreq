# ADR-0002: character metrics are supplied by the caller

- Status: accepted
- Date: 2026-08-05

## Context

[ADR 0001](0001-no-std-no-io-no-font-in-core.md) forbids the core from opening fonts, so
advances have to arrive from somewhere. Three shapes were considered.

A trait the caller implements (`fn advance(&self, ch: char) -> Length`) reads naturally but
puts a virtual call in the innermost loop of line composition and forces the caller to
answer per character, which is wrong for shaped text where advances belong to glyphs and
clusters, not code points.

Having the library consult a font handle reintroduces exactly the coupling ADR 0001
rejects.

Taking advances as data — a slice supplied alongside the text — matches how shaping
already works. HarfRust, ICU4X, and every renderer already have this array in hand at the
moment they would call us.

## Decision

Layout entry points take the text together with its already-measured advances. The library
never asks a question about a character it was not told the answer to.

Where a rule needs a quantity that is a property of the writing system rather than the
font — the width of an ideographic space, the amount by which a comma may be compressed —
that quantity is expressed as a fraction of the ideographic em supplied by the caller, not
as an absolute measurement the library invents.

## Consequences

The caller is responsible for shaping before calling us, which is correct: shaping is
HarfRust's job and we are not going to do it better.

Tests become fixed input to fixed output with no external state. A conformance case is a
string, an array of advances, and the expected placement — reviewable by a human who knows
JLReq but not Rust, and comparable against another implementation.

The API cannot silently degrade when a font is missing a glyph, because the API never
looked at a font. Fallback is the caller's decision, made before the advances are handed
over, and the layout result is a pure function of what it was given.
