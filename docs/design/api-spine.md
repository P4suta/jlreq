# The 0.1.0 API spine

This is the human-readable contract for the two public Rust libraries. The exact facade and
core exports, together with all 22 core Style mappings, are frozen in
[`docs/public-api.toml`](../public-api.toml) and checked in both directions by `xtask api`.

## Facade: text and fonts in, drawable glyphs out

The normal path is intentionally one call:

```rust,no_run
# fn layout(font_bytes: Vec<u8>) -> Result<(), jlreq::LayoutError> {
let mut fonts = jlreq::FontLibrary::new();
fonts.register_font(font_bytes)?;
let options = jlreq::LayoutOptions::try_new(240.0, 16.0)?;
let layout = jlreq::layout("日本語組版", &fonts, options)?;

for glyph in layout.glyphs() {
    if let Some(font) = layout.font(glyph.font_id()) {
        let _draw = (
            font.bytes(),
            font.face_index(),
            glyph.glyph_id(),
            glyph.draw_origin(),
            glyph.font_size_26_6(),
            glyph.variations(),
            font.synthesis(),
            glyph.transform(),
        );
    }
}
# Ok(())
# }
```

`LayoutOptions::try_new(line_extent, font_size)` validates and quantizes its two required
values. Consuming `with_*` setters cover writing mode, alignment, core `Style`, language,
base direction, line gap, tab width, explicit `TabStop`s, first-line indent, `Widow`
policy, OpenType features and variations, and `ResourceLimits`; every field also has a
same-named getter, and the collections have replacing `with_*s` forms whose empty
iterator clears them. Value types follow one convention — `with_*` setters, bare-name
getters — everywhere: `LayoutOptions`, `SpanStyle`, `ParagraphStyle`, and
`ResourceLimits` alike. `LayoutEngine` exposes the same plain-text and typed-document
calls with reusable internals.

Tab stops, alignment at stops (`TabAlignment`, including decimal-point `Character`
alignment), and the widow policy are facade mirrors of the core types, taking the same
quantized `f32` units as every other public length. Explicit stops replace the evenly
spaced ladder derived from the tab width.

`FontLibrary` registers owned memory fonts and TTC indices, family/style metadata, a
primary face, and ordered fallback. When no family is supplied, one is derived from the
font's own `name` table (typographic family preferred), so `register_font` plus a span
family request just works; a span family that matches nothing keeps the fallback result
and reports a `font.unknown-family` diagnostic. `FontId` equality identifies the library
slot — preserving cross-library determinism — while lookups check provenance, so an
identifier minted by a different library resolves to `None` rather than the wrong font.
Each `FontResource` also exposes em-relative design metrics (ascent, descent, line gap,
x-height, cap height, underline geometry) for renderer-side decoration; composition never
depends on them. The optional `system-fonts` feature is the only OS discovery surface. It
matches weight, width, and slant and records selected default axes plus synthetic
bold/skew in `FontResource`. HarfRust, Fontique, ICU4X, and unicode-bidi types are
private.

## Typed authored content

`DocumentBuilder` adds non-overlapping `SpanStyle` ranges, mandatory and prohibited
breaks, and all nine inline structure families. Ruby can be mono, group, or jukugo, with
automatic or explicit `RubyRun` association. The builder shapes annotation strings for
ruby, emphasis, reference marks, and scripts; callers never manufacture low-level
annotation clusters. `ScriptPosition` is honoured in placement: superscripts share the
annotation side with ruby, subscripts mirror to the opposite block side, and the line
reserves space on whichever sides it uses.

The builder validates byte boundaries and cross-field relationships before producing an
immutable `Document`. `layout_document` then returns a complete result or a typed error.

`DocumentBuilder::paragraph_style` applies a `ParagraphStyle` — optional overrides for
line extent, alignment, JLReq policy `Style`, first-line indent, widow policy, and tab
stops — to every paragraph its range fully contains, which is what makes indented body
text, a narrower quotation measure, or a centered heading possible inside one document.
Styles must not overlap, and a range that cuts a paragraph is rejected at layout: half a
paragraph cannot take its own measure.

