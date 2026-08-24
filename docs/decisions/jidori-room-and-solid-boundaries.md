<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: what a jidori measures its room in, and where a run with no boundary left stands

- Applies to: the jidori round in [`pipeline`](../../crates/jlreq/src/pipeline.rs), and the
  same round in [`engines/ocaml/lib/pipeline.ml`](../../engines/ocaml/lib/pipeline.ml) and
  [`engines/racket/structure.rkt`](../../engines/racket/structure.rkt)
- Standing: `Unstated`
- JLReq: §3.7.3, §B.1, §C.3, §3.8.3
- Observed by: `just census tabs` (31,211 requests) and `just census constructs` (18,515
  requests), the jidori variants; no built-in case reaches a jidori with no open boundary

## The silence

§3.7.3 states jidori processing in two rules and one exception, and each of the three
leaves a quantity unnamed.

**"Adjusted using spacing between characters" does not say what is being adjusted from.**
The rule is

> The jidori text should be adjusted using spacing between characters so that the sides of
> the text are aligned at the defined length.

The defined length is stated — the second rule makes it "a whole number of full-width
characters at the size defined for the surrounding text" — but the length the run *starts*
at is not. Whether the room a jidori has to open is the difference between that length and
the sum of the characters' own advances, or the difference between it and what the run
would have measured on the line with Table 1's amounts between its members, is a question
§3.7.3 does not reach. The two differ at every run that holds a boundary Table 1 states an
amount at, and §3.7.3 does not say either when the measurement is taken — before §3.8.3's
ladder has run, or after a line that had to give spacing back has taken some of the same
amounts away.

**"Positions where line breaks are prohibited" ends in "and so on".** The exception is

> The following, however, should be set solid: Positions where line breaks are prohibited:
> between grouped numerals (cl-24); between Western characters (cl-27); between two
> inseparable characters (cl-08); and so on. These sequences should be treated as a single
> block.

Three examples and an "and so on" is not a set. Whether the excluded boundaries are exactly
the ones Table 2 forbids a line to end at — and, if so, at which of §C.3's four levels — or
some smaller list the three examples are meant to enumerate, is not stated.

**"Treated as a single block" does not say where the block stands.** A run every one of
whose boundaries is prohibited has no site to open at all, so the first rule's "aligned at
the defined length" cannot be satisfied on both sides at once. §3.7.3 does not say whether
such a block is set at the head of its cells, at their end, or centered in them.

## The reading

**A jidori's room is the defined length less what the run measures on the line, with
Table 1's amounts read as transcribed.** The run's natural extent is the sum of its
members' advances *including* the spacing Table 1 states after each of them, except after
its last member, whose own body is all the run contributes there. It is the amount the
matrix states, taken before either ladder has run. The surplus is what is left of the
defined length after that, and it is never negative: a run already wider than its cells is
set as it stands rather than reduced into them.

**The boundaries §3.7.3 sets solid are exactly the boundaries a line may not end at.** The
predicate is the same one composition asks — Table 2 read at the paragraph's own §C.3
level, with §C.3's four common prohibitions on top of it — and not the three examples the
sentence names. A boundary that is legal for a line break is a site the surplus is divided
over; a boundary that is not takes none of it.

**A run with no legal boundary left is one block at the head of its cells.** The whole
surplus falls after the run's last member, so the block's first character stands at the
start of the defined length and the space stands after it.

The surplus divides equally over the eligible boundaries, and the odd units follow
`adjustment.remainder` — the same answer that orders every other set of adjustment sites
in this engine.

## Why

**A jidori is a run on the line, and a run on the line is measured the way the line
measures it.** §3.7.3's own subject is a stretch of text whose *total length* is specified;
the length it has before the specification takes effect is the length the line would have
set it at, Table 1 included, because Table 1 is what the line's own arithmetic is. Reading
the natural extent as bodies alone would make a jidori of two Western characters and a
jidori of a bracket and an ideograph open by the same amount when the line sets them at
visibly different widths, and would make the section's own exception incoherent: a boundary
"set solid" is a boundary whose Table 1 amount stands, which is a statement about an amount
the run is being measured with.

The last member is the exception because the amount Table 1 states after it is not inside
the run. It belongs to the boundary between the jidori and whatever follows, which the
line owns — the same reading [stacked-structure-geometry](stacked-structure-geometry.md)
publishes for a structure's own closing character, applied to a construct that sets its
text along the line rather than beside it.

**"And so on" after three examples of one category names the category.** The sentence's own
colon does the work: it says *positions where line breaks are prohibited*, and then gives
three. Reading the list as closed would leave the phrase before the colon with nothing to
govern, and would make a jidori open a boundary at which the line it stands on may not
break — which is exactly the outcome the sentence's own reason forbids, since it explains
the exception by saying such sequences "should be treated as a single block". Reading it as
the category is also the only reading that answers at a coordinate the three examples do
not name at all, and Table 2 is where JLReq keeps that answer. Asking it at the paragraph's
own §C.3 level rather than at a fixed one follows from the same argument: the level is what
makes Table 2 an answer about *this* paragraph.

**A block has to stand somewhere, and the head is the only choice the section supports.**
When no boundary is eligible, the run is one object of a known width standing in a known
number of cells, and the surplus is space no rule has a site for. Setting the block at the
head puts the space where the line's own reading order puts unused room, and keeps the
first character of a jidori at the position a caller computed its cells from. Centering it
would invent a second rule about a quantity §3.7.3 states nothing about, and setting it at
the end would move the run's first character away from a position the caller specified.
This is the weakest of the three readings and the one most clearly a choice among
defensible answers.

**Only a census reaches it.** No built-in case sets a jidori whose every boundary is
prohibited — it takes a run of Western characters, or of grouped numerals, in cells wider
than the run. The `constructs` and `tabs` censuses set a jidori across every class pair the
matrices carry, at cells the run under-fills and over-fills, which is where the three
readings above become three different lines.

## What would change it

A revision of §3.7.3 that names the length the adjustment starts from settles the first
reading; the concrete form is a sentence saying whether the inter-character spacing of the
surrounding composition is part of the run's own length. A revision that closes its "and so
on" — or that cites Table 2 by name — settles the second. A sentence stating where a jidori
that cannot be opened stands settles the third, and is the one a conformance case would
carry both answers for today.

Where §3.7.3's own two renderings state opposite rules about an *inserted space* inside the
run, that is a divergence rather than a silence and is published separately, in
[jidori-inserted-space-locale-split](jidori-inserted-space-locale-split.md).
