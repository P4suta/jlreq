# Roadmap

## Current status: unreleased 0.0.0

jlreq has not reached 0.1. Both product packages set `publish = false`; there is no
released compatibility contract, support line, release date, or implied path to 1.0.
Passing the repository's implementation and conformance gates is evidence about the current
tree, not a release decision.

The current implementation is exploring a deliberately small eventual product boundary:

- one dependency-free `no_std + alloc` Rust library;
- one validated paragraph composition pipeline for caller-shaped UTF-8 clusters;
- nine inline constructs, horizontal and vertical placement, tabs, widow control, and
  whole-paragraph optimization;
- 22 typed JLReq 2020 Style choices; and
- a language-independent black-box conformance protocol.

The working specification identifier is `jlreq-2020-08-11+unicode-17.0.0`. Complete JIS X
4051 conformance is not claimed; the implementation covers only alternatives JLReq records.

## Implemented workstreams

These headings preserve ownership keys used by the conformance ledger. “Implemented” means
the current tests exercise the workstream; it does not mean stable or released.

## M0 — Classification and specification data

## M1 — Line feasibility and adjustment

## M2 — Mojikumi spacing

## M3 — Whole-paragraph composition

## M4 — Inline constructs and Appendix F

## M5 — Vertical composition

## Independent reference engines

[`engines/ocaml/`](engines/ocaml/README.md) is a from-scratch OCaml implementation of the
conformance protocol, gated on a milestone sequence of its own
(`engines/ocaml/milestones/`) that is unrelated to the Rust workstream numbers above. It
currently claims milestone 0 — the transport, envelope, and specification tables, with no
layout logic yet — and advances toward the full eighty-nine-case built-in suite one
disjoint milestone per pull request. `engines/racket/` will follow the same shape. See
[ADR 0024](docs/adr/0024-independent-reference-engines.md).

## Before any release

- Keep development test-first: reproduce an observable failure, verify Red, implement the
  smallest coherent behavior, verify Green through the Rust API and protocol suite, then
  refactor under the architecture gates.
- Expand mixed-script and vertical reference fixtures, malformed-input fuzz corpora, and
  arithmetic-extreme coverage.
- Profile realistic paragraphs without exposing implementation tuning knobs.
- Treat `docs/api-1.0.toml` as a candidate-surface control only; compatibility remains open
  to change before an explicit release decision.
- Do not remove `publish = false`, create a version tag, or move changelog entries out of
  `Unreleased` as part of ordinary development work.

## Candidate long-term invariants

These are design goals to evaluate before a stable release, not current promises:

- `Style::default()` remains identical to `Style::jlreq_2020()`.
- A future JLReq revision adds a dated profile and specification identifier rather than
  reinterpreting an existing profile.
- The conformance protocol uses a new `jlreq.conformance/N` identifier for incompatible
  envelope or body changes.
- Integer layout results remain bit-identical across supported targets.

## Permanent non-goals

Font I/O, shaping, UAX #14 discovery, bidi resolution, rasterization, and drawing stay
outside the library. These belong to integrations that feed shaped clusters and break
opportunities into `jlreq` and consume its renderer-ready placements.
