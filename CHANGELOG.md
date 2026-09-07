# Changelog

All notable user-facing changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). Detailed pre-release development
chronology is archived in [DEVELOPMENT-HISTORY.md](DEVELOPMENT-HISTORY.md).

## [Unreleased]

These are the completed 0.1.0 release-candidate notes. The publication date is deliberately
unset. No crate upload, tag, or GitHub Release has occurred.

### Added

- `jlreq`, a high-level MSRV 1.88 Rust facade that turns UTF-8 text and in-memory
  TTF/OTF/TTC data into renderer-independent visual-order glyph placements.
- `FontLibrary`, one-shot `layout`, reusable `LayoutEngine`, typed `LayoutOptions` with
  the JLReq composition-policy profiles (`Style::jlreq_2020` and the four published
  alternatives, re-exported at the crate root), explicit family/style/fallback
  configuration, family names derived from the font's own `name` table, OpenType features
  and variations, and an opt-in `system-fonts` feature.
- Grapheme and variation-sequence fallback, UAX #9 bidi, HarfRust shaping, UAX #14 line
  opportunities, paragraph preservation, tabs with explicit `TabStop`s and all four
  alignments including decimal-point `Character` stops, first-line indents, `Widow`
  policy, horizontal/vertical physical placement, Latin rotation, and tate-chu-yoko.
- `DocumentBuilder` spans, per-range `ParagraphStyle` overrides (measure, alignment,
  policy, indent, widow, tab stops), mandatory/discretionary/prohibited break controls,
  mono/group/jukugo ruby, emphasis, warichu with automatically balanced furawake
  sublines, jidori, reference marks, position-honoring superscript/subscript scripts, and
  formulae; annotation strings are shaped automatically, span roles cover decimal points,
  digit-group separators, and dividing marks with suppressible inference, and
  `MetricsFrame` asserts the virtual body. Finished documents read everything back.
- `TextLayout`, `TextLine`, and `GlyphPlacement` with provenance-checked sparse-ID font
  lookup, original UTF-8 ranges, per-glyph construct attribution, line and paragraph
  metadata, draw origins, resolved sizes and variations, em-relative `FontMetrics`, 26.6
  geometry, transforms, bidi levels, and whitespace/annotation-preserving cell bounds.
- Affinity-exact hit testing and carets at wraps, paragraph breaks, bidi boundaries, and
  empty lines; visual and line-to-line caret motion; grapheme, dictionary-backed word,
  and sentence segmentation; visually contiguous selection rectangles that do not bridge
  unselected bidi runs, plus a filled per-line variant.
- `FontSynthesis` and renderer-visible system-font default axes, synthetic emboldening,
  and skew state.
- Typed high-level errors carrying stable codes, messages, and ranges, positioned
  missing-glyph and unknown-family diagnostics, configurable limits for input/font bytes,
  fonts, paragraphs, runs, glyphs, constructs, and core operations, and atomic failure
  with engine reuse.
- Runnable teaching examples for every documented capability, and a repository gate that
  compiles and executes the README and guide examples against a fixture font.
- `jlreq-conformance`, the binary-only protocol-v1 runner and sample engine, plus
  independent OCaml and Racket engines and generated three-way censuses.
- Reproducible specification generation, API/semver controls, coverage, mutation, fuzz,
  CodeQL, dependency/license, isolated offline package, SBOM, checksum, and provenance
  gates.

### Changed

- The dependency-free pre-shaped composition library is now `jlreq-core` (MSRV 1.85,
  `no_std + alloc`); the same low-level surface is also accessible as `jlreq::core`.
- Public-library responsibilities are separated from the conformance process contract.
  Fontique 0.11.1, HarfRust 0.13.3, ICU4X segmenter 2.3.0, and unicode-bidi 0.3.18 are
  facade implementation dependencies and do not enter the core.
- Variation settings now use deterministic 26.6 equality/hashing and merge global,
  system-selected, and span values by tag with the last layer winning. Paragraphs advance
  by actual line/annotation cells plus one line gap, including large styled spans.
- Mutation-ledger validation now runs through the cross-platform Rust `xtask`; pull-request
  mutation smoke is split into four deterministic shards. Benchmarks now cover variable
  font state, multi-paragraph annotations, and bidi editing queries.
- Release packaging produces and offline-verifies three crate archives. Future publication
  order is `jlreq-core`, registry visibility, then `jlreq` and
  `jlreq-conformance`.

### Security

- Malformed fonts and arbitrary text are explicit threat-model inputs. Finite-range checks
  precede deterministic 26.6 quantization, and bounded work returns no partial result.
- The conformance runner bounds messages, suites, case counts, stderr retention, inactivity,
  and child-process cleanup.

[Unreleased]: https://github.com/P4suta/jlreq/compare/v0.1.0...HEAD
