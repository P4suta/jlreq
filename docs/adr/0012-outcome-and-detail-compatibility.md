# ADR-0012: kumihan may add detail, never an outcome

- Status: accepted
- Date: 2026-08-05

## Context

The roadmap adds capability at every milestone after M0. Optimal breaking arrives at M3,
the inline constructs at M4, vertical writing at M5, and each of them enlarges types the
public API already exposes. An adopter who integrates at M1 must still be there at M5, or
[ADR 0003](0003-layer-above-icu4x.md)'s adoption argument is lost: nobody adopts a library
that demands a rewrite every milestone.

The usual answer is `#[non_exhaustive]` on everything. It is necessary and it is not
sufficient, and the way it is insufficient is the reason this decision exists.
`#[non_exhaustive]` on an output enum forces every caller to write a catch-all arm. When a
later version adds a variant, the caller's code still compiles and now silently ignores the
new case. For a library whose entire value is that wrong Japanese should be loud, shipping
a compatibility regime whose mechanism is silent fallthrough would be self-defeating.

Semantic versioning promises that code still compiles. What an adopter needs is that code
still means what it meant.

## Decision

One rule governs input types, output types, and policy types alike: kumihan may add detail,
and may never add an outcome.

Every public type is open in the direction kumihan writes it and closed in the direction
the caller reads it. Input types are `#[non_exhaustive]`, obtained from a named constructor
and configured by consuming builder methods, so a new field never breaks a call site.
Policy is opaque, so growing the set of questions is invisible. Output types are
`#[non_exhaustive]` too, and each open enum is paired with a frozen projection: a total
accessor answering the question the caller actually has, whose answer set may never grow.

Whether a line may end at a boundary projects to a boolean. Whether a line fits projects to
a boolean. Whether a classification is ambiguous projects to a boolean. A new variant
recording a further reason a break is refused is detail, because the projection still says
no and every caller's behavior is unchanged; that is a minor release. A variant meaning
"conditionally breakable" would be an outcome, and it is forbidden forever — if the
specification ever needed one it would be a new type and a new function, because silently
changing what an existing caller's code means is exactly the failure this library exists to
prevent.

The pairing is recorded in `docs/api-frozen.toml`, which exists from M0 with its full
contents rather than being written after the types it governs — a gate authored against
types that already exist is a gate written to pass them, which inverts its purpose. It has
four tables. `[[frozen]]` names each output enum and the total accessor that projects it,
whose answer set may never grow. `[[exempt]]` names each public type allowed to be
exhaustive, with the sentence of the specification that closes it. `[[no_impl]]` names each
type and the traits it may never gain, which is where
[ADR 0010](0010-prohibition-is-not-a-penalty-value.md)'s structural claim and
[ADR 0011](0011-typed-axes-and-direction-as-a-datum.md)'s axis separation are held.
`[[forbidden]]` names shapes that may never appear in the public surface at all, which is
where the two ADRs that were otherwise held only by review acquire a gate: a core item whose
name says it measures something ([ADR 0002](0002-caller-supplied-metrics.md)), and a
classification function whose whole parameter list is a code point, which
[ADR 0008](0008-classification-is-a-function-of-an-occurrence.md) proves cannot exist and
which is the first thing an adopter in a hurry would reach for.

The gate checks four things: that no public type is exhaustive without an exempt entry, that
every named projection still exists, that every `#[non_exhaustive]` type in an input
position has a named constructor — otherwise the type is unconstructible and the compatibility
regime has quietly made the API unusable — and that no forbidden shape appears. The file is
owned by the code owners, so relaxing any of it is a reviewed decision by construction rather
than an attribute somebody deleted.

"Input position" is defined mechanically here, before the gate that reads it is written,
because a predicate settled afterwards is settled by whatever the code happened to do. A
public type is in an input position when it appears in the parameter list of any public
function in the workspace other than as the receiver, including inside a reference, a slice,
a range, an option or a result. A named constructor is an associated function returning
`Self`, or `Result<Self, _>`, or `Option<Self>`. Applying that predicate to this design at
the moment it was written found four types that failed it — the ruby run a caller must
supply, the block demand a sibling crate must build, and the answer and provenance every
layer produces — which is the argument for pinning the predicate now in one sentence rather
than discovering it in a gate authored later.

The two forbidden tables are read as narrowly as they are written, and that is stated so
neither is quietly widened into a gate that fails on the design it protects. The name guard
matches the identifiers of declared public items — functions, types, constants, modules,
traits — and never parameter names, field names, or keys in a data format. A line measure is
an input the caller states; a function named for measuring is the thing ADR 0002 forbids;
they are not the same word doing the same job, and only the second is caught.

The exhaustive set is short and each member is closed by the specification rather than by
us — the thirty character classes, the two axis positions of Table 1, the referent pair,
the three cell tokens of Table 2, the three reduction tables, the four strictness levels. A
caller matching all thirty classes must not be forced to write a catch-all, because a
catch-all over character classes is precisely where a silently wrong default hides.

## Consequences

Every milestone after M0 is a minor release. M3 adds a strategy variant, M4 adds
constructs and policy questions, M5 adds a direction value, and an M1 adopter recompiles
without editing.

Callers cannot write struct literals for inputs. That is the cost, it is paid once at
integration, and it is what buys the rest.

The one change that is forever breaking is the fixed-point unit of
[ADR 0007](0007-two-scalars-and-the-fixed-point-unit.md), which is why that decision is
made now.
