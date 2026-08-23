<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: where a stacked structure divides, and what its own edges carry

- Applies to: the warichu and furawake rounds in
  [`pipeline`](../../crates/jlreq/src/pipeline.rs), and the same rounds in
  [`engines/ocaml/lib/pipeline.ml`](../../engines/ocaml/lib/pipeline.ml)
- Standing: `Unstated`
- JLReq: §3.4.2, §3.7.2, §B.2#13, §C.3
- Observed by: `just census constructs` (15,870 requests), the warichu and furawake variants

## The silence

A warichu (§3.4.2) and a furawake (§3.7.2) are the two structures that *divide*: each sets
its own text on two or more sublines that run beside the line rather than along it. Three
questions follow from that and no sentence answers them.

**"A position where line breaking is permitted" does not say permitted by what.** §3.4.2
says a warichu divides at such a position. Permitted by Table 2, which is what the phrase
means everywhere else in JLReq? Or permitted by the caller, who stated the paragraph's own
break opportunities? The two are different sets, and at least one boundary separates them
sharply: §C.3 forbids a line to end after an opening bracket, and a warichu whose text opens
with one has to divide somewhere.

**"Should not be longer" does not say what to do when nothing satisfies it.** §3.4.2:

> the length of the second line should not be longer than the length of the first line.

Whether that is a *bound* — a position that violates it is not a candidate, and a note with
no satisfying position is left undivided — or a *preference* among the candidates is not
stated. It also does not say how two positions that balance equally are settled.

**Table 1's line-end column does not say what "the line" is when a structure ends.** The
bracket that closes a warichu is the last character of the *structure*. Whether the space
Table 1 states after it belongs to that bracket's own reported advance, or stands after the
whole block, is a question about which object Table 1's columns are asked of, and Appendix B
is written as though a line had no structures in it.

## The reading

**§3.4.2's "a position where line breaking is permitted" is a position the caller stated.**
A warichu divides at one of the request's own break opportunities where it offers any, and at
whichever cluster boundary balances the two sublines best where it offers none — so the
sentence is a restriction where the caller made one and nothing where the caller did not.
Table 2 is not consulted: a warichu divides after an opening bracket, which §C.3 forbids a
line to end at.

**§3.4.2's balance sentence is a preference among the stated positions and not a bound on
them.** Where every position the caller offered leaves the second subline longer, the least
unbalanced of them is taken rather than the note being left undivided. Two positions that
balance equally are settled by the earlier one.

**A stacked structure's own last character carries no space, and the structure does.** The
space Table 1 states after a closing warichu bracket stands after the whole block and is no
part of the bracket's reported advance — visible at `(cl-29, cl-05)`, the quarter em a middle
dot takes after a warichu. The same holds at the end of every subline: the character that ends
one takes nothing after it, and Table 1's line-end column is asked of the line and of nothing
else.

## Why

**A subline is not a line, so Table 2's answer is about something else.** Table 2 states
where a *line* may end, and §C.3's prohibition on ending after an opening bracket is a rule
about what a reader sees at the right edge of the measure. A warichu's two sublines are set
one above the other inside one position on the line; nothing about them is a line edge in the
sense §C.3 is written for. Consulting Table 2 would import a rule from a geometry the
structure does not have, and would make some warichu texts — one that begins with an opening
bracket and holds three characters, say — undividable for a reason that has nothing to do
with what a reader would see. The caller's own break opportunities, by contrast, are
statements about *this text*, which is what §3.4.2's sentence is looking for.

**"Should" is the specification's own word for a preference.** JLReq distinguishes what is
necessary from what is preferred throughout, and §3.4.2 chose the weaker form here. A bound
reading would have to say what happens when the bound cannot be met, and the only two
answers available — leave the note undivided, or overflow — are both worse than an unbalanced
division and neither is stated. Taking the earlier of two equal positions is the tie-break
that needs no further rule: it is the order the text is already in. The `constructs` census
gives a warichu four half-em characters in one variant and five in another for exactly this
reason — the sentence is satisfiable at four and not at five, which is where it stops being
a bound and starts being a preference.

**Table 1's columns are asked of the line, and a structure is one object on it.** The whole
of how a stacked structure participates in a line is that it occupies one position: it takes
one advance, it is one thing to spacing, breaking and adjustment. If the closing bracket's
own advance carried the trailing space, then the block's reported width and the width the
line was measured at would be two different numbers, and a caller reading back the geometry
would find the structure ending in a place the line does not agree with. §B.2 note 13 names
four edges and two of them are the warichu's own, which is the specification acknowledging
that a warichu has edges of its own — and an edge of its own is not the line end.

## What would change it

A revision of §3.4.2 that names Table 2 explicitly, or that says "a position where the text
permits line breaking", settles the first reading in either direction; a conformance case for
a warichu whose text opens with a bracket would carry both answers as `disagreements` today.
A revision that changes "should not be longer" to a requirement, and states what an
unsatisfiable requirement does, settles the second. For the third, an Appendix B legend that
distinguishes the end of a *line* from the end of a *structure* — or a note in §3.4.2 saying
whose advance the trailing space is part of — would move the reading into the transcription,
where it belongs, because it is a fact about how the matrix is addressed rather than about
what a warichu is.
