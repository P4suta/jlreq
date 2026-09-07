<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# ADR-0032: a forced break leaves a line that under-reports what it holds

- Status: accepted — the defect is recorded, not corrected
- Date: 2026-09-07
- Builds on [ADR 0029](0029-the-coordinate-system-is-a-contract.md) and
  [ADR 0031](0031-a-line-reserves-annotation-space-on-the-wrong-side.md).

## Context

`TextLine::inline_extent` is the length the line occupies, and two things read it. The
composer emits `layout.overfull` when it exceeds the measure
(`crates/jlreq-core/src/pipeline/composition.rs`), and an alignment other than the start
edge positions the run from it. Everywhere else in this workspace it equals the span of the
line's own cells exactly.

After a `discretionary_break` it does not. The facade fuzz target reached this layout —
`"\u{fffd}\u{fffd}Zc !\u{fffd}"` at a measure of 16.0 and a size of 8.0, horizontal, with a
break declared at byte 8 — and the two numbers separate on the line the break leaves behind:

| declared break | line | range | `inline_extent` | span of its cells |
| --- | --- | --- | --- | --- |
| none | 0 | 0..10 | 2485 | 2485 |
| none | 1 | 10..13 | 512 | 512 |
| at byte 8 | 0 | 0..8 | 1845 | 1845 |
| at byte 8 | 1 | 8..13 | **1024** | **1664** |

Three clusters of one em each, at 512 apiece with a 128 gap between the last two, span 1664
in a line that reports 1024 — which is the measure, to the unit. That is what keeps the
diagnostic quiet: `composition.rs` asks `inline_extent > line_extent`, and 1024 is not
greater than 1024, so `layout.overfull` names only line 0 and the reader is told nothing
about line 1.

The same line also begins before its own origin: its three cells run −128..384, 384..896 and
1024..1536 in a box of 0..1024.

Alignment is not involved, and it is worth saying because it is the explanation this looked
like. `Alignment::Center` on the same text without the break reproduces the sound numbers in
the first two rows, and *with* the break it reproduces those three cells to the unit — the
same positions the break alone produces. Nor is it the first line of the pair: the line
*before* a forced break reports its span correctly. Only the remainder does.

## Decision

**The defect is recorded and left in place.** The line extent is computed in
`jlreq-core`'s composer, and correcting it moves the coordinates of every line a
discretionary break produces — through `layout.overfull`, through every non-start
alignment, and through the conformance cases that pin them. That is the change
[ADR 0031](0031-a-line-reserves-annotation-space-on-the-wrong-side.md) describes and the
same reason applies: it belongs in a change of its own, with the census as the arbiter,
not appended to one about the facade's geometry oracle.

## What the oracle says instead

`jlreq::verify::Fault::CellEscapesTheMeasureSilently` used to fire here, on the *first*
cell of that line — the one at −128, before the line's own inline origin. Its exemption
was the trailing *run*: the cells at the line's end that ぶら下げ hangs and whose advances
the composer collapsed. The cells above overrun the line's box at **both** ends,
−128 before it and 1536 past it, so the trailing cell was excused and the leading one was
reported.

Making the exemption symmetric would have been wrong, and measuring said so:
[ADR 0031](0031-a-line-reserves-annotation-space-on-the-wrong-side.md)'s furawake lays two
lanes at −2048..0 in a line whose text is otherwise flush to its measure at 9839 — a
leading run with **no** trailing run — and a symmetric rule silences it.

So the rule is asymmetric. A trailing run is excused on its own, because hanging
punctuation is a trailing thing and JLReq says so. A leading run is excused only when there
is a trailing run as well. Neither ever excuses an interior cell — one with a fitting cell
on each side — which is the fault's whole subject.

**That second clause is fitted, not derived.** No sentence of JLReq is behind it: it is the
line separating the two layouts this project can measure, and the furawake test is what it
was fitted against. It is stated here so that the next case is handled the way this repo
already handles known-and-pinned defects — recorded in an ADR and exempted in the fuzz
target, the way `deferred_by_adr_0031` exempts the furawake — rather than by growing a
third condition on a rule that would then be fitted to three points.

## A second thing this records: a point two cells hold

`Fault::HitTestMissesItsOwnCell` asked, for every cluster, whether the middle of its cell
hit-tests back to its own bytes. Over arbitrary text that is not an invariant. A cell is one
em along the inline axis while an advance is whatever the font says, so a proportional
cluster's cell reaches over its neighbour by construction; a hung comma, a control character
whose advance the composer collapsed, and a construct's own lanes all put cells on top of
each other on purpose. Where two cells hold the same point there is no single owner to
demand. Which of them answers is `better_hit`'s tie-break — ordinary, specified, and tested
where it lives.

Four fuzz findings in a row were that shape, each a new way for two cells to share a point.
The check now asks only where a point has exactly one possible owner: a cell whose interior
no other cell in the layout intersects. That is what makes any answer other than that owner a
defect rather than a preference, and it is one bound rather than one exemption per shape.

The teeth are unchanged — the three defects the check was built to catch all move cells that
do not overlap — but the witness had to change with it. `cell_the_hit_test_cannot_reach` was
two cells at one place, which is now exactly the ambiguity the check declines to judge. It is
a line whose box reaches across the text of the line after it: `hit_test` picks a line before
it picks a glyph, nearest bounds and earliest wins a tie, so the second line's only cell is
answered for out of the first, with bytes three characters away.

## Consequences

- A caller who declares a `discretionary_break` gets a line whose `inline_extent` may be
  short of the text it holds, with no diagnostic saying so. Measuring the cells is the
  workaround; the geometry is right, only the summary is not.
- `a_line_after_a_forced_break_under_reports_its_extent` in
  `crates/jlreq/tests/geometry.rs` pins both numbers. It fails when the composer is
  corrected, which is when this ADR closes.
- The facade's geometry oracle states two fewer things than it did, and both of them were
  things it could not know. `docs/design/geometry.md` carries the narrowed wording.
- The furawake case of [ADR 0031](0031-a-line-reserves-annotation-space-on-the-wrong-side.md)
  is still reported, and `a_furawake_in_a_reordered_line_is_still_wrong` now pins the two
  lane cells by byte range rather than only by fault kind — it was the test that caught the
  symmetric rule going too far, and pinning the ranges is what makes it able to.
