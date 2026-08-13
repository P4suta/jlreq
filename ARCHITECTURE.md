# Architecture

This document describes the 1.0 target and the invariants enforced during migration. The
older `jlreq-*` decomposition remains only as unpublished specification data,
implementation, and regression assets.

## Boundary in the text stack

```text
font I/O + shaping + UAX #14 + bidi              renderer / PDF / game engine
                 │                                           ▲
                 └── shaped clusters + break opportunities ──┤
                                                             │
                         kumihan ───── placements + diagnostics
```

`kumihan` never loads a font, shapes a glyph, discovers a Unicode break opportunity,
resolves paragraph bidi, or draws. The caller supplies those answers. The library owns
Japanese normalization, spacing, construct behavior, line choice, and physical placement.

## Product boundary

There are two public contracts:

1. `kumihan`, a dependency-free Edition 2024 library with MSRV 1.85 and `#![no_std]` plus
   `alloc`;
2. `kumihan-conformance`, a binary-only product that communicates with any engine through
   the versioned NDJSON protocol.

`xtask` is repository tooling. The eight old crates are `publish = false` migration assets;
the new library does not depend on any of them. Publication stays disabled until the
release criteria in `ROADMAP.md` hold.

## Logical pipeline

The implementation is one directional pipeline:

```text
model → specification/style → normalize/rules → construct → compose → place → API views
```

The present source groups some adjacent stages into files, but ownership follows this
direction:

- `model` owns caller-unit sizes, frames, writing modes, shaped clusters, and source bytes;
- `style` owns the 22 typed decisions and dated profiles;
- normalization joins Appendix A two-code-point keys without losing original cluster
  attribution;
- private classification and spacing select specification behavior;
- `construct` validates and lowers the nine document structures;
- composition performs one whole-paragraph dynamic-programming search;
- placement creates renderer-ready base and attachment views;
- the crate root re-exports only names listed in `docs/api-1.0.toml`.

No classification, seam, adjustment stage, feasibility score, ladder, badness, or rule ID is
public. A caller builds `ShapedText`, validates a `Paragraph`, calls `compose`, and draws the
returned views; it never wires internal stages together.

## Input invariants

All public scalar geometry is bounded `i32` in the caller's unit. Intermediate sums and
costs use checked or saturating `i64`; floating point is rejected by the purity gate. The
private specification unit remains 1/720 em where specification fractions are needed.

Every public source coordinate is a UTF-8 byte offset or range. `ShapedText::new` verifies:

- exact, ordered, non-overlapping source coverage;
- UTF-8 boundaries;
- non-negative shaped advances;
- valid per-cluster sizes and explicit metric frames; and
- the distinction between a proportional ligature and a non-Latin cluster hiding multiple
  Appendix A keys.

Appendix A two-code-point keys may arrive split across shaping clusters. Normalization makes
the internal key indivisible while placements still point back to the original shaped
clusters. Breaks and constructs cannot split such a key.

`ParagraphBuilder` jointly validates the line extent, indent, break kinds, nested/disjoint
construct ranges, ruby runs, tab stops, widow policy, alignment, and writing mode. Once a
`Paragraph` exists, `compose` is infallible. Unsatisfied fit and quality constraints are
represented by placements plus stable diagnostics, not by a late error.

## Composition and placement

The only search policy is whole-paragraph optimization. Mandatory breaks partition the
search; discretionary breaks carry a cost; ordinary break opportunities are filtered by
Japanese line-start and line-end rules. First-fit, feasibility objects, ladders, badness,
and tuning knobs are implementation details.

Tabs take part in measurement and placement rather than being a second line API. Their
alignment is expressed on the logical inline axis, so the same stops work in horizontal and
vertical writing.

`Layout`, `Line`, `ClusterPlacement`, and `Attachment` have private fields and read-only
accessors. A placement identifies the original cluster or construct ordinal, byte range,
logical inline/block coordinates, advance, size, frame, local writing mode, and transform.
This is sufficient to draw vertical proportional glyphs and tate-chu-yoko without
reshaping.

## Style compatibility

Every alternative in `spec/derived/questions.tsv` maps one-to-one to a dedicated
`#[non_exhaustive]` enum in `kumihan::style`. `StyleBuilder::build` rejects contradictory
combinations. Generic string settings, public `Question`/`Choice` values, and internal rule
IDs are excluded.

`Style::default()` is permanently `Style::jlreq_2020()`. A future JLReq revision adds a new
dated profile; it does not alter an existing profile. `docs/api-1.0.toml` maps all 22 enum
names and counts back to generated specification data, and the API gate compares that
mapping in both directions.

## Diagnostics and conformance

An `InputError` means the caller supplied an invalid representation. A `Diagnostic` means a
validated paragraph was placed but a stable observable condition should be reported. Only
the diagnostic code, severity, input range, and JLReq reference are contracted; private
rule sequences and adjustment steps are not.

The process protocol deliberately describes only inputs and observable outputs. It never
asks an external implementation to expose classes, internal seams, or algorithm stages.
The protocol and specification identifiers are mandatory on every message so an old engine
cannot accidentally be judged against a new suite.

## Mechanical enforcement

- `purity`: no `std`, I/O, font dependency, floating point, or undeclared dependency edge
  in the core;
- `api`: open public types, constructibility, forbidden shapes, exact 1.0 exports, and the
  22 typed Style mappings;
- `placeholder`: no unwritten body or lint suppression in core code;
- `derive`, `generate`, `attest`: reproducible specification derivation and transcription
  provenance;
- `spec-links`, `direction`, `conform`: legacy rule attribution and differential assets
  remain intact while their behavior moves behind the new black-box API;
- normal Rust tests: invalid input, all style choices, all constructs in both directions,
  paragraph search, placement, and protocol behavior.

The old `docs/api-frozen.toml` remains temporarily as a control over migration assets used
by `ops` and legacy tests. It is not the public 1.0 contract; it is removed together with
those crates after differential migration is complete.
