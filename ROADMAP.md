# Roadmap

## 1.0 baseline

The 1.0 release criteria are complete:

- the public Rust surface is the dependency-free `no_std + alloc` `kumihan` crate;
- callers provide shaped UTF-8 clusters and break opportunities, then make one `compose`
  call over a validated `Paragraph`;
- all nine inline constructs, horizontal and vertical placement, tabs, widow control,
  mandatory/discretionary breaks, and whole-paragraph optimization share that pipeline;
- every one of the 22 JLReq 2020 choice points is a dedicated typed Style setting;
- all 100 observable inventoried rules have protocol-v1 black-box cases;
- the remaining three editorial and three non-observable statements carry primary evidence;
- ICU4X byte offsets and HarfRust glyph clusters are exercised by reference integrations;
- the eight pre-1.0 crates and their compatibility controls have been removed.

The stable specification identifier is
`jlreq-2020-08-11+unicode-17.0.0`. Complete JIS X 4051 conformance is not claimed; the
library implements only the alternatives that JLReq records.

The following headings preserve the stable ownership keys used by the conformance ledger;
all are complete in the unified pipeline.

## M0 — Classification and specification data (complete)

## M1 — Line feasibility and adjustment (complete)

## M2 — Mojikumi spacing (complete)

## M3 — Whole-paragraph composition (complete)

## M4 — Inline constructs and Appendix F (complete)

## M5 — Vertical composition (complete)

## Post-1.0 compatibility

- `Style::default()` remains identical to `Style::jlreq_2020()` forever.
- A future JLReq revision adds a new dated profile and specification identifier; it does
  not reinterpret an existing profile.
- Public additions and removals require an intentional update to `docs/api-1.0.toml` and a
  semantic-version review.
- The conformance protocol is versioned independently. An incompatible envelope or body
  change requires a new `kumihan.conformance/N` identifier.
- Integer layout results remain bit-identical across supported targets.

## Ongoing work

Development remains test-first: reproduce an observable failure, verify Red, implement the
smallest coherent behavior, verify Green through the Rust API and protocol suite, then
refactor under the architecture gates.

Useful post-1.0 improvements include broader mixed-script and vertical reference fixtures,
more fuzz corpus seeds for malformed ranges and arithmetic extremes, performance profiling
without public tuning knobs, and conformance cases for clarified future JLReq revisions.

## Permanent non-goals

Font I/O, shaping, UAX #14 discovery, bidi resolution, rasterization, and drawing stay
outside the library. These belong to integrations that feed shaped clusters and break
opportunities into `kumihan` and consume its renderer-ready placements.
