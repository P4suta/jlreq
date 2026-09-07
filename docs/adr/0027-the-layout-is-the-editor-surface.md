<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# ADR-0027: the layout is the editor surface, and paragraphs take styles

- Status: accepted
- Date: 2026-09-01
- Builds on [ADR 0025](0025-three-product-layers.md) and
  [ADR 0026](0026-facade-convenience-defaults.md).

## Context

The 0.1.0 facade advertised editing — affinity-exact hit testing, carets, selection
rectangles — but every consumer still had to reimplement caret motion, line navigation,
word selection, and construct-aware selection, and would get Japanese word boundaries
wrong without a dictionary segmenter. Several implemented core capabilities also had no
facade path at all: `Widow`, `first_line_indent`, three of the four `TabAlignment`s,
`Break::discretionary`, four `ClusterRole`s, `Frame::HalfEm`, and `ScriptPosition` — the
last a frozen public export that was stored and then discarded, so superscripts and
subscripts produced byte-identical layouts. Finally, every paragraph in a document shared
one `LayoutOptions`: no indented body, no narrower quotation, no centered heading.

## Decision

1. **`ParagraphStyle`.** `DocumentBuilder::paragraph_style(range, style)` overrides line
   extent, alignment, policy `Style`, first-line indent, widow policy, and tab stops for
   every paragraph the range fully contains. Styles must not overlap; a range that cuts a
   paragraph is rejected at layout (`document.paragraph-style-splits-paragraph`), because
   half a paragraph cannot take its own measure. `LayoutOptions` carries the document-wide
   defaults for the previously unreachable knobs, and `TabStop`/`TabAlignment`/`Widow`
   join the facade as quantized-`f32` mirrors of the core types — the same idiom as
   `Alignment` and `WritingMode`, chosen over re-export because the core's caller units
   are the facade's private 26.6 representation.
2. **Editing primitives live on `TextLayout`.** Visual caret motion, line-to-line motion,
   grapheme boundaries, and dictionary-backed word and sentence segmentation are methods
   of the layout, implemented over the same physical geometry and the same ICU4X services
   the layout itself used. Segmenter types stay private (ADR 0025); results are plain
   offsets, ranges, and the existing `HitTest`. Lines expose their index, paragraph
   ordinal, and first/last-in-paragraph flags, and every glyph reports the ordinal of the
   construct it belongs to, resolved against `Document::construct`.
3. **`ScriptPosition` becomes real in the core, compatibly.** `ConstructKind::Script`
   (private) gains a position; `Construct::script` keeps today's superscript-side meaning
   and `Construct::script_at` declares a side. Subscripts mirror to the opposite block
   side and the line reserves space independently per side it uses. Every existing
   construction path lowers to the superscript side, and the protocol cannot express a
   position, so the 122,199-case censuses observe no change.

## Deliberate exclusions

- **Serde derives.** `ARCHITECTURE.md` states the facade does not serialize a document;
  rather than reinterpret that sentence, the read-back surface (`Document::spans`,
  `paragraph_styles`, `constructs`, break lists, and full getters on every value type)
  makes consumer-side serialization complete without one. Revisit only with a maintainer
  decision that narrows the architecture sentence.
- **Pagination and column flow.** Block-axis partitioning is break selection, which ADR
  0015 places in the core, and no core capability exists yet. The line metadata added
  here (paragraph ordinals, per-line extents) is deliberately sufficient for a consumer
  to implement column flow by re-laying per region.
- **Anisotropic cluster sizes (長体/平体).** Reachable in the core via `Size::new`, but a
  facade exposure would extend the renderer draw contract with a scale channel; deferred
  until that contract change is designed on its own terms.
- **Changing the sentence-terminator inference default.** The `!`/`?` heuristic stays
  conservative; authors who need a different reading assert `TextRole::SentenceTerminator`
  or suppress classification with `TextRole::Plain`, both added here. The default is
  entangled with the character-insertion reading recorded in
  `docs/decisions/sentence-medial-dividing-mark.md` and does not move without it.

## Consequences

An editor builds on the layout without a parallel Unicode stack, and Japanese word
selection is correct by default. Documents express real book structure — headings,
indents, quotations, widow control, decimal-point tab alignment — through one builder.
The facade surface grows by five public types and roughly thirty methods, all frozen in
`docs/public-api.toml` and exercised by the integration suite, the documentation-example
gate, and the widened fuzz targets.
