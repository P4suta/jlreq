<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# ADR-0031: a line reserves its annotation's space on the side it is not drawn on

- Status: accepted — the defect is recorded, not corrected
- Date: 2026-09-06
- Builds on [ADR 0029](0029-the-coordinate-system-is-a-contract.md) and
  [ADR 0030](0030-a-construct-is-centered-in-its-line.md).

## Context

A line that carries ruby, emphasis dots, a reference mark or a superscript reserves room for
it: `place_attachments` finishes with

```rust
line.block_extent = line.block_extent + attachment_extent + mirrored_extent;
```

and the composer then starts the next line at `block_cursor + line.block_extent`. But the
annotation itself is placed by `attachment_block`, at `line.block_origin - size.block()` in
horizontal writing — **before** the line's own origin, on the other side of the line from
the room that was just reserved for it.

Those two are only consistent when every line carries the same annotation extent. Then each
line's reserved tail is exactly the next line's annotation, and the stack closes. That is the
case every fixture in this workspace is in, and it is why nothing noticed:

- `C.2-note-7/ruby-runs-govern-breaks` has ruby on both of its lines, so line 0's reserved
  `[1000, 1500]` is exactly where line 1's ruby is drawn.
- `3.3.1/mono-ruby-internal-boundary-expands` has ruby on line 0 only, so the tail is merely
  unused.
- The other twenty-five attachment-bearing conformance cases are single-line.

**No case in the suite puts a bare line before an annotated one**, and that is the ordering
that breaks. Composed in `jlreq-core` with a two-line paragraph whose ruby is attached to the
second line only:

```
line 0: block_origin=0    block_extent=1000   body [0, 1000]
line 1: block_origin=1000 block_extent=1500   body [1000, 2000]   ruby [500, 1000]
```

Line 1's ruby is painted over the lower half of line 0's characters. The facade reproduces it
unchanged, and `jlreq::verify::inspect` called the layout sound, because
`annotation-overlaps-its-base` compares an annotation against the body of **its own** line —
the one line it is guaranteed not to be standing on.

The facade reaches the same state through ordinary wrapping. Mono ruby on the fixture
paragraph of `crates/jlreq/tests/construct_matrix.rs`, at the narrow measure:

| clusters | the construct lands on | its ruby | that line's predecessor | sound |
| --- | --- | --- | --- | --- |
| 3 | line 0 | `[-512, 0]` | none | yes |
| 4 | line 1 | `[512, 1024]` | line 0's text, `[0, 1024]` | no |

Three clusters and four differ only in which line the wrap puts the construct on. That is
the whole defect: an annotation on the first line has nothing to land on, and an annotation
on any later line lands on whatever the line before it did not reserve.

`docs/design/geometry.md` never said which side the reservation is on, so nothing was
contradicted. That silence is what let the two halves disagree.

## Decision

**The defect is recorded and left in place. It is not corrected in this change.**

Two models would correct it, and both move coordinates that a differential census validated:

1. **Annotation inside the box.** Draw the annotation at `block_origin` and the body at
   `block_origin + attachment_extent`. The line box then contains everything it reserved, the
   stack closes for any mix of annotated and bare lines, and the first line's ruby stops
   falling outside the layout's own origin. It moves the `block` of every attachment **and of
   every body cluster on an annotated line**.
2. **Reserve for the next line.** Keep the drawing side and make the cursor advance by this
   line's body plus the *next* line's annotation extent. It needs a second pass over the
   placed lines, it leaves the first line's annotation outside the layout, and it changes what
   `block_extent` means — so the same cases move anyway.

Either way all twenty-seven attachment-bearing conformance cases change, body coordinates
included. §3.3 gives no inter-line box model to derive the new values from — unlike
[ADR 0030](0030-a-construct-is-centered-in-its-line.md), where §3.2.5's "align the whole string
to the center of the vertical line" named the answer — so every value would be a new
unverifiable claim, and the 122,199-request OCaml/Racket census that would check them cannot
be run in this environment.

Recording a defect that the project can see, reproduce and measure is worth more than a
correction it cannot check.

## What was built instead

- `jlreq::verify::Fault::AnnotationOverlapsAnotherLine` asks every annotation about every line
  that is not its own. It has a witness, as `docs/design/invariants.md` requires, and it is the
  check whose absence hid this: the layout above reported itself sound.
- `crates/jlreq/tests/construct_matrix.rs` pins the twenty-four
  `(construct, length, writing mode)` combinations it fires on, as
  `ANNOTATION_ON_A_WRAPPED_LINE`. They are the annotation-bearing constructs at the lengths
  that make the fixture paragraph wrap; the same constructs are sound when the paragraph fits
  on one line, which is why the sweep asks two measures.
- `docs/design/geometry.md` now states which side the room is reserved on and that the
  annotation is drawn on the other, so the contract says what the code does.

## A second thing this records: a furawake in a reordered line

The facade lays a line out by walking its cells in visual order with a cumulative
cursor. A warichu or furawake lane restarts behind its predecessor, and the cursor can
only take that step back when the two cells are still adjacent in the walk. Bidi
reordering is free to separate them, and then the lanes sit end to end and the second
one leaves the measure -- which is what every furawake did before
[ADR 0030](0030-a-construct-is-centered-in-its-line.md) and what only this case still
does. A facade fuzz case of a furawake in a right-to-left paragraph found it.

One half of it was a regression and is fixed: the backwards step used to be handed to a
cell the walk reached somewhere else entirely, moving an em of text sideways and making
`hit_test` answer with the wrong bytes. A restart is now taken only when the cursor
really goes from the one cell to the other.

The other half is the pre-correction behavior surviving where the fix cannot reach, and
it is left alone. Two models were tried and both were worse: computing the gaps in
visual order breaks every ordinary bidi line, because the difference between two logical
coordinates means nothing for cells the composer did not place next to each other; and
giving a construct one bidi level throughout, so that it reorders as the inline object it
is, broke more column counts than it fixed. Expressing a lane restart in a reordered line
needs the cursor model replaced by absolute placement, which is not a merge-time change.

`a_furawake_in_a_reordered_line_is_still_wrong` pins it, and the fuzz target carries the
matching exemption -- keyed on a reordered line holding a construct, not on
`BaseDirection::RightToLeft`, which `Auto` would walk straight around.

## Consequences

- A caller stacking ruby lines of uniform extent — the ordinary case, and every case the
  suite covers — is unaffected.
- A caller mixing annotated and bare lines gets ruby drawn over the preceding line. There is
  no workaround in the public API; the coordinates are the composer's.
- The known-broken list in `construct_matrix.rs` is no longer empty, and that is deliberate:
  it names what is wrong and points here.
- Correcting this needs the census, and therefore an environment with the OCaml and Racket
  engines. It is the first thing to do in one.
