<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# ADR-0026: the facade supplies working defaults where the core demands declarations

- Status: accepted
- Date: 2026-09-01
- Builds on the layer split of [ADR 0025](0025-three-product-layers.md) and the
  facade-contains-no-rule boundary of [ADR 0015](0015-the-crate-graph-and-the-inline-line-seam.md).

## Context

Pre-publication review of 0.1.0 from a first-time consumer's perspective found three
places where the facade forwarded a core declaration requirement to application authors
who cannot reasonably meet it, and one where an identifier could silently resolve to the
wrong data:

1. `Document::furawake(range, columns, gap)` always failed with
   `input.furawake-split-count` unless the caller also supplied exactly `columns - 1`
   `mandatory_break` offsets inside the range, because the core deliberately composes
   sublines at *declared* positions (JLReq §3.7.2, as ADR 0015 records) and the facade
   suppresses automatic opportunities inside constructs. The repository's own root README
   shipped this failure.
2. `FontLibrary::register_font` recorded an empty family, so the natural pairing —
   `register_font` then `SpanStyle` with a family request — silently matched nothing and
   fell back with no diagnostic.
3. `FontResource` exposed no design metrics, so a renderer could not draw an underline or
   align baselines without parsing the font again itself.
4. `FontId` was a bare slot ordinal: an identifier minted by one `FontLibrary`, used
   against another whose slot happened to exist, resolved to the wrong font with no error.

## Decision

1. **Furawake synthesizes balanced splits.** When no caller break falls strictly inside a
   furawake range, lowering partitions the range's *shaped clusters* — not characters,
   because splits must land on cluster boundaries — into `columns` sublines of
   `count / columns` clusters, earlier sublines taking the remainder, and inserts the
   resulting cluster-start offsets as mandatory breaks. Any caller break inside the range
   disables synthesis for that construct, so explicit declarations keep the exact-count
   core contract, and ranges with fewer clusters than columns still surface the core
   error. The core contract itself is untouched: the facade computes declarations, which
   is composition of layers, not a composition rule.
2. **Families derive from the font.** Registration with an empty family reads the font's
   own `name` table — typographic family (ID 16) preferred over legacy (ID 1); Windows
   US-English, then any Windows language, then the Unicode platform, then Macintosh Roman
   restricted to ASCII — with a hand-written, total, bounds-checked reader kept inside
   the facade so the closed dependency surface (ADR 0025) is preserved. The same reader
   family supplies `FontResource::metrics` from `head`, `hhea`, `OS/2`, and `post` as
   em-relative values. A span family no face declares reports the new
   `font.unknown-family` diagnostic instead of staying silent.
3. **Identifiers carry provenance.** `FontId` holds a private per-library nonce from a
   process-global counter. Equality, ordering, hashing, and `Debug` remain slot-only,
   because the published compatibility contract promises bit-identical layouts from
   identical bytes and options across distinct libraries, and layouts embed identifiers.
   Lookups — `FontLibrary::get` and `TextLayout::font` — check the nonce, so a foreign
   identifier resolves to `None` or `font.unknown-id` rather than the wrong font.

## Alternatives considered

- *Synthesizing furawake splits by character count.* Rejected: the core validates splits
  against shaped-cluster boundaries, so character balancing produces
  `input.break-splits-cluster` on any multi-character cluster.
- *Deriving splits in the core.* Rejected: ADR 0015 places all break selection in one
  layer and keeps the facade rule-free; conversely, a core default would change observable
  behavior for existing protocol inputs, which the census pins.
- *Reporting furawake underflow (`count < columns`) at the facade.* Rejected: the core
  already reports it deterministically with the construct's range, and a duplicate facade
  code would freeze a second name for the same fact.
- *Parsing names and metrics through a new dependency.* Rejected: ADR 0025 fixes the
  dependency set, and the tables involved are small fixed layouts; the readers are total
  and fuzzed directly (`fuzz_targets/font_name_table.rs`).
- *Including the nonce in `FontId` equality.* Rejected: it would break the published
  determinism bullet in `docs/design/api-spine.md` and the cross-library equality tests
  that witness it.

## Consequences

The quickstart shape — register one font, author constructs, lay out — works without
ceremony, and the README example that used to fail at runtime is now also executed by the
`examples` gate. Derived families make `register_font` + span families coherent, at the
cost of a name-table read at registration. Cross-library misuse becomes visible instead
of silently wrong, while layouts from identical inputs remain bit-identical. The facade
gains ~200 lines of total parsing code whose failure mode is `None`, exercised by
truncation and bit-flip fault-injection tests and by a dedicated fuzz target.
