# ADR-0019: a fact has one carrier, and the carrier is whoever measured it

- Status: accepted
- Date: 2026-08-06

## Context

[ADR 0015](0015-the-crate-graph-and-the-inline-line-seam.md) refused to put run identity on
an item, with the argument that "two carriers of one fact are two things a caller can
desynchronize". The argument is right and the design reproduced the defect four more times
without noticing, because each instance arrived through a different door.

Ruby size arrived through three doors at once. `Question::RUBY_SIZE` states it as a document
policy, `Ruby::with_size` states it per construct, and the ruby's own annotation stream
declares a `Scale` — because the caller shaped the reading at some size and measured it
there. Nothing reconciles a policy of "half" against a declared ruby em of 480 against a
base em of 1000. §3.3.8's overhang allowances are stated in ruby ems, so a disagreement about
what a ruby em is produces wrong overhang everywhere, silently, on the construct whose
placement rules are the most elaborate in the specification.

Ruby alignment arrived through two. `Question::RUBY_ALIGNMENT` and `Ruby::with_alignment`
both state it and neither is said to win.

The rounding remainder arrived through a public parameter. `Em::resolve_inline`,
`Em::resolve_block` and `ConditionalSpace::resolve` each took a bare remainder while
[ADR 0007](0007-two-scalars-and-the-fixed-point-unit.md) claimed the per-size bookkeeping was
"an invariant of the type rather than a discipline at its call sites". A remainder produced
against one size and spent against another is a different absolute length, and the three
signatures let exactly that be written.

The emphasis-dot advance arrived through absence, which is the same defect with the count
zero. §3.3.9 says "the center of emphasis dots is aligned with that of the base characters",
and centering a mark needs the mark's own advance. `EmphasisDots::new` took no advance, so
the library would have computed a position from a width it was never told — which
[ADR 0002](0002-caller-supplied-metrics.md) forbids, and which the `[[forbidden]]` name guard
cannot see, because nothing involved is named `measure`.

## Decision

Every fact the library uses has exactly one carrier, and three rules pick it.

**A fact the caller measured is carried by the measurement.** The ruby em is the annotation
stream's declared `Scale` and nothing else. `RubySize` and `Question::RUBY_SIZE` are deleted
rather than reconciled, because they were a second statement of a quantity ADR 0002 already
makes the caller's, and because §3.3.3 does not close the set they enumerated: it names half
the base size as the principle and one-third ruby (三分ルビ) as a variant, and then says that
for headings at twelve points or more the ruby "is generally smaller than half the size of
the base characters" with no ratio given at all. A two-or-three-valued type cannot state that
third case; a declared `Scale` states all three and is exact. §3.3.3's two named sizes stay
in the rule inventory and are exercised by conformance cases over declared scales, which is
where a claim about geometry belongs. The same rule settles the emphasis dot:
`EmphasisDots::new` takes the mark's advance, at the size §3.3.9 fixes.

**A fact the specification fixes is carried by the specification, and the caller does not
restate it.** §3.3.9 fixes the emphasis-dot size at half the base and the side at
block-start; neither is a parameter. §3.3.3's anisotropy is likewise not a parameter — it is
the caller's `Scale`, which has been anisotropic since ADR 0007 for exactly this reason.

**A fact that is genuinely a choice is carried by the policy, and a per-construct statement
of it overrides the policy for that construct.** That is the one precedence rule, it is
stated once, and it applies wherever both exist. `Question::RUBY_ALIGNMENT` is the document's
default; `Ruby::with_alignment` is this ruby's. There is exactly one place where the two are
reconciled — lowering — and every answer records which of the two applied, so a report says
whether an alignment was the document's or this construct's.

Where a lower crate cannot see the policy, the choice reaches it as an ordinary parameter and
there is one function that derives it. `distribute` lives in `jlreq-unit`, which does not
depend on `jlreq-spec`, so it takes the remainder rule directly; every call site inside the
workspace obtains that argument from one function over `Policy`, so the policy is still the
single carrier and the parameter is a transport rather than a second source.

Where a fact is bookkeeping rather than a choice, the carrier is a type and the parameter is
deleted. `Residual` leaves the public surface entirely. The two bridges and
`ConditionalSpace::resolve` take a `Size` — one character size together with its ordinal in
the text's own scale table, obtainable only from a text — and a `&mut Carry`, which keys the
remainder by that ordinal. A remainder produced at one size and spent at another is then not
an expression that can be written, which is what ADR 0007 claimed and did not hold.

## Consequences

Four contradictions become unrepresentable rather than diagnosable, so four diagnostics that
would have had to be invented are not. There is no `RubySizeContradictsScale` to match
`FrameContradictsAdvance`, because there is nothing left to contradict.

The policy space shrinks by one question and the ruby builder by one method, and the API is
easier to use for it: a caller that has shaped ruby has already answered the question the
deleted knobs asked.

One asymmetry is deliberate and is named so it is not read as an oversight.
`Frame` is a caller statement about an advance the caller also supplies, and ADR 0002 makes
the advance authoritative while
[ADR 0018](0018-an-item-is-one-occurrence.md) makes the frame required where it decides
geometry — so those two are not two carriers of one fact but a measurement and a statement of
what it covers, and their disagreement stays a diagnostic. That is the test this decision
applies: two carriers of the *same* fact are collapsed, and two facts that merely constrain
each other are both kept and their disagreement reported.
