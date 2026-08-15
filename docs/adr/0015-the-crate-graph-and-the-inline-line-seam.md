# ADR-0015: two crates below the core, and the line layer owns every break

- Status: superseded by [ADR 0022](0022-unified-public-crate-and-process-conformance.md);
  retained as the historical derivation of the construct/composition seam
- Date: 2026-08-05

ADR 0020 corrects two sentences below. `jlreq-spec` depends on `jlreq-unit`, so the two are
not at the same depth; and the seam types carry no rule address, so "everything crossing the
seam lives in `jlreq-unit`" holds without the edge this document assumed it needed none of.

## Context

[ARCHITECTURE.md](../../ARCHITECTURE.md) draws a six-crate graph with `jlreq-class` at the
bottom and `jlreq-line` and `jlreq-inline` as siblings above `jlreq-spacing`. Working the
specification through, one of its edges is wrong, one edge that looks necessary turns out
not to be, and a fixpoint hides behind a third — each for a reason the specification states.

`jlreq-class` cannot be the bottom. It needs the fixed-point unit, because §3.1.6 makes an
intrinsic advance a per-member property rather than a per-class one — cl-03 alone carries a
quarter em, a half em, and a full em across its four members. It also needs the policy
space, because §C.2 notes 1 through 3 make a strictness relaxation a *reclassification*: a
`々` permitted at the line head "shall be treated as a member of the ideographic character
(cl-19) class", which changes its answer in all six matrices. Policy therefore has to reach
the classifier and cannot live above it.

The tempting edge is `jlreq-inline` depending on `jlreq-line`, on the argument that §3.4.2
defines warichu (割注) as two lines of small characters broken "at a position where line
breaking is permitted, and where the two resulting lines are as close as possible to the
same length", using "the same line breaking rules as for basic text" — so the construct
layer would call the line layer to compose the interior. §3.4.3 falsifies it. When the
warichu does not fit in what is left of the current line it wraps onto the following one,
and the Japanese note records straddling two lines as 頻出, frequent, rather than exotic.
The interior's available measure is therefore not one number and is not known until the
outer break is chosen. A construct layer that holds neither the measure nor the outer search
cannot compose that interior at all, so the edge does not buy what it appears to buy.

The specification then says the same thing three more times with different constructs.
§3.7.2 sets furiwake (振分け) as several sub-lines inside one line, top-aligned, split at
declared positions, and states flatly that one furiwake block must not extend across
multiple base text lines. §3.7.3 sets jidori (字取り) as one run whose inter-character
spacing is adjusted to fill an explicitly specified length. §3.2.5 sets tate-chu-yoko
(縦中横) left to right and centers the whole string on the vertical line, so its interior is
laid out on an axis the outer line does not own while it occupies one em of that line. Four
constructs, one shape: a span of items the line layer does not lay out as ordinary inline
text.

A fixpoint hides behind the ruby edge, pointing the other way. Appendix B's legend says ruby
may extend over a `hang` space "as long as it is not reduced due to line adjustments. When
it is reduced, ruby text can be extended up to the size of the reduced spacing." The
overhang allowance is therefore not known until the line is adjusted, while the item extents
that drive the adjustment come from the constructs.

## Decision

Two crates are added below the existing six. `jlreq-unit` holds quantities, axes, and the
item vocabulary. `jlreq-spec` holds the specification-reference vocabulary: rule addresses,
provenance, and the policy space. They are two rather than one because they are unrelated
concepts that merely sit at the same depth, and because `jlreq-conform` must reach the rule
inventory to report coverage without pulling in the whole facade; merging them would produce
a crate named after lengths that contains several thousand generated JLReq headings.

`jlreq-unit` holds the item *vocabulary* and not the text built from it. A text's validity is
a statement about Appendix A ([ADR 0018](0018-an-item-is-one-occurrence.md)), so the type
lives in `jlreq-class`, where the table it must be checked against is. Every crate that names
a text already depends on `jlreq-class`, so this costs no edge and adds no crate; what it buys
is that the constructor is at the depth where it can enforce its own invariant instead of
documenting it.

