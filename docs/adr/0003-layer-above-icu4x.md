# ADR-0003: compose above ICU4X and HarfRust rather than replacing them

- Status: accepted
- Date: 2026-08-05

## Context

Japanese line composition needs break opportunities and shaped advances. Both already
exist in Rust and are maintained by people with more resources than this project:
ICU4X implements UAX #14 line breaking, HarfRust and rustybuzz implement shaping, and
Parley composes them into rich text layout.

What none of them does is the Japanese layer. UAX #14 says where a break is *permitted*;
JLReq says which of those permitted breaks are *acceptable*, how much the surrounding
punctuation may be compressed to avoid an unacceptable one, and what to do when no
acceptable break exists. A downstream project using ICU4X today gets `、` at the start of a
line — not because ICU4X is wrong, but because that question is out of its scope.

An engine that reimplemented break discovery and shaping in order to own the whole
pipeline would be a competitor to Parley rather than something Parley could adopt, and it
would be answering a question that is already answered.

## Decision

jlreq consumes break opportunities and advances and produces placement. It does not
implement UAX #14, does not shape, does not do bidi, and does not rasterize.

The public surface is shaped so an adopter can keep its existing stack: the input is text
plus advances plus candidate break positions, the output is line boxes and spacing
adjustments. Nothing in the API requires the caller to have obtained those inputs from any
particular library.

## Consequences

The addressable adopter set is everyone who already has a text stack — Typst, Parley,
cosmic-text, PDF writers, game engines — rather than only projects willing to replace one.
Adoption is additive.

We inherit ICU4X's and HarfRust's correctness on the parts they own, and their bugs. That
is the right trade: those are the best-tested implementations available, and duplicating
them would mean owning the duplicate forever.

The boundary must be honest. If a JLReq rule turns out to require information that only the
shaper has, the answer is to take it as an additional input, not to start shaping.
