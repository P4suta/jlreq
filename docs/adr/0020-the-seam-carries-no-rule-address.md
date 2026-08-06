# ADR-0020: the seam carries no rule address, and `jlreq-spec` depends on `jlreq-unit`

- Status: accepted
- Date: 2026-08-06

## Context

[ADR 0015](0015-the-crate-graph-and-the-inline-line-seam.md) gives `jlreq-unit` and
`jlreq-spec` an empty dependency row each, on the ground that they are "unrelated concepts
that merely sit at the same depth". The API spine's own signatures falsify that, in two
places pointing opposite ways, and the contradiction is not visible from either one alone.

`jlreq-unit` cannot satisfy its half. The spine writes
`Segment::new(items, scale, interior, rule: RuleId)` and
`Separation::new(after, least, rule: RuleId)`, and `RuleId` is declared in `jlreq-spec`. So
the seam types need the edge `jlreq-unit -> jlreq-spec`.

`jlreq-spec` cannot satisfy its half either. [ADR 0019](0019-one-fact-one-carrier.md) makes
`distribute`'s remainder argument legitimate only because "every call site inside the
workspace obtains that argument from one function over `Policy`"; the spine writes that
function as `Policy::remainder(self) -> RemainderRule`, and `RemainderRule` is declared in
`jlreq-unit`. So the policy space needs the edge `jlreq-spec -> jlreq-unit`.

Both edges together are a cycle. Cargo refuses it, and the design would deserve the
refusal: a crate named after lengths that reaches the rule inventory and a crate named
after the specification that reaches the arithmetic is not two layers.

The failure mode of leaving it open is worse than either edge. The two halves are met by
two different later milestones — M4 writes the segment, M1 writes the policy space — so
each would meet the wall alone, and each would invent a local answer, which is how one
coherent design becomes two. It was in fact discovered by deleting one endpoint on each
side: `Segment` and `Separation` were silently omitted from `jlreq-unit`, leaving `Interior`
and `Straddle` exported with nothing in the workspace holding one, and `Policy::remainder`
was silently omitted from `jlreq-spec`, leaving `RemainderRule` with no derivation from a
policy at all.

## Decision

The edge is `jlreq-spec -> jlreq-unit`, and the seam types carry no rule address.

**The direction is forced by two documents that are not being reopened.** ADR 0019 states
that "`distribute` lives in `jlreq-unit`, which does not depend on `jlreq-spec`", and
ADR 0015's own consequence is that "a consumer that needs only the quantities now pulls in
nothing at all". The opposite edge contradicts both sentences; this one contradicts
neither, and costs `jlreq-conform` a two-crate closure instead of a one-crate closure where
the alternative was the whole facade.

**`Segment` and `Separation` therefore drop their `rule` field.** Nothing is lost, because
provenance already has a carrier in this workspace and it is strictly richer: `Answer<T>`
carries up to three `RuleId`s *and* the `Standing` of the chain, and both crates at the
seam — `jlreq-inline` and `jlreq-line` — already depend on `jlreq-spec`. `jlreq-inline`
produces `Answer<Segment<'_>>` and `Answer<Separation>`; both are `Copy`, so this costs no
allocation and no edge. A `rule: RuleId` beside them would have been a second provenance
mechanism in a workspace that has one, which is exactly what
[ADR 0019](0019-one-fact-one-carrier.md) forbids — the argument applies to a field of a
seam type as much as to a field of an item.

**The edge is declared in the manifest when the code needs it.** `Policy::remainder` reads
`Question::REMAINDER`, a named constant emitted beside the generated policy space, so the
function itself arrives with `spec/derived/questions.tsv`. Until then `jlreq-spec` names no
`jlreq-unit` type and declaring the dependency would fail `cargo shear`. The crate graph in
`docs/design/api-spine.md`, in `ARCHITECTURE.md` and in `xtask`'s `CRATE_GRAPH` is the
*permitted* adjacency, and it carries the edge now, so the commit that lands
`Policy::remainder` is checked against a decision already made rather than making one under
schedule pressure.

**The purity gate stops being silent about an absent seam type.** Each row of its seam table
now carries the milestone the spine places the type in; a type no crate declares is a
violation at or past that milestone and a printed note before it. A gate commissioned to
prove that "the seam is connected" (ADR 0015) must not print that sentence over two rows it
never looked at, which is what let this contradiction sit unresolved while `just purity` was
green.

## Consequences

`jlreq-unit` still depends on nothing, so ADR 0015's strongest consequence survives
verbatim. `jlreq-spec` is one edge deeper than ADR 0015 drew it, and that sentence in
ADR 0015 — "unrelated concepts that merely sit at the same depth" — is now false of the
depth and still true of the concepts; it is corrected here rather than in place, because the
reasoning that produced it is worth reading as it was written.

Every rule an answer rests on now travels in exactly one shape, `Provenance`, from the
classifier to the placed annotation. A conformance case asserting why a segment is a segment
asserts on the same field it uses for every other answer, and there is no second spelling
for a reviewer to learn.

One more edge of this kind should be refused. Two crates at the bottom with one edge between
them is a layering; three with two is a chain that wants to be one crate, and the argument
for splitting them — that `jlreq-conform` reaches the rule inventory without the facade —
gets weaker with every edge added below it.
