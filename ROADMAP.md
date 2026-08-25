# Roadmap

## Current status: 0.1.0 prepared, not published

Both product packages and their release artifacts are ready for 0.1.0. The public Rust API,
stable error codes, protocol v1, integer placement, MSRV 1.85, and `no_std + alloc` boundary
are the 0.1.x compatibility line. Preparation deliberately does not upload crates, create a
tag or GitHub Release, configure a Trusted Publisher, or change branch protection.

The release has a deliberately small product boundary:

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
advanced toward the built-in suite one disjoint milestone per pull request and now claims
milestone 9, the last one: all eighty-nine cases answer bit for bit, so `just ocaml-gate`
and `just conform-ocaml` are the same run and the required CI job holds the engine to the
whole suite. Ten synthetic censuses agree with the Rust engine across 122,199 further
requests, and the twenty-six observable policies the exercise turned up — rules two engines
must share to pass the same case, stated in no sentence of JLReq and no file under `docs/` —
are listed in `engines/ocaml/README.md` and are candidates for `docs/decisions/`.
[`engines/racket/`](engines/racket/README.md) independently implements the same complete
protocol surface. The generated census summary, rather than prose copied by hand, records
all ten census counts and all three pairwise zero-difference results. See
[the summary](docs/generated/conformance-summary.md) and
[ADR 0024](docs/adr/0024-independent-reference-engines.md).

## Publication-only work remaining

- Choose the release date and move the completed `Unreleased` notes to `0.1.0`.
- With explicit maintainer approval, upload `jlreq`, wait for the crates.io index, then
  upload `jlreq-conformance`.
- Configure crates.io Trusted Publishing after that required first manual publication.
- Create the `v0.1.0` tag and GitHub Release from the already verified artifacts.
- Apply any desired external branch-protection settings separately.

## Release-line invariants

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
