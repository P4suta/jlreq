<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# ADR-0029: the coordinate system is a contract, and something has to hold it

- Status: accepted
- Date: 2026-09-06
- Builds on [ADR 0025](0025-three-product-layers.md),
  [ADR 0027](0027-the-layout-is-the-editor-surface.md), and
  [ADR 0028](0028-the-trace-is-not-a-diagnostic.md).

## Context

The only thing `jlreq` returns is coordinates. The repository documented the *fields* a
renderer reads and never the *space* they are in: not which way an axis grows, not which
corner a placement names, not whether `draw_origin` is a baseline. `docs/guide.ja.md` had
the closest thing, twenty-four lines listing accessors, in Japanese only.

Nothing held the geometry either. `jlreq_core::verify` states the composer's logical layout,
the trace channel states its reasoning, and the goldens pin both. The facade's physical
rectangles — the cells a renderer draws into, the ones `hit_test` measures against, the ones
`caret_rect` and `selection_rects` are cut from — were derived from those and then only
returned. The acceptance suite called `line.bounds()` and `glyph.cell_bounds()` on real
layouts and discarded the values; the unit tests pinned each formula against a hand-built
fixture, which fixes the expression and never compares the two.

Three defects lived there. A conditional space at a class boundary is billed to the boundary,
so a cell's advance is larger than the step to its neighbor; clamping that shortfall at zero
spent the space twice and pushed the rest of the line an eighth of an em per boundary. The
two halves of a tate-chu-yoko run share one inline coordinate; advancing past the first spent
a whole em the line never had. And the run's members were mapped onto the page from their own
upright orientation rather than the paragraph's, which placed them off the column entirely —
in the one writing mode the construct exists for. All three moved drawn text away from the
geometry hit testing was still using, in ordinary Japanese text, and no gate could see any of
them. A fourth, a warichu set at full size in the em the composer reserved for two reduced
lanes, is recorded rather than fixed: it is a size the facade never lets a caller state, and
that is its own decision to take.

## Decision

1. **The coordinate system is written down and normative.**
   [`docs/design/geometry.md`](../design/geometry.md) states the axes, the cell model, the
   corner `GlyphPlacement::origin` names, that `draw_origin` is not the baseline and what
   converts it, what `TextLine::inline_extent` excludes, and why a cell's advance is not the
   step to its neighbor. The rustdoc on the types carries the same statements, because a
   consumer reads docs.rs and not `docs/`.

2. **`jlreq::verify` is the facade's half of the invariant harness**, in the shape
   `jlreq_core::verify` already established: `inspect` returns a typed `Report` rather than
   panicking, `Fault` is `#[non_exhaustive]` with wildcard-free `kind` and `line`
   projections, and the module sits outside the compatibility promise in
   `docs/public-api.toml` for the same reason the core's does — the set of statements is
   expected to grow.

3. **A statement with no possible witness is not a `Fault`.** `origin` is its cell's corner,
   and a line's `bounds` cover its own cell, are both derived from the same fields as the
   values they would be compared against, so no layout can break them. They are held by unit
   tests, so that a change to `cell_bounds` cannot end them silently, and they are not in the
   fault list, because `docs/design/invariants.md` already rules out invariants no input can
   violate.

   The same rule caught a third statement after this decision was written. *A cell stays
   inside its line* was asked against `TextLine::bounds`, which is the union of the very
   cells being asked about, so no layout could break it either; its witness only failed
   because it mutated a fixture after that union had been computed. It survives as a
   statement about the line's *composed box*, split by axis: past the block edge is a cell
   drawn onto a neighboring line, past the measure is a line holding more than it reports.
   Asking them together had been reporting every warichu overhang — a block-axis fault — as
   a measure overrun.

4. **A tate-chu-yoko run stands in its column.** The construct changes which way a glyph
   faces, not which way the column runs, so a member's coordinates are mapped from the
   paragraph's axes like every other cell on its line and its cell is its own reduced
   advance across the column by one em down it. Deciding the mapping from the member's own
   upright orientation put the run at an `x` equal to its position down the column, clear
   of the column entirely.

5. **The trace gains a third recording point.** `draw.cell` and `draw.line` record what the
   facade did with the composer's placements: the coordinate it was given, the advance it
   was charged, and the step actually taken. The core channel could not have shown either
   defect, because both were in arithmetic that happens after the core has finished.

## Consequences

`hit_test`, `caret_rect`, `selection_rects`, `TextLayout::bounds` and every glyph position
move where a class boundary or a stacked run occurs — which is most Japanese text with Latin,
brackets, or punctuation in it. That is the fix, not a side effect; the previous coordinates
disagreed with the layout they came from.

The statements are asked from four places: their own unit tests, a corpus assembled to make
them branch, the facade fuzz target on inputs nobody wrote a test for, and — for the one
statement a machine cannot check, that the baseline conversion is right — an example that
draws a layout and puts the text on the baseline it derives.

`jlreq-core` is untouched, so the 122,199-request census observes no change and none needed
re-running.
