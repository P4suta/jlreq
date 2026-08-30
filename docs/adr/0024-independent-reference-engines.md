# ADR-0024: independent reference engines

- Status: accepted
- Date: 2026-08-23

## Context

[ADR 0006](0006-conformance-suite-as-artifact.md) makes the conformance suite a published
artifact precisely so that "another implementation can supply another [trait
implementation] and run the same suite." Three years into this repository, that claim had
never been tested: `jlreq-sample-engine` is the only engine that has ever spoken protocol
v1, and it is a thin adapter over the `jlreq` crate it ships beside. A protocol whose only
implementation shares a process, a build, and a standard library with the thing it is meant
to validate has not demonstrated language independence; it has asserted it.

[ADR 0022](0022-unified-public-crate-and-process-conformance.md) already drew the line the
protocol has to hold to: "an engine written in another language must be testable without
reproducing Rust types or jlreq's private phases." The request and response models in
`docs/design/conformance.md` were designed against that requirement — original UTF-8
ranges in, observable placements and diagnostics out, no classification, no rule address,
no internal stage — but a specification that has never been implemented twice is a
specification that has only been read once.

A second implementation is also the only affordable way to catch a specific class of bug:
a rule this project believes it implements correctly, and has a passing case for, but whose
behavior was never actually derived from JLReq or from the machine-readable `spec/`
tables — it was derived by reading `pipeline.rs`. [ADR
0009](0009-generated-data-and-attested-transcription.md) already applies double-entry
transcription to the roughly 5,400 hand-keyed cells of Tables 1 through 6 for exactly this
reason: a single reading, however careful, cannot distinguish "the specification says this"
from "the first implementation happened to do this." Extending double entry from
*transcription* to *behavior* means building a second engine that is barred from reading
the first one's source.

## Decision

`engines/` holds independent implementations of the protocol-v1 conformance protocol,
starting with `engines/ocaml/` and followed by `engines/racket/`. Each is built from
`spec/` and from the public protocol contract only
(`crates/jlreq-conformance/protocol.schema.json`, `docs/design/conformance.md`, the sample
engine's wire vocabulary); each is barred from reading `crates/jlreq-core/src/` — `pipeline.rs`
above all — and from `crates/jlreq-core/src/generated/` or the xtask generators that write it.
The rule, the integer contract both engines follow, and the milestone mechanism are
recorded in the new "Independent reference engines" section of
`docs/design/conformance.md`, which is their single shared source rather than something
restated per engine.

Neither engine is a Cargo workspace member. `engines/ocaml/` builds with `dune` from a
`dune-project` at the repository root (dune cannot depend on a file above its project
root, and the engine reads `spec/*.tsv` directly); `engines/racket/` builds with `raco`.
This keeps every gate whose scope is the Cargo graph — `purity`, `api`, `direction`,
`derive`, `generate`, `deny`, `shear`, `msrv`, `mutants`, `release-plz` — unaffected by
their presence, exactly as `fuzz/` already sits outside the workspace as its own
compilation unit. `xtask repository` and `reuse lint` do still see them: their Markdown
links must resolve and every source file needs an SPDX header, the same as anywhere else in
the tree.

Each engine reaches the full eighty-nine-case built-in suite through nine cumulative,
disjoint milestones rather than in one step. A `milestones/CURRENT` file names the
milestone the tree currently claims; the engine's CI job runs the cumulative suite through
that milestone, so `CURRENT` — not the engine's completeness in the abstract — is what a
pull request is actually held to. `conform-ocaml` and `conform-racket` join `ci-required`,
gated at whatever milestone is current, from the PR that adds the job. At
`CURRENT = 9` the cumulative suite is the full suite and the milestone gate becomes
identical to running all eighty-nine cases. `just conform-engines`, which `just ci` and the
pre-push hook run, wraps both engine gates with a toolchain check: a developer without an
OCaml or Racket toolchain installed gets a loud `SKIPPED` line and a green local run, while
CI has the toolchain and enforces the gate for real.

Both engines have now reached `CURRENT = 9` and pass all eighty-nine built-in cases. Their
generated ten-kind census additionally exercises 122,199 requests and records zero
differences among the Rust, OCaml, and Racket implementations; the committed
[conformance summary](../generated/conformance-summary.md) is the mechanically checked
record of that completed milestone.

Where the hand-keyed matrices are transcribed a second time, each reference engine reads
the opposite locale from the Rust sample engine: the sample engine reads
`spec/captured/table*.en.tsv`; `engines/ocaml/` and `engines/racket/` read
`spec/captured/table*.ja.tsv`. Agreement across all three is then evidence about the
transcription itself, not an artifact of two engines reading the same keystrokes.

## Consequences

A rule that is observable in the Rust engine's behavior but not written down in JLReq, in
`spec/`, or in `docs/decisions/` will produce a genuine three-way disagreement the first
time a reference engine reaches it, rather than silently propagating because the second
implementation copied the first. Settling such a disagreement means returning to the
specification and recording the resolution in `docs/decisions/`, never reading the other
engine's source and matching its answer, and never counting votes among whichever engines
happen to be running. Every one of these disagreements found this way is exactly the value
[ADR 0006](0006-conformance-suite-as-artifact.md) predicted a second implementation would
have.

Building and gating two more language toolchains is real, ongoing cost: `engines/` adds
OCaml and Racket to the CI matrix and to what a contributor working on those trees
needs installed locally, tracked in `CONTRIBUTING.md`. The milestone mechanism is what keeps
that cost bounded during development — a reference engine is gated on the portion of the
suite it actually implements, not on all eighty-nine cases from its first commit. Both
engines are now at `CURRENT = 9`, so the built-in suite is fully cross-checked.

`engines/` is deliberately not a path to a released product. Neither engine gains a stable
CLI contract, a version number, or a publish target; both stay tooling in service of the
the `jlreq`, `jlreq-core`, and `jlreq-conformance` products ADR 0025 names, the same
way `xtask` is tooling and
not a product crate. If a reference engine's design or its findings turn out to be broadly
useful outside this repository, that is a separate, later decision — not a reason to gate
one here on eventual portability.
