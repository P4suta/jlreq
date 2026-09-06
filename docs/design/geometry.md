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
widens its line, which is intended. Where the run sits inside that line is not yet right; see
[What is not yet true](#what-is-not-yet-true).

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

Three constructs that are not plain body text come out wrong. All three are stated here
rather than excused in the checker; the coordinates are pinned in
[`crates/jlreq/tests/construct_geometry.rs`](../../crates/jlreq/tests/construct_geometry.rs)
and the set of broken combinations in
[`crates/jlreq/tests/construct_matrix.rs`](../../crates/jlreq/tests/construct_matrix.rs),
which sweeps every construct the builder offers at every length from one cluster to five in
both writing modes. That sweep is what found two of the three: each construct had been
tested at exactly one length, and for tate-chu-yoko that length was the one where its defect
cancels.

Two of the three share a cause, and it is `jlreq-core`'s. A multi-lane construct is
positioned against **the paragraph's em**, and the line's block extent is then grown to hold
it without the construct being re-centred in what it grew to. While the em and the line agree
the construct is centred; once the line is wider it is not, and the surplus is drawn onto the
line beside it.

**A warichu is set at full size.** The facade hands the composer the paragraph's own em for
every cluster and has no way to be told otherwise. JLReq §3.4 sets a 割注 in smaller
characters, two lanes inside the space one line takes, and `jlreq-core` places it that
way — the lanes go half an
em either side of the line's block origin and the line reserves one em for the pair. Given
full-em clusters, each lane overhangs its line by half an em on the block axis, which is to
say onto the line beside it. `jlreq::verify` reports both lanes as `cell-escapes-its-line`,
and `crates/jlreq/tests/document_trace.rs` records exactly those two faults by name.

One consequence the checker does not name separately, because naming it would be a second
report of the same defect: the overhanging lane reaches into the *adjacent* line's cells,
and nothing compares one line's cells against another's.

What the size does **not** disturb is the order. In `VerticalRl` the first lane is the
right-hand one, as vertical reading order requires, and each lane runs along the inline axis
in source order; a full-size lane is drawn over its neighbour, not reversed.
`a_warichu_reads_the_way_its_writing_mode_does` in
[`crates/jlreq/tests/geometry.rs`](../../crates/jlreq/tests/geometry.rs) holds that, with
four characters — two put one character in each lane, which looks the same either way round.

**A tate-chu-yoko run is not centred in its line.** JLReq §3.2.5 asks for the string to be
set solid from left to right and then centred in the vertical line. The line's block extent
is `max(em, members × advance)`, which is right — a run wider than the em widens the line,
and `docs/conformance-deferrals.toml` records that as owned, conformance-measured behaviour.
The run's *position* in that line is not: the group is centred on the line's block **origin**
rather than on its centre, so it is displaced by `(members − 2) × advance / 2`. At two
members that is zero — which is the count every test in this workspace used — and at one,
three, four and five members the run leaves its own line, which `jlreq::verify` reports.

This one is not the facade's. The displacement is in the block coordinates `jlreq-core`
emits, and the facade maps them faithfully; the coordinates conformance case
`3.2.5/tate-chu-yoko-solid-centered-group` expects carry the same displacement at three
members. It is recorded rather than corrected because correcting it changes composed output,
and because a conformance case is a claim about the specification rather than an
implementation detail to edit in passing.

A run widening its line is **not** a defect, and an earlier revision of this document said it
was, on a requirement §3.2.5 does not state. The section asks for solid setting and centring
and nothing narrower. Getting two digits into one em is a matter of using their half-width
forms, which is shaping, which [ADR 0001](../adr/0001-no-std-no-io-no-font-in-core.md) and
[ADR 0002](../adr/0002-caller-supplied-metrics.md) place with the caller.

**A furawake is placed half an em short per extra column.** The line reserves one em per
column and the facade's full-em clusters fill exactly that, so unlike the warichu the size is
right. `place_furawake_segment` centres the segment inside `paragraph.text.size().block()` —
one em — so with two columns the lanes land half an em before where the line put them, and
the first lane sits on the line above. Nothing in this workspace had a geometric test for a
furawake at any length; it is wrong at all of them, in both writing modes.

Choosing the warichu's size is the same open question as the ruby size §3.3.3 leaves open
and the anisotropic sizes [ADR 0027](../adr/0027-the-layout-is-the-editor-surface.md)
defers: ADR 0019 settles that a size the caller measured is carried by the measurement, and
the facade offers no way to state one for a warichu at all.

## Where each statement is enforced

| statement | held by |
| --- | --- |
| lines partition the source, separators excepted | `verify::inspect` — `coverage-*`, `lines-do-not-meet` |
| lines never overlap and never reverse direction | `verify::inspect` — `lines-overlap`, `block-progression-reverses` |
| a cell stays inside its line on the block axis | `verify::inspect` — `cell-escapes-its-line` |
| an interior cell stays inside the measure (inline axis) | `verify::inspect` — `cell-escapes-the-measure-silently` |
| an annotation stands beside its base, not over it | `verify::inspect` — `annotation-overlaps-its-base` |
| a caret stands on some line | `verify::inspect` — `caret-stands-on-no-line` |
| a click in a cell answers with that cell's bytes | `verify::inspect` — `hit-test-misses-its-own-cell` |
| `origin` is the cell's inline-start, block-end corner | a unit test, because both sides are derived from the same fields and a check with no possible witness is a guess |
| the exact coordinates of a known layout | `crates/jlreq/tests/geometry.rs` |
| every step the facade took | the `draw.cell` and `draw.line` goldens |
| the baseline conversion | `examples/render_svg.rs`, by looking at it |
