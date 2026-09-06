<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# The coordinate system

The only thing `jlreq` returns is coordinates, and until this document existed it never said
what they meant. Where the origin is, which way each axis grows, what point a glyph's origin
names, and whether that point is a baseline were all readable from the source and stated
nowhere. Three defects lived in that silence long enough to ship — each of them a
disagreement between the cells a renderer would draw into and the cells hit testing was
measuring against — and none was visible to any gate, because nothing in the repository drew
anything or compared one rectangle to another.

So: the statements below are the contract. [`jlreq::verify`](../../crates/jlreq/src/verify.rs)
holds every layout to them, [`crates/jlreq/tests/geometry.rs`](../../crates/jlreq/tests/geometry.rs)
asks it over a corpus assembled to make them branch, the facade fuzz target asserts them on
inputs nobody wrote a test for, and
[`crates/jlreq/examples/render_svg.rs`](../../crates/jlreq/examples/render_svg.rs) draws a
layout so a wrong reading is visible rather than merely arguable.

## Axes

Physical coordinates are the screen's: **+x is right, +y is down**. There is one origin per
layout, at the start of its first line, and every number `TextLayout` returns is in that
space and in signed 26.6 fixed point.

Two axes are named after the text rather than the screen, and which screen axis each one is
depends on the writing mode:

| | `HorizontalTb` | `VerticalRl` |
| --- | --- | --- |
| inline axis (along a line) | +x | +y |
| block axis (line to line) | +y | **−x** |

`VerticalRl` is the only place a physical axis runs backwards: columns progress leftwards,
so a later line has a *smaller* `x`. Nothing else in the crate is signed against the reader's
expectation.

## Cells

Every rectangle this crate returns is a **layout cell** — the space the composition gave a
glyph, a line, or a whole layout. It includes advance space, whitespace, and annotation
cells. It is not the ink of an outline: a rasterizer derives ink bounds from the selected
face, size, variations, and synthesis, and ink may sit inside the cell or overhang it.

- `GlyphPlacement::cell_bounds` is one glyph's virtual body (仮想ボディ).
- `TextLine::bounds` covers the line's own text cell **and** every annotation beside it, so
  a ruby line's bounds are taller than its `block_extent`.
- `TextLayout::bounds` is the union of the lines'.

## The corner a placement names

`GlyphPlacement::origin` is the cell's **inline-start, block-end** corner. Concretely:

| | `HorizontalTb` | `VerticalRl` |
| --- | --- | --- |
| `origin.x` | the cell's left edge | the cell's **right** edge |
| `origin.y` | the cell's **bottom** edge | the cell's top edge |

`draw_origin` is that point plus the shaper's own offset for the glyph. `TextLine::origin`
is the same corner of the line's text cell — inline-start, block-**start** — which is not
the same convention, because a line grows away from its origin along the block axis while a
glyph's origin sits at the far end of its own.

A tate-chu-yoko run is set upright inside vertical text, and that is all it changes. Its
members stand in the column like every other cell on their line: the axes are the
paragraph's, the corner rule is the same, and only the glyph's own orientation — reported as
`GlyphTransform::TateChuYoko`, and the direction of its `advance_x` — differs. What the run
does have is a cell narrower than an em: the composer gives each member its own reduced
advance across the column and reserves the run's total rather than an em per member, so a
two-digit run in a 16-unit em occupies about eleven units of column, side by side, in one
em of it.

Mapping a member's coordinates from its *own* orientation instead of the paragraph's put
the run at an `x` equal to its position down the column — clear of the column entirely —
until `docs/adr/0029` settled that the paragraph decides the axes.

## The one thing `draw_origin` is not

**It is not the baseline.** A rasterizer that places an outline there puts every glyph one
descent too far along the block axis — about a tenth of an em for a typical Japanese face,
which is small enough to look like hinting and wrong enough to misalign every underline.

The baseline is one em-relative descent from the cell's block-end edge, and
`FontMetrics::descent` is negative, which is exactly the correction:

```rust,ignore
let cell = glyph.cell_bounds();
let descent = layout
    .font(glyph.font_id())
    .and_then(jlreq::FontResource::metrics)
    .map_or(0.0, |metrics| metrics.descent());
let baseline_y = cell.y() + cell.height() + descent * glyph.font_size();
```

