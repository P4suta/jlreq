# ADR-0022: one public Rust crate and one process conformance contract

- Status: superseded by [ADR 0025](0025-three-product-layers.md)
- Date: 2026-08-14
- Supersedes the crate topology in [ADR 0015](0015-the-crate-graph-and-the-inline-line-seam.md)
  and its seam-carrier amendment in
  [ADR 0020](0020-the-seam-carries-no-rule-address.md).
- Amended by [ADR 0023](0023-the-project-is-named-jlreq.md): the unified crate and the
  process contract below are named `jlreq` and `jlreq-conformance`, and the protocol
  identifier is `jlreq.conformance/1`. The topology this ADR decides is unchanged.

## Context

The pre-1.0 implementation split classification, spacing, line composition, constructs,
units, specification addresses, conformance, and a facade into separate crates. That graph
was useful while discovering ownership, but it exposed implementation stages as products.
Callers had to connect lowering, feasibility, adjustment, and placement themselves, and
types such as rule addresses and internal indices became compatibility obligations.

Conformance has a different audience from the Rust API. An engine written in another
language must be testable without reproducing Rust types or jlreq's private phases.

## Decision

The only intended public library is `jlreq`, a dependency-free `no_std + alloc` crate.
The workspace remains unpublished at `0.0.0`. Its private modules follow the one-way graph
in [ARCHITECTURE.md](../../ARCHITECTURE.md), but module boundaries are not products and no
compatibility layer preserves the old crates.

The only second public contract is the versioned NDJSON process protocol implemented by the
binary-only `jlreq-conformance` package. It exchanges pre-shaped input and observable
placements and diagnostics. Classification, rule addresses, adjustment stages, and search
mechanics never cross either public boundary.

The Rust crate and CLI share a release version but not a dependency surface: JSON and
reference-integration dependencies remain outside `jlreq`. The `api`, `direction`,
`purity`, `conform`, and `repository` gates hold these boundaries mechanically.

## Consequences

Users have one validated paragraph builder and one infallible composition call. Internal
algorithms and provenance can evolve without multiplying public crates. The candidate 1.0
Rust names and protocol-v1 messages are mechanically controlled during development, but do
not become stable compatibility contracts until an explicit release decision.

Historical ADRs and changelog entries retain their old crate names because they record the
reasoning that produced the implementation. Current indexes and architecture documents point
to the unified owners so those historical names are never mistaken for installable products.