A finished document reads back everything the builder accepted: `spans()`,
`paragraph_styles()`, `constructs()` (as borrowed `InlineConstruct` values whose ordinals
match glyph provenance), `mandatory_breaks()`, and `prohibited_breaks()`, and `SpanStyle`
exposes a getter for each of its fields. Consumers can therefore inspect, diff, or
serialize a document without a parallel data model.

## Renderer-facing results

`TextLayout` owns:

- the original UTF-8 source;
- physical `TextLine` values in reading order;
- `GlyphPlacement` values in visual draw order;
- every referenced `FontResource`; and
- positioned, stable-code diagnostics.

A glyph exposes font ID, glyph ID, source byte range, optional annotation attribution,
draw origin, advances, offsets, resolved size and variation axes, cell bounds, 26.6
geometry, transform, and bidi level. A line exposes source range, physical origin and
extents, writing mode, cell bounds, and its visual glyph slice. The layout itself reports
its `writing_mode()` (defined even when there are no lines) and retains the exact
`options()` it was produced with, so an editor can re-lay content out without keeping its
own copy. `TextLayout::font` is the required ID lookup because retained font identifiers
may be sparse.

`hit_test`, `caret_rect`, and `selection_rects` are defined over the same physical
geometry and support both writing modes and bidi. They never require the renderer to infer
logical ordering from glyph order. `caret_rect` requires `Affinity`; hit-test results can
therefore round-trip exactly at wraps, paragraph breaks, and bidi boundaries. Selection
rectangles are split at visually unselected runs.

`cell_bounds` and line/layout `bounds` are layout-cell boundaries, not outline ink
boundaries. They preserve whitespace and annotation cells. Renderers derive ink bounds
from font outlines when needed.

## Errors, diagnostics, and atomicity

`LayoutError` distinguishes invalid font data or TTC index, missing fonts, invalid
options, invalid typed document data, font-library misuse (`InvalidFontRequest`), resource
exhaustion, core input failure, and core composition failure. Its `code()` and optional
source range are stable; `message()` carries a stable one-sentence explanation where the
variant has one, and `Display` shows the message, the byte range when present, and the
code. Display prose may improve.

Invalid fonts, options, ranges, and resource limits return no `TextLayout`. Missing glyphs
and overfull or widow conditions that still permit a complete answer are diagnostics.
Failures do not poison `LayoutEngine`; the next call is independent.

All public float values are checked for finiteness and range and quantized to 26.6 fixed
point before processing. Equivalent quantized inputs are intentionally equivalent.
`FontVariation` consequently implements `Eq` and `Hash`; global, system, and span values
are overlaid per tag with the span layer last.

## Core: pre-shaped exact composition

`jlreq-core` is reachable directly and as `jlreq::core`; the composition-policy types the
facade takes as input — `Style`, `StyleBuilder`, and `StyleError` — are additionally
re-exported at the `jlreq` root so the headline typesetting knob needs no module path.
The core accepts caller-shaped UTF-8
clusters, integer advances, break opportunities, paragraph policy, and typed constructs.
Its `ShapedText`, `ParagraphBuilder`, `Style`, `CompositionLimits`, `Composer`,
`Layout`, placements, diagnostics, and typed errors retain the existing public behavior
under the new crate path.

The core is dependency-free, `no_std + alloc`, MSRV 1.85, and contains no font I/O,
Unicode line segmenter, bidi implementation, rasterizer, or drawing backend. Its result
uses logical integer geometry; the facade maps that result to physical 26.6 glyph geometry.

## Compatibility

Both libraries begin at 0.1.0. Within 0.1.x:

- directly exported names in `docs/public-api.toml` are preserved;
- `Style::default()` retains the JLReq 2020 reading;
- stable error and diagnostic codes retain their meanings;
- explicit font bytes and options retain deterministic output;
- the core keeps MSRV 1.85 and the facade keeps MSRV 1.88; and
- protocol v1 and the specification identifier remain separate compatibility contracts.

Before first publication the semver gate is network-free. Later 0.1.x candidates run
`cargo-semver-checks` in patch mode for both `jlreq-core` and `jlreq` against the latest
normal non-yanked registry release.
