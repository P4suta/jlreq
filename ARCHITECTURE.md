# Architecture

This document describes the prepared 0.1.0 implementation and the boundaries enforced by
the repository gates.

## Three product layers

```text
UTF-8 + explicit font bytes
            │
            ▼
 jlreq (MSRV 1.88)
 graphemes · fallback · bidi · shaping · line opportunities
            │ validated pre-shaped clusters and constructs
            ▼
 jlreq-core (MSRV 1.85, no_std + alloc, no dependencies)
 JLReq classification · kinsoku · spacing · adjustment · composition
            │
            ▼
 TextLayout: visual-order glyphs + owned fonts + diagnostics
            │
            ▼
 caller renderer / PDF writer / game engine

 jlreq-conformance ── protocol-v1 black-box validation of any engine
```

The layers have different users and different compatibility surfaces:

1. `jlreq` is the high-level facade. It accepts text and font resources, hides Fontique,
   HarfRust, ICU4X, and unicode-bidi types, and returns renderer-independent physical glyph
   placement.
2. `jlreq-core` is the deterministic composition engine for callers that already own
   shaping and Unicode preprocessing. Its former public items and result model remain
   available under their new crate path and through `jlreq::core`.
3. `jlreq-conformance` is binary-only. It validates implementations through the existing
   `jlreq.conformance/1` NDJSON protocol and never becomes an in-process dependency of
   either library.

`xtask`, `fuzz/`, and the independent implementations under `engines/` are repository
tooling rather than products.

## High-level processing order

The facade performs these steps in a fixed order:

1. Split paragraphs while preserving every original UTF-8 byte range, including CRLF and
   Unicode paragraph separators.
2. Itemize extended graphemes and script, then choose the first fallback face that covers
   the whole grapheme or variation sequence.
3. Resolve paragraph bidi with UAX #9.
4. Shape font/script/direction runs with HarfRust and map glyph clusters back to source
   ranges.
5. Merge UAX #14 opportunities, mandatory breaks, tabs, and explicitly prohibited breaks.
6. Lower typed inline structures and ask `jlreq-core` to apply kinsoku, punctuation
   spacing, line adjustment, and exact composition.
7. Reorder each completed line visually and convert logical placement into horizontal or
   vertical physical coordinates.

Automatic semantic classification is deliberately conservative: obvious decimal points,
digit separators, and sentence punctuation may be classified, but units, quantities,
formulae, ruby, and other authored meaning require `DocumentBuilder`.

`layout` constructs a fresh engine for one call. `LayoutEngine` reuses parsed fonts,
shaper state, Unicode services, and core scratch allocation. Cache state is never part of
an answer, so the two paths are bit-identical and an engine remains reusable after a typed
failure.

## Font and shaping boundary

`FontLibrary` owns immutable font bytes and registers a TTF/OTF face or an indexed TTC
face with family and style metadata. Its primary face is both the first candidate and the
source of `.notdef`; an explicit fallback order controls all later candidates. Fallback is
performed on a complete extended grapheme. If no face covers it, the grapheme and source
range remain present, a primary `.notdef` is shaped, and a positioned
`font.missing-glyph` diagnostic is returned.

Fontique-backed OS discovery is behind the disabled-by-default `system-fonts` feature.
Layout is cross-platform deterministic when the same explicit bytes, face indices, text,
and options are supplied. An OS collection may change or choose a different face and is
therefore outside that guarantee.

OpenType feature tags and variation coordinates are facade values. No upstream crate type
crosses the public API.

## Documents and results

`DocumentBuilder` validates UTF-8 ranges, non-overlapping spans, explicit break controls,
and nine typed inline structures: ruby (mono, group, and jukugo), tate-chu-yoko, emphasis
dots, warichu, furawake, jidori, reference marks, scripts, and formulae. Annotation strings
are shaped by the same font, fallback, and bidi machinery as body text.

`TextLayout` owns its source, lines, diagnostics, and every `FontResource` used by its
glyphs. Each `GlyphPlacement` has visual draw order, font and glyph IDs, original UTF-8
range, physical origin, advance, offset, transform, and bidi level. `hit_test`,
`caret_rect`, and `selection_rects` use that same geometry for horizontal, vertical,
and mixed-direction text.

The facade does not rasterize, draw, allocate GPU resources, or serialize a document.

## Numeric and resource invariants

Floating-point input is accepted only at public convenience boundaries. NaN, infinity,
negative values where forbidden, and values outside the representable range are rejected.
Valid values are immediately rounded to signed 26.6 fixed point. All shaping, composition,
result comparison, and physical conversion use those quantized values.

`ResourceLimits` independently bounds input bytes, font count, total font bytes,
paragraphs, shaping runs, glyphs, constructs, and core operations. The core additionally
bounds clusters, break candidates, constructs, tab stops, and charged exact-search
transitions. A limit or validation failure returns no partial layout and mutates no
caller-owned object. Scratch and cache state remain valid for the next engine call.

## Core module direction

The dependency-free core retains its one-way implementation pipeline:

```text
model/style/limits → spec → normalize/rules → construct
                   → paragraph → compose/place → layout → public API
```

All core source coordinates are UTF-8 byte offsets or ranges; geometry is bounded integer
caller units. `ShapedText` validates ordered complete source coverage, cluster boundaries,
advances, sizes, and metric frames. `ParagraphBuilder` jointly validates line extent,
breaks, nested/disjoint constructs, ruby associations, tab stops, widow policy, alignment,
and writing mode. Composition returns a complete exact `Layout` or a typed error. It
never switches to approximate first-fit behavior.

The 22 JLReq 2020 policy choices remain dedicated enums under
`jlreq_core::style` (also `jlreq::core::style`). `Style::default()` remains identical
to `Style::jlreq_2020()`; a future specification revision adds a dated profile instead of
changing an existing one.

## Conformance and specification identity

The process protocol describes only observable input and output, not internal classes or
algorithm stages. Every message carries `jlreq.conformance/1` and
`jlreq-2020-08-11+unicode-17.0.0`. Moving the Rust core did not change either identifier.
The conformance inventory remains 100 covered rules out of 106, with the other six
explicitly classified in the ledger.

## Mechanical enforcement

- `purity`: `jlreq-core` has no dependencies, `std`, I/O, fonts, or floating point;
  facade and conformance dependencies are explicit and closed.
- `api` and `semver`: the facade and core 0.1.x exports match
  `docs/public-api.toml`; later patches compare both public libraries to the registry.
- `direction` and `placeholder`: core modules follow the declared graph and contain no
  unfinished body or lint suppression.
- `derive`, `generate`, and `attest`: specification data and generated core tables are
  reproducible and provenance-bound.
- `conform`: protocol schema, cases, inventory, and specification identifiers agree.
- `coverage`, `mutants`, and fuzzing: all handwritten products are exercised; generated
  exclusions and equivalent mutations are individually ledgered.
- `package`: all three archives are extracted into isolated Cargo homes after a locked
  fetch, then built, tested, documented, and installed offline.
- `repository`, REUSE, deny, CodeQL, actionlint, and zizmor hold publication state,
  licensing, dependency policy, links, workflow pinning, and security hygiene.

No gate uploads a crate, writes a tag, creates a GitHub Release, chooses a date, or stores a
credential.
