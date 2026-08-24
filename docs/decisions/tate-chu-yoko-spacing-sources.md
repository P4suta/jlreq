<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: which of §3.2.5 and the matrices states the space beside a tate-chu-yoko run

- Applies to: the cl-30 placement round in
  [`pipeline`](../../crates/jlreq/src/pipeline.rs), and the same round in
  [`engines/ocaml/lib/pipeline.ml`](../../engines/ocaml/lib/pipeline.ml)
- Standing: `Adjudicated` for the first silence — §3.2.5's own prose and the table its own
  sentence points at do not state the same rule, which is the `contradictory` permission
  `xtask/src/policy.rs` maps onto that standing — and `Unstated` for the second, which no
  sentence of either reaches at all.
- JLReq: §3.2.5, §3.6, §B.1, §B.2, §3.8.3, §3.8.4
- Observed by: `just census tate-chu-yoko` (4,761 requests) for the cl-30 coordinates, and
  `just census tabs` (24,334 requests) for the cell after a tab sign

## The silence

§3.2.5 states the spacing beside a tate-chu-yoko run in its own words, in four amounts:

> In principle, when tate-chu-yoko is set after a comma (cl-07) or closing bracket (cl-02),
> or before an opening bracket (cl-01), half em spacing is added. In addition, when
> tate-chu-yoko is set after a full stop (cl-06) in the middle of a line, half em spacing
> is added. […] When tate-chu-yoko is set before full stops, commas or closing brackets, or
> after opening brackets, the inter-character spacing is set solid.

and then hands the subject to Appendix B, saying that the details are

> described as a complete table in §B.

Table 1's cl-30 row and column state those four **and six more**: a quarter em against a
middle dot (cl-05) in both directions, and against cl-21, cl-24, cl-25 and cl-27 in both
directions. Two statements of one rule that are not equivalent, and §3.2.5 does not say
which of them is the exception — the prose does not say "and nothing else", and the
sentence pointing at the table does not say "except for what is stated above".

The second silence is narrower and no sentence reaches it. §3.8.3's reduction ladder reads
Table 3 and §3.8.4's expansion ladder reads Table 6, and both matrices state amounts at
cl-30 coordinates where §3.2.5's prose put **no space at all**: Table 3 states `1/4-0
stage 4` at `(cl-30, cl-05)` and Table 6 states `1/4-1/2 stage 2` at `(cl-30, cl-27)`.
Whether a ladder may take back or add to a space that was never set is a question about the
relation between §3.2.5 and Appendix E, and neither states it.

The same question is asked wherever a *different* section spends an amount Table 1 states,
and §3.6 is the other place it happens. A tab sign takes the distance from where it stands
to its stop, and the Table 1 amount after the character before it is inside that distance:
§3.6 has made the amount the sign's. Whether the reduction and expansion ladders still see
a cell there — measuring their room in an amount another section has already spent — is the
cl-30 question again with a different section on the other side of it, and §3.6 says as
little about Appendices D and E as §3.2.5 does.

## The reading

**§3.2.5's prose is the whole of the spacing beside a run, and the sentence pointing at
Appendix B is not.** The four amounts the section states are set. The six further amounts
Table 1's cl-30 row and column state are not.

**The reduction and expansion ladders read their matrices at face value at the same
coordinates.** Table 3's `1/4-0 stage 4` at `(cl-30, cl-05)` and Table 6's `1/4-1/2 stage
2` at `(cl-30, cl-27)` both apply, even though §3.2.5 set no space there for §3.8.3 to take
back. The observable consequence is that a run on a line that had to give space back ends
up a quarter em *inside* the character before it.

The expansion **ceiling**, by contrast, is measured against the space §3.2.5 actually set
and not against Table 1's. The `tate-chu-yoko` census pins that distinction at 156 of its
requests.

**A ladder reads the cell after a tab sign as transcribed too.** §3.6 spends the amount —
it is inside the distance the sign takes to its stop — and Appendices D and E go on
measuring their room in it, exactly as they go on reading the six cl-30 cells §3.2.5
withdrew. The reading is one reading at both coordinates: a matrix cell is read at the
coordinate it is addressed by, whatever section has spent the amount it names.

A third coordinate has the same shape and is not published here, because the question it
raises is about §B.2 note 13 rather than about a ladder: a Western word space the line
collapsed, whose Table 1 cells the ladders also go on reading. It is recorded in
[engines/racket/README.md](../../engines/racket/README.md) until the collapse itself is
settled.

## Why

**A section that states four amounts in its own words has decided them.** §3.2.5 is not a
summary of Table 1's cl-30 row; it is a rule with its own conditions — "in the middle of a
line" for cl-06 is a condition Table 1 has no column for — and its four amounts are stated
with the modal force JLReq uses for rules it means. The sentence pointing at Appendix B
says where the *complete* table lives, which is true of every section of §3: Appendix B is
where a reader goes for a coordinate §3 did not name. Reading it as an override would make
§3.2.5's own four sentences redundant, and a specification that states a rule twice is
better read as stating it once and pointing at the general table than as contradicting
itself.

**A ladder is a reading of a matrix, and a matrix cell is not conditional on the section
that placed the space.** Tables 3 and 6 are addressed by class pair alone (ADR 0021): a
cell names two classes and no neighbor, no section and no provenance for the space it
governs. An engine that skipped a cl-30 cell because §3.2.5 rather than Table 1 decided the
boundary would be reading a condition into the matrix that the matrix does not carry, and
would have to answer the same question at every other coordinate where a section states an
amount in its own words. Taking the cells at face value is the reading that needs no such
rule.

**The ceiling is a different kind of quantity from a cell.** A cell states an amount; a
ceiling states how far a *placed* space may grow. Measuring the ceiling against a space
Table 1 states but §3.2.5 did not place would cap a quantity that is not there. That is why
the two halves of this reading are not inconsistent: the cells are read from the matrix, and
the ceiling is read against the line.

**A second section spending the amount changes nothing about the cell, which is the point.**
§3.6 and §3.2.5 do different things to the same kind of quantity — one withdraws it, the
other absorbs it into a distance of its own — and if the ladders had to know which, they
would need a coordinate for *why* an amount is where it is. Tables 3 through 6 carry two
classes and nothing else. Reading a cell at a tab boundary is therefore not a second
decision beside the cl-30 one; it is the same decision reached again at the only other place
in JLReq where a section spends a Table 1 amount, and reaching it twice by the same argument
is what makes it a reading of the matrices rather than a rule about tate-chu-yoko.

## What would change it

A revision of §3.2.5 that says either "and no other spacing is added" or "the amounts in §B
apply in addition" settles the first reading in one sentence, and is the change most likely
to arrive, because the divergence is visible to any reader who compares the section with the
table. Evidence that publishers set the six further amounts in practice — a document that
puts a quarter em between a tate-chu-yoko run and a middle dot — would be recorded as a
`disagreements` entry on a conformance case for `(cl-30, cl-05)` rather than as a change to
this reading, because both outputs would then be attested and neither would be a defect.

A revision of §3.8.3 or §3.8.4 that scopes a ladder to the boundaries the sections of §3
actually set would settle the second, and would take a `neighbor` or a `provenance`
coordinate that Tables 3 through 6 do not have today. The same revision settles the tab
coordinate, and would have to name §3.6 as well as §3.2.5 to do it — which is the clearest
statement of why the two are one reading: a change that answered only one of them would
leave the matrices addressed two ways.
