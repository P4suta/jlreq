<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# ADR-0028: the trace is a second observation channel, not a second carrier

- Status: accepted
- Date: 2026-09-06

## Context

This workspace could say what it produced and it could not say why. A `Layout` states the
answer. A `Diagnostic` states the handful of conditions a program is expected to branch on,
and there are exactly three of them in shipped code — `font.missing-glyph`,
`layout.overfull`, `layout.widow`. Everything between the input and the answer was computed
and dropped: which break candidates were weighed, what each one cost, which Table 1 cell
supplied an amount, which rung of the adjustment ladder absorbed a line's surplus.

The absence had left marks. `pipeline/adjustment.rs` carries `#[cfg(test)]` allocating
wrappers over its no-allocation forms — `prepare_line_adjustments`, `reduction_sites`,
`distribute_reduction`, `capped_round_robin`, `distribute_adjustment` — whose only purpose
is to let a test see an intermediate distribution. `CallState::charge_shape` in the facade
is a `#[cfg(test)]`-only counter for the same reason. The demand to see the middle of the
computation was real enough to be built twice, and both times it was built for the crate's
own tests and closed to everybody else, including the person debugging a layout at three in
the morning.

The reason to be careful about adding a channel is
[ADR 0019](0019-one-fact-one-carrier.md): a fact has one carrier, because two carriers of
one fact are two things that can disagree. That decision deleted `RubySize`, deleted
`Question::RUBY_ALIGNMENT`'s duplicate, and refused a `rule` field on a seam type in
[ADR 0020](0020-the-seam-carries-no-rule-address.md) on exactly that ground. A trace event
that restated a diagnostic would be the same defect wearing a new name.

## Decision

**The trace records the derivation, never the result.** Three channels, three subjects, no
overlap:

| channel | subject | audience | compatibility |
| --- | --- | --- | --- |
| `Layout` | what was produced | a renderer | frozen for the release line |
| `Diagnostic` | a condition worth branching on | a program | frozen; codes are the key |
| `Trace` | how the engine got there | a person | free to move |

ADR 0019's own test is the one applied here: it collapses "two carriers of the *same*
fact" and keeps "two facts that merely constrain each other". A diagnostic saying a line is
overfull and a trace saying what the line measured against what was available are the
second kind. Neither is derivable from the other — the trace has no notion of which
conditions deserve a caller's attention, and the diagnostic has no notion of the search
that produced the line — and the trace is not consulted by any code path that computes an
answer.

Two mechanical guards rather than a promise:

1. **Disjoint namespaces.** `Fact::kind` names live in `prepare.`, `search.`, `break.`,
   `space.`, `reduce.`, `expand.`, `hang.`, `ruby.`, `jidori.`, `warichu.`, `furawake.`,
   `tcy.`, `tab.`, `line.`, `place.`. None of these is one of the seven product
   error-code namespaces `xtask repository` holds against
   [docs/error-codes.md](../error-codes.md), and a unit test asserts it. A trace name can
   therefore never be mistaken for a compatibility key, and the gate's scan can widen
   later without swallowing the trace.
2. **No restatement.** There is no `Fact::Overfull` and no `Fact::Widow`. Where `place`
   emits the overfull diagnostic, the trace's contribution is the line's measured extent
   against its measure — the geometry that *produced* the diagnostic, not a second
   announcement of it.

**The trace is outside the compatibility contract, and says so in its own documentation.**
Event shapes, their order, and their rendering may change in any release.
[ADR 0012](0012-outcome-and-detail-compatibility.md) requires an output enum that grows to
be paired with a frozen total projection; `Fact` is `#[non_exhaustive]` and pairs with
`Fact::kind`, `Fact::category` and `Fact::jlreq`, each a wildcard-free `match`. The absence
of a `_ =>` arm is the mechanism: a new variant does not compile until all three tables and
the rendering name it.