`jlreq-line` and `jlreq-inline` are siblings, and **every break selection in the workspace
happens in `jlreq-line`** — the outer paragraph's and every nested one's, over one
feasibility computation and one adjustment ladder. The line layer gains one
construct-neutral concept, a *segment*: a span of items at its own size whose interior is
either laid out on an axis this line does not own, or filled to a stated extent, or split
into balanced sub-lines, or split at declared positions. Tate-chu-yoko, jidori, warichu, and
furiwake are those four cases in order, and the line layer can be read end to end without
meeting any of their names. Straddling is a property of the balanced case (§3.4.3) and
forbidden in the other three (§3.2.5, §3.7.2, §3.7.3), so the search that knows the
remaining measure is the search that decides it.

The seam is data, and it has a producer and a consumer. `jlreq-inline` lowers the caller's
declared constructs into four things: per-item run identity, the segments, the least spacing
a construct forces at a base-text boundary (§3.3.8 rule 1, where ruby longer than its base
pushes the bases apart), and block-axis demand. Composition consumes exactly those, as a
builder step on the paragraph. Nothing else crosses, and every type that does lives in
`jlreq-unit`, so neither crate names a type the other owns.

Run identity has one carrier and one rule. It is *not* a field of an item, because an item
is what the caller measured and a run is what lowering computed, and two carriers of one
fact are two things a caller can desynchronize — the principle
[ADR 0019](0019-one-fact-one-carrier.md) generalizes. The overlay is built by a validating
constructor that checks that each identity names one contiguous span and that no two kinds
share one, so a caller with its own construct model may build it directly — a real
capability rather than a loophole — and uniqueness is enforced rather than promised.

Lowering allocates the identities, and it publishes the map back. A caller that used `lower`
did not invent the identities it sees in the output, so the contribution answers, for any of
them, which construct kind and which position in the slice the caller passed it came from,
and every error and every placed annotation names the construct that way rather than by a
bare ordinal. Requiring the caller to allocate the identities instead would have created a
second identity space to keep in sync with the slices it already holds, which is the defect
this paragraph opens by refusing.
Lowering does not remove break candidates either: a candidate inside an indivisible
construct is refused by the ordinary same-run predicates of §C.2 notes 6 through 8 and 13,
in the crate that owns break refusal, which is what puts the refusal in the rejection report
with its rule like every other.

The fixpoint is resolved by splitting it rather than by iterating. `jlreq-line` owns both
halves it needs — the `hang` permission comes from the tables, the surviving space from its
own adjustment — so it resolves the overhang *allowance* and reports it per boundary.
`jlreq-inline` places annotations against an allowance it is told. There is no edge back.

The purity gate stops checking mere core membership and checks this exact adjacency, over a
crate list derived from the workspace members with an explicit non-core denylist — so a
crate added without being placed in the graph fails the gate instead of being skipped. The
same gate checks that the seam is connected: every type listed as crossing it must appear in
the signature of a producer and of a consumer, so a seam with nothing on the far end fails
rather than passing silently.

## Consequences

There is one break-selection implementation for four nested constructs and one paragraph,
which is [ADR 0004](0004-writing-mode-abstraction.md)'s argument applied to a second axis.
The alternative was a second breaker that drifts, and the direction it would have drifted in
is not implementing §3.4.3 at all.

The facade orders the two layers — lower, compose, place — and that is composition of
layers, which is the only thing it is for. It contains no rule.

ARCHITECTURE.md's boundary table and [ADR 0001](0001-no-std-no-io-no-font-in-core.md)'s list
of core crates both grow by two. Every new crate costs six coordinated edits — the workspace
members, the gate's adjacency table, the `Justfile`'s crate list and its MSRV recipe, which
enumerates paths rather than globbing, and a release-plz package block. That price is paid
twice here and should be refused a third time without an argument this strong.

A consumer that needs only classification still never pulls in line composition, which is
what the original table protected. Adding crates below it strictly improves that property: a
consumer that needs only the quantities now pulls in nothing at all. And a consumer that
needs annotation placement no longer pulls in the line breaker, which the rejected edge
would have forced on it.
