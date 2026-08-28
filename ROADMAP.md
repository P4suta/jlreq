# Roadmap

## Current status: 0.1.0 prepared, not published

The three product packages are implemented:

- `jlreq`: high-level text/font-to-glyph layout on MSRV 1.88;
- `jlreq-core`: dependency-free `no_std + alloc` composition on MSRV 1.85; and
- `jlreq-conformance`: protocol-v1 black-box validation.

The code, documentation, CI, package isolation, release workflows, API snapshot, checksum,
SBOM, and attestation paths are prepared. Publication date, credentials, uploads, tag, and
GitHub Release remain deliberately unperformed.

The specification identifier remains `jlreq-2020-08-11+unicode-17.0.0`. The current
conformance inventory remains 100/106; six editorial or non-observable entries are
explicitly ledgered. Complete JIS X 4051 conformance is not claimed beyond the alternatives
JLReq records.

## Implemented workstreams

These headings preserve ownership keys used by the conformance ledger. “Implemented” means
the current tests exercise the workstream; it does not imply publication.

## M0 — Classification and specification data

## M1 — Line feasibility and adjustment

## M2 — Mojikumi spacing

## M3 — Whole-paragraph composition

## M4 — Inline constructs and Appendix F

## M5 — Vertical composition

## High-level text integration

The facade preserves source ranges while splitting paragraphs, itemizes graphemes/scripts,
performs grapheme-wide fallback, resolves UAX #9, shapes font/script/direction runs,
combines UAX #14 and authored break controls, calls the core, and emits per-line visual
order in physical coordinates. Explicit font bytes are the reproducibility boundary.

## Independent reference engines

The OCaml and Racket engines independently implement the complete protocol surface from
`spec/` and public conformance documents. Their generated ten-kind census covers 122,199
requests and records zero differences among the three implementations. See
[the generated summary](docs/generated/conformance-summary.md) and
[ADR 0024](docs/adr/0024-independent-reference-engines.md).

## Publication-only work remaining

- Select an actual date and run `just finalize-release YYYY-MM-DD`.
- Have a repository administrator verify the recorded release-environment, reviewer,
  branch, required-check, and full-SHA Action settings.
- Run the successful manual `Release check` workflow for the finalized candidate and
  verify its three crate archives plus six binary archives, checksums, SBOMs, and
  attestations.
- Supply a narrowly scoped token only for the first publication, upload `jlreq-core`,
  wait for registry visibility, then upload `jlreq` and `jlreq-conformance`.
- Only after all uploads succeed, create `v0.1.0` and the GitHub Release; configure
  Trusted Publishing and revoke the initial token.

## Release-line invariants

- `Style::default()` remains identical to `Style::jlreq_2020()`.
- Explicit identical font bytes, face indices, text, and options produce bit-identical
  quantized results on supported systems; system-font selection is not covered.
- The core remains dependency-free `no_std + alloc`; facade dependency types remain
  private.
- The conformance protocol uses a new `jlreq.conformance/N` identifier for an incompatible
  envelope or body.
- Resource refusal is atomic and never returns a partial layout.

## Permanent non-goals

- rasterization, canvas/PDF/GPU drawing, and renderer ownership;
- a CLI or JSON layout service beyond the conformance validator;
- a WASM-specific facade, website, or hosted service; and
- semantic guessing of units, quantities, formulae, or annotations that an author can mark
  explicitly with `DocumentBuilder`.
