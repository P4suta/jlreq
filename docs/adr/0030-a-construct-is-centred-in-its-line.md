<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# ADR-0030: a construct is centred in its line, not in an em

- Status: accepted
- Date: 2026-09-06
- Builds on [ADR 0007](0007-two-scalars-and-the-fixed-point-unit.md),
  [ADR 0019](0019-one-fact-one-carrier.md), and
  [ADR 0029](0029-the-coordinate-system-is-a-contract.md).

## Context

A warichu, a furawake and a tate-chu-yoko each occupy their line differently from a run of
ordinary clusters: they put two or more lanes, or two or more upright members, into the
block extent of one line. Three of them were placed wrongly, and the three defects were one
defect.

`compose_line` settled two questions in one pass. It accumulated the line's block extent
while it placed, and it placed each construct against `paragraph.text.size().block()` — the
paragraph's em — because that was the only extent available when the construct came up. The
line was then grown around the construct without the construct being re-centred in what it
grew to. While the em and the line agree, centring against either gives the same answer;
once the line is wider than an em they diverge, and the surplus is drawn onto the line
beside it.

- A tate-chu-yoko run was displaced by `(members − 2) × advance / 2`. That is **zero at two
  members**, and two is what every fixture in this workspace used: the geometry corpus, the
  `shared-space` trace golden, the SVG example, `tate_chu_yoko_is_one_centered_solid_item_in_a_vertical_line`,
  and conformance case `3.2.5/tate-chu-yoko-solid-centered-group`. At three members a member
  is drawn on the neighbouring column.
- A furawake was displaced by half the surplus for every column past the first, at every
  length, in both writing modes. Nothing in the workspace had a geometric test for one.
- A warichu was set at the paragraph's own em. §3.4 sets a 割注 in smaller characters and the
  composer reserves exactly one em for the two lanes, so the pair was twice what the line
  held and each lane overhung by half an em.

A fourth defect was the facade's and only became visible once the first three were gone:
`assign_trailing_gaps` clamped a backwards step at zero, which laid a construct's lanes end
to end instead of side by side and carried the error into every cell after them.

None of this was visible to any gate. It was found by sweeping each construct across every
length from one cluster to five in both writing modes and asking `jlreq::verify` — a test
that costs half a second and that nobody had written, because each construct had a fixture
at one length and a fixture at one length is a fixture at one length.

## Decision

1. **A construct is centred in the block extent of the line that holds it.**
   `construct_block_start` is the one expression that answers where a construct of a given
   extent begins in a line of a given extent, and the tate-chu-yoko, warichu and furawake
   placements all call it. When the construct is wider than the line the surplus splits
   evenly, which is the same expression — a line already grew to hold anything that could be
   wider, so this is centring rather than clamping.

2. **A line's block extent is decided before anything on it is placed.** `line_block_extent`
   walks the same branches the placement loop does and answers only the extent question.
   Placing against the extent known *so far* is what made the two questions circular.
   Annotations are excluded: they are reserved after the body is placed and stand beside it,
   so a construct is centred in the body the line composed, not in the room its ruby needed.

3. **A warichu is reduced by the facade, not by the composer.** JLReq §3.4 states a size;
   the composer's job is to reserve one em for the pair, which it already did. The facade
   halves the size, the advance, and each glyph's own advance and offset for the clusters in
   a warichu before handing them over — a linear scale of the same outlines, so a renderer
   drawing at the reported `font_size` lands on the reported cell. It is not a reshape: a
   face whose half-size metrics differ from half its full-size metrics is measured at the
   size the caller asked for, which [ADR 0002](0002-caller-supplied-metrics.md) makes the
   caller's to state.

4. **A backwards step inside one construct is a lane restart and the cursor takes it.**
   Everywhere else a backwards step is visual reordering — the composer's coordinate is
   logical and the facade walks cells in visual order — and there the cursor holds its
   place. The two cases are told apart by whether the neighbouring cells belong to the same
   construct.

5. **Expected outputs were derived, not re-blessed.** Five committed expectations moved:
   conformance cases `3.2.5/tate-chu-yoko-solid-centered-group` and both
   `3.7.2/furawake-declared-sublines` cases, and the two core public tests named above. Each
   new value was worked out from §3.2.5 and §3.4 by hand and then checked against the code,
   rather than the other way round, because a conformance case is a claim about the
   specification and blessing one makes the claim say whatever the code says.

## Consequences

Every other conformance case, both trace-golden suites and the whole acceptance suite are
byte-identical, which is the evidence that the change is the one intended and not a broader
shift. The facade's `constructs` golden moved because the warichu halved; its diff was read
line by line.

**The 122,199-request differential census cannot be re-run in this environment and has not
validated this change.** The census is the workspace's strongest evidence that composed
output matches the independent OCaml and Racket engines, and it is the evidence this change
does not have. What it has instead: three conformance cases whose values were derived from
the specification text, a construct sweep whose unsound set is empty, and the fact that
nothing outside the three constructs moved at all.

`crates/jlreq/tests/construct_matrix.rs` is the gate that would have caught all of this. It
sweeps thirteen constructs across five lengths and both writing modes and holds the set of
unsound combinations empty; a construct that breaks at a length nobody chose now fails a
test rather than waiting to be looked at.
