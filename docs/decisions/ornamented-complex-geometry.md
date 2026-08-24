<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: what an ornamented character complex is centered on, and how many of them a run is

- Applies to: the cl-21 placement round in
  [`pipeline`](../../crates/jlreq/src/pipeline.rs), and the same round in
  [`engines/ocaml/lib/pipeline.ml`](../../engines/ocaml/lib/pipeline.ml)
- Standing: `Unstated`
- JLReq: §3.3.9, §3.7.1, §B.2#9, §C.2#6, §E.2#5
- Observed by: `just census constructs` (20,102 requests), the emphasis and script variants

## The silence

The four classes §3.3.9 and §3.7.1 build — cl-20 through cl-23 — have no Appendix A key, so
their rows and columns in all six matrices are reached only by *building* the construct.
Three questions live there and no sentence answers any of them.

**"Centered on the base character" does not say on what.** §3.3.9's emphasis mark is half
the size of its base character and centered on it. A character has two extents that could be
meant: its own em box, and what it occupies on the line once Table 1 has put space after it.
The two part wherever a space is stated, and JLReq's own glossary entry for nakatsuki —
"aligned […] to the horizontal center of the base character in horizontal writing mode" — is
the same phrase with the same ambiguity.

**JLReq never says how many complexes an emphasis run is.** §B.2 note 9, §C.2 note 6 and
§E.2 note 5 are all stated about

> two consecutive characters belonging to the same ornamented character complex (cl-21)

and every one of them turns on whether the two characters are in the same complex. For a
superscript that is one construct over one complex, the question does not arise. For an
emphasis run over five characters, "the same complex" is either all five or one each, and
the three notes give different lines under the two answers.

**§3.7.1 hands its geometry away.** The section says of superscripts and subscripts that
JLReq takes

> the character size and the block direction positioning of superscripts and subscripts
> alongside the base character to be implementation definable parameters

which disclaims the size and the block-direction position. It does not say what the
annotation is centered on along the *inline* axis, what happens when the annotation is longer
than its complex, or whether an annotation that does not fit widens the line.

## The reading

**§3.3.9's "center of the base characters" is the center of the advance the line gave it,
spacing and all.** Not the em box. Where an emphasis run stands before an ideographic
character, the run is cl-21 and Table 1 states a quarter em after it, so the mark sits an
eighth of an em later than the em-box reading would put it. The same reading decides where
§3.7.1's annotation is centered.

**§3.3.9 makes each base character its own ornamented character complex, and §3.7.1 makes
the whole construct one.** Table 6's quarter em therefore opens between two emphasized
characters of one run and never inside one superscript's complex, and a break stated inside
an emphasis run is answered while one inside a `script` or a `reference-mark` construct is
refused ([construct-break-refusal](construct-break-refusal.md)).

**§3.7.1's annotation is centered on its complex, hangs over both neighbors where it is
longer, and opens the line nowhere.** It overhangs without §3.3.8's kind of permission and
without §3.3.6's kind of spacing. `ruby.alignment` selects nothing there either: §3.3.5's
question is about a reading, and §3.7.1's annotation is not one.

## Why

**A mark is placed on the line, and the line is where the advance is.** The em box is a
property of the font's own idea of the character; the advance is what the paragraph decided,
and every other placement in the engine is stated against it. Centering on the em box would
put the mark somewhere that depends on a metric the protocol lets the caller override
independently of the advance, and would make the mark's position and the base character's
reported extent disagree by half of whatever space Table 1 states — which is exactly the
eighth of an em the cl-21-against-cl-19 coordinate makes visible. Nothing before M7 could
reach that coordinate at all.

**One complex per character is what makes the three notes do work.** Under the whole-run
reading, §E.2 note 5 forbids expansion between *any* two characters of an emphasis run, and
Table 6's cl-21 row and column would then be unreachable from an emphasis construct: a five
character run would be a single indivisible block for spacing, breaking and expansion alike,
and §B.2 note 9 would have no coordinate inside a run to speak about. Under the
one-per-character reading each note has a subject: the boundary *between* two emphasized
characters is a boundary between two complexes and takes the table's amount; the boundary
inside a superscript's complex is not and takes nothing. §3.7.1's construct is one complex
for the same reason read the other way — a superscript is one thing associated with one base
character, and its own characters are not separately ornamented.

**§3.7.1's own disclaimer is about the block axis, and does not license inventing an inline
rule from nothing.** The section names the size and the block-direction position as
implementation definable and stops there, which leaves the inline geometry to be read from
what the construct *is*: an annotation set alongside a complex. Centering it across the
complex is the reading under which "alongside" means the whole thing rather than one edge of
it. Letting it overhang rather than widen the line is the reading under which §3.7.1 changes
no other line in the paragraph, which matters because a superscript is common in running text
and a rule that opened the line for it would make §3.7.1 a line-adjustment section — which it
is not, and which §3.8 would then have to account for.

## What would change it

A sentence in §3.3.9 naming the em box would reverse the first reading, and the coordinate
that would show it is `(cl-21, cl-19)` — the eighth of an em — which a conformance case can
carry with both answers as `disagreements` today. A statement in §3.3.9 or in §C.2 note 6 of
how many complexes a run of emphasized characters is settles the second, and is the revision
this project would most like to see, because three separate notes depend on it. For §3.7.1, a
revision that states the inline geometry — or that names it implementation definable too, as
it already does for the block axis — would move the third reading from `Unstated` to
`Alternative`, at which point both answers become selectable rather than one being published.
