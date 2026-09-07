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
- `TextLine::bounds` covers the line's own text cell **and every cell in the line**, so a
  ruby line's bounds are taller than its `block_extent`, and so are the bounds of a line
  holding a cell that escaped. It is a union, not a claim: ask it what to repaint, never
  whether a cell is where it should be.
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
`GlyphTransform::TateChuYoko`, and the direction of its `advance_x` — differs. Along the
inline axis the whole run stands at **one** position and occupies one em of it, however many
members it holds. Across the column each member gets its own advance and they sit side by
side, and the line's block extent is `max(em, members × advance)` — a run wider than the em
widens its line, which is intended. The run then stands centered in whatever that comes to;
see [Constructs that are not plain body text](#constructs-that-are-not-plain-body-text).

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

`TextLine::block_extent` is the line's own text **plus the room reserved for its
annotations** — ruby, emphasis dots, a reference mark, a superscript, a subscript. The next
line begins where that extent ends.

Which side of the line the room is on is the part worth stating, because the two sides
disagree. A subscript is reserved *and drawn* after the line's text, inside the box. Every
other annotation is reserved after the line's text and **drawn before its origin**, on the
opposite side — so an annotated line's box is offset from its own content by the annotation's
extent, and the room it reserved is filled by whatever the *next* line draws backwards into
it. That closes only while consecutive lines carry equal annotation extent. When a bare line
is followed by an annotated one, the annotation lands on the bare line's characters.

That is a defect, it is reproducible, and it is not fixed:
[ADR 0031](../adr/0031-a-line-reserves-annotation-space-on-the-wrong-side.md) records it, the
two models that would correct it, and why the specification does not say which is right.
`verify::inspect` reports it as `annotation-overlaps-another-line`, and
`ANNOTATION_ON_A_WRAPPED_LINE` in
[`crates/jlreq/tests/construct_matrix.rs`](../../crates/jlreq/tests/construct_matrix.rs) pins
the combinations it fires on.

## The step between two cells is not an advance

Two of the three defects lived here, so it is stated rather than left to be inferred.

A cell's advance is what the composer **charged** it. The distance to the next cell is a
different number, and the composer is the authority on it:

- A conditional space at a class boundary is billed to the boundary, so each side's advance
  holds part of it and the next cluster legitimately begins *inside* the preceding advance.
  `漢`+`A` places one quarter em; deriving positions by summing advances places it twice.
- The two halves of a tate-chu-yoko run share **one** inline coordinate and differ only in
  block, so the step between them is zero. Advancing anyway spends an em the line never had.
- A warichu or furawake lane restarts at the construct's own inline origin. That step is
  backwards and it is real: the lanes stand side by side, so the cursor takes it. Clamping it
  at zero laid the lanes end to end and carried the error into every cell after them.
  Everywhere *else* a backwards step is visual reordering rather than a restart — the
  composer's coordinate is logical and the cells are walked in visual order — and there the
  cursor holds its place.

The `draw.cell` trace family records all three numbers — the composer's coordinate, the
advance it charged, and the step actually taken — for every cell, precisely so that a
disagreement among them is visible in a diff rather than only on screen. `draw.line` then
states the line's composed extent beside the coordinate its last cell reached.

## Constructs that are not plain body text

A warichu, a furawake and a tate-chu-yoko each occupy their line differently from a run of
ordinary clusters, and each was wrong until [ADR 0030](../adr/0030-a-construct-is-centered-in-its-line.md).
The rule they now share is one sentence: **a construct is centered in the block extent of the
line that holds it.** The line is as wide as its widest construct, so that extent is settled
before anything is placed rather than accumulated while placing.

- **A warichu is set at half the paragraph's size.** JLReq §3.4 sets a 割注 in characters
  smaller than the text around it, two lanes inside the space one line takes, and the
  composer reserves exactly one em for the pair. The facade halves the size, the advance and
  each glyph's own metrics for the clusters in a warichu before handing them over.
- **A furawake fills the columns its line reserved**, one em per column, its lanes flush and
  side by side rather than end to end.
- **A tate-chu-yoko run is centered across its column** at every member count. The line's
  block extent is `max(em, members × advance)` — a run wider than the em widens its line,
  which §3.2.5 permits and `docs/conformance-deferrals.toml` records as owned behavior —
  and the run stands centered in whatever that comes to.

The reading order is a separate statement and was never disturbed: in `VerticalRl` a
warichu's first lane is the right-hand one, as vertical reading order requires, and each lane
runs along the inline axis in source order.
`a_warichu_reads_the_way_its_writing_mode_does` in
[`crates/jlreq/tests/geometry.rs`](../../crates/jlreq/tests/geometry.rs) holds that with four
characters, because two put one character in each lane and look the same either way round.

Exact coordinates are in
[`crates/jlreq/tests/construct_geometry.rs`](../../crates/jlreq/tests/construct_geometry.rs);
[`crates/jlreq/tests/construct_matrix.rs`](../../crates/jlreq/tests/construct_matrix.rs)
sweeps every construct the builder offers at every length from one cluster to five in both
writing modes and pins the set of unsound combinations exactly. That sweep is what found two
of the three defects: each construct had been tested at exactly one length, and for
tate-chu-yoko that length was the one where its displacement cancels. Its list is not empty —
it holds the annotation combinations of ADR 0031 above.

## Where each statement is enforced

| statement | held by |
| --- | --- |
| lines partition the source, separators excepted | `verify::inspect` — `coverage-*`, `lines-do-not-meet` |
| lines never overlap and never reverse direction | `verify::inspect` — `lines-overlap`, `block-progression-reverses` |
| a cell stays inside its line on the block axis | `verify::inspect` — `cell-escapes-its-line` |
| an **interior** cell stays inside the measure (inline axis) — a run at the line's end is ぶら下げ, a run at its start is excused only alongside one at its end ([ADR 0032](../adr/0032-a-forced-break-leaves-a-line-that-under-reports-itself.md)) | `verify::inspect` — `cell-escapes-the-measure-silently` |
| an annotation stands beside its base, not over it | `verify::inspect` — `annotation-overlaps-its-base` |
| an annotation stands on no other line's text | `verify::inspect` — `annotation-overlaps-another-line`, which today reports the open defect of [ADR 0031](../adr/0031-a-line-reserves-annotation-space-on-the-wrong-side.md) |
| a caret stands on some line | `verify::inspect` — `caret-stands-on-no-line` |
| a click in a cell **no other cell holds** answers with that cell's bytes — cells are em boxes over proportional advances, so where two of them share a point the owner is `better_hit`'s tie-break, not a geometric fact ([ADR 0032](../adr/0032-a-forced-break-leaves-a-line-that-under-reports-itself.md)) | `verify::inspect` — `hit-test-misses-its-own-cell` |
| `origin` is the cell's inline-start, block-end corner | a unit test, because both sides are derived from the same fields and a check with no possible witness is a guess |
| the exact coordinates of a known layout | `crates/jlreq/tests/geometry.rs` |
| every step the facade took | the `draw.cell` and `draw.line` goldens |
| the baseline conversion | `examples/render_svg.rs`, by looking at it |