**Recording is a runtime choice, not a compile-time one.** `compose` and `compose_traced`
run one body; the untraced entry passes a sink that records nothing and allocates nothing.
A `#[cfg(feature)]` would have produced two compiled paths where only one is validated: the
three-implementation census in [ADR 0024](0024-independent-reference-engines.md) needs
OCaml and Racket toolchains that `mise` deliberately does not manage, so it runs only from
a manually dispatched workflow, and "core behavior on existing input does not move" is a
promise nothing was holding between releases. Now something does — `compose` and
`compose_traced` are held to the same layout, the same error, and the same charged
transition count, and committed goldens hold the reasoning itself.

**The rule address is the inventory the diagnostics already use.** `Diagnostic::jlreq` is a
`&'static str` naming a section. `RuleAddress` names the same inventory in the grammar
[ADR 0013](0013-rules-are-addressed-by-specification-address.md) fixes, and additionally
renders the cell form — `B.1@cl-05,cl-05` — which nothing in the workspace previously
rendered even though `xtask conform` has validated it since the trims validator was
written. This is not the second provenance mechanism ADR 0020 refused: that decision
concerned a `rule: RuleId` field on the retired `Segment`/`Separation` seam types, beside
an `Answer<T>` that already carried up to three addresses and a `Standing`. There is no
`Answer<T>` here and no second mechanism; there is one public rule-address channel on
results, and the trace uses the same inventory to say which sentence it acted on.

**A phase decides which stage may speak.** Preparation, search, and placement all reach the
same ladder and ruby helpers, and only placement is setting a line. Preparation in
particular calls them with a boundary of zero for every cluster, because it is summing a
paragraph rather than composing a line; an unphased site event from there would not merely
repeat itself, it would name a boundary that is not one. Kinsoku is the exception and is
admitted during preparation, because a kinsoku refusal names the boundary it refused.

Nothing is lost by the suppression. Every candidate the search weighs carries both its
natural width and its width after the available reduction is spent, so the capacity the
search assumed is their difference — a subtraction the reader performs rather than a
paragraph of events restating it.

**A budget, not a resource.** A trace stops recording at a ceiling and says it was
truncated. Truncation is deliberately not a `ComposeError`: `CompositionResource` is public
surface with a documented code, and a debugging aid must not be able to turn a composable
paragraph into a refusal.

**One document, one trace.** The core cannot see face selection, grapheme itemization, or
paragraph segmentation, so it cannot answer the question a caller asks most often: why did
this glyph come from that font. `jlreq::trace::DocumentTrace` records those, and absorbs
each paragraph's core trace with its byte offsets shifted into document coordinates, rather
than handing a caller two channels to reconcile by hand. The absorbed line is rendered
exactly as the core rendered it — paragraph-local offsets included — because rewriting a
core line would make the two goldens disagree about the same decision; the facade's own
`para.segment` states the paragraph's document range, which relates the two frames.

The facade's categories are a second, separate set. That is not symmetry for its own sake:
no facade family grows faster than the input, so its default records everything, while the
core keeps a default that excludes the two families that are superlinear.

Absorption happens *before* the composer's result is unwrapped. A paragraph that refuses is
the case whose reasoning a reader most needs, so a refusal must not take the trace with it.

## Consequences

The question a user actually asks — why did this line break here, why did this space
shrink, why did this glyph come from that font — has a mechanical answer for the first
time, and the answer cites the sentence it rests on.

The project gains the drift oracle it could not otherwise have. `crates/jlreq-core/tests/`
and `crates/jlreq/tests/` hold rendered traces byte for byte, in suites that run on every
push across three operating systems, where the census runs on none. A change that reorders
the ladder or charges a different surcharge moves a golden even where the final geometry
agrees, and geometry agreeing by coincidence is precisely the case that would otherwise
ship.

The two `#[cfg(test)]` inspection hatches become a stated channel rather than a private
one. They remain for now, because removing them is a separate change with its own risk.

The trace costs a load, a mask, and a not-taken branch per site when it is off, and no
allocation at all. That is the price of one implementation instead of two, and it is the
right one to pay while the census cannot be re-run.