`FontMetrics` exists for this. `render_svg` uses exactly this expression, and SVG's `<text>`
places its own baseline at the point it is given, so if the expression were wrong the
example's glyphs would sit off their own drawn cells.

## What a line reports

`TextLine::inline_extent` is the length the measure was met at, **excluding hanging
punctuation** — JLReq's ぶら下げ, where a full stop or comma is allowed past the measure.
The hung glyph is still placed and still has a cell, so `bounds` covers it and
`inline_extent` does not. A renderer painting a line background wants `bounds`; a caller
asking whether the measure was met wants `inline_extent`.

A line may also hold more than its measure without hanging anything, when nothing in the
adjustment ladder can give the remainder back. That is not silent: it is the
`layout.overfull` diagnostic.

## The step between two cells is not an advance

Two of the three defects lived here, so it is stated rather than left to be inferred.

A cell's advance is what the composer **charged** it. The distance to the next cell is a
different number, and the composer is the authority on it:

- A conditional space at a class boundary is billed to the boundary, so each side's advance
  holds part of it and the next cluster legitimately begins *inside* the preceding advance.
  `漢`+`A` places one quarter em; deriving positions by summing advances places it twice.
- The two halves of a tate-chu-yoko run share **one** inline coordinate and differ only in
  block, so the step between them is zero. Advancing anyway spends an em the line never had.
- A warichu or furawake lane restarts near the line's start. That step is backwards, and it
  is a new lane rather than a shared coordinate, so the cursor does not follow it.

The `draw.cell` trace family records all three numbers — the composer's coordinate, the
advance it charged, and the step actually taken — for every cell, precisely so that a
disagreement among them is visible in a diff rather than only on screen. `draw.line` then
states the line's composed extent beside the coordinate its last cell reached.

## What is not yet true

One construct does not satisfy the statements above, and it is stated here rather than
excused in the checker.

**A warichu is set at full size.** JLReq §3.4 sets a 割注 in smaller characters, two lanes
inside the space one line takes, and `jlreq-core` places it that way — the lanes go half an
em either side of the line's block origin and the line reserves one em for the pair. The
facade never reduces the size: `DocumentBuilder::warichu` marks a range and the clusters in
it reach the composer at the paragraph's own em, so two full-em lanes are placed in the em
the line reserved and each overhangs by half of one.

`jlreq::verify` reports both lanes as leaving their measure, and it is meant to.
`crates/jlreq/tests/warichu_size.rs` pins the geometry as it is, and
`crates/jlreq/tests/document_trace.rs` records exactly those two faults by name, so the
expectation fails the moment either is fixed. Choosing the size is the same open question as
the ruby size §3.3.3 leaves open and the anisotropic sizes
[ADR 0027](../adr/0027-the-layout-is-the-editor-surface.md) defers: ADR 0019 settles that a
size the caller measured is carried by the measurement, and the facade offers no way to
state one for either.

## Where each statement is enforced

| statement | held by |
| --- | --- |
| lines partition the source, separators excepted | `verify::inspect` — `coverage-*`, `lines-do-not-meet` |
| lines never overlap and never reverse direction | `verify::inspect` — `lines-overlap`, `block-progression-reverses` |
| a cell stays inside its line | `verify::inspect` — `cell-escapes-its-line` |
| an interior cell stays inside the measure | `verify::inspect` — `cell-escapes-the-measure-silently` |
| an annotation stands beside its base, not over it | `verify::inspect` — `annotation-overlaps-its-base` |
| a caret stands on some line | `verify::inspect` — `caret-stands-on-no-line` |
| a click in a cell answers with that cell's bytes | `verify::inspect` — `hit-test-misses-its-own-cell` |
| `origin` is the cell's inline-start, block-end corner | a unit test, because both sides are derived from the same fields and a check with no possible witness is a guess |
| the exact coordinates of a known layout | `crates/jlreq/tests/geometry.rs` |
| every step the facade took | the `draw.cell` and `draw.line` goldens |
| the baseline conversion | `examples/render_svg.rs`, by looking at it |
