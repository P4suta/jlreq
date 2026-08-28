<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: what an Appendix A Remarks cell naming only an advance the format cannot express does

- Applies to: [`spec`](../../crates/jlreq-core/src/spec.rs), and the same lookup in
  [`engines/ocaml/lib/spec.ml`](../../engines/ocaml/lib/spec.ml)
- Standing: `Unstated`
- JLReq: §A.03, §A.24, §A.25, §A.26, §A.27, §3.9.2
- Observed by: `just census vertical`, which is where the two affected keys first surfaced

## The silence

Appendix A lists a key under a class, and its Remarks cell qualifies the listing. Several
cells qualify by advance alone: `字幅は四分角` (a quarter em) and `字幅は三分角` (a third of an
em) name widths that the protocol's `frame` vocabulary — `full-em`, `half-em`,
`proportional` — has no word for.

Two readings of such a cell are coherent and Appendix A distinguishes neither:

- **"No width stated."** The qualification names nothing the caller can be held to, so it
  qualifies nothing and the listing is available at every frame.
- **"A width no caller can declare."** The qualification names a real width, the format
  cannot express it, so no caller ever satisfies it and the listing is available at none.

This is not the same question as [grouped-numeral-qualification](grouped-numeral-qualification.md),
which asks what reaches cl-24 where §A.24's Remarks names a *job* as well as a width. Here
the cell names a width and nothing else.

## The reading

**A Remarks cell naming only an advance the protocol cannot express excludes its listing,
rather than qualifying nothing.** The second reading, at both keys where the two part:

- **U+0020 SPACE** is listed as a grouped numeral (§A.24) and as a unit symbol's character
  (§A.25) at a quarter em, and as the Western word space (§A.26) unqualified. Under this
  reading the first two listings are unavailable at every frame the format can state, so
  U+0020 stays **cl-26** however the caller labels the occurrence.
- **U+2010 HYPHEN** is listed as a hyphen (§A.03) at a quarter em and as a Western character
  (§A.27) proportional. Under this reading a proportional hyphen is **cl-27**.

## Why

**The alternative makes a qualification into its own opposite.** Reading `字幅は四分角` as "no
width stated" takes a cell that *narrows* a listing and makes it *widen* one: the listing
becomes available at every frame, including the full em and the half em that the cell was
plainly written to exclude. A cell that says "this key is in this class when it is a quarter
em wide" cannot coherently be read as "this key is in this class at any width", and the fact
that the format has no word for a quarter em is a fact about the format rather than about
Appendix A.

**Both consequences are the ones a reader would want.** A full-em U+0020 is an ideographic
space, and calling it a grouped numeral because the caller declared a
`digit-group-separator` role would be a classification driven by a label rather than by what
is on the page. A proportional U+2010 is a Western hyphen, and keeping it cl-03 at a
proportional frame would apply the Japanese hyphen's spacing to a Western one. Under the
"qualifies nothing" reading both of those follow, and both were live defects: they surfaced
as census differences at M4 and M5 respectively, in the two directions.

**Two keys tell the readings apart, and both are observable.** That is what makes this a
policy and not a matter of taste. The `vertical` census reaches U+0020 as three different
listings and U+2010 as two, and a class difference at either key changes Table 1's row and
column and therefore the line.

## What would change it

A `frame` vocabulary that could express a quarter em and a third of an em would dissolve the
question: the cells would become ordinary width qualifications and the listing would be
available at exactly the frames they name. That is a change to
`crates/jlreq-conformance/protocol.schema.json` rather than to JLReq, and it is the change
that would settle this for every engine at once — at the cost of a `frame` vocabulary with
five members that only Appendix A ever uses.

Short of that, a revision of Appendix A stating whether a width qualification the reader
cannot express is a restriction or a remark would settle it directly. Evidence that a
publisher treats a full-em U+0020 as a grouped numeral in running text would be recorded as a
`disagreements` entry on a conformance case for that key rather than as a change here.
