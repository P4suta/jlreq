# ADR-0025: separate the high-level facade from the composition core

- Status: accepted
- Date: 2026-08-28
- Supersedes the product topology in
  [ADR 0022](0022-unified-public-crate-and-process-conformance.md).
- Retains the project, binary, and protocol names chosen by
  [ADR 0023](0023-the-project-is-named-jlreq.md).

## Context

The single dependency-free `jlreq` library accepted only pre-shaped clusters and authored
break opportunities. That was an appropriate engine boundary, but it made the shortest
application path require font parsing, fallback, shaping, Unicode line segmentation, bidi,
cluster attribution, and renderer mapping before Japanese composition could begin.

Those responsibilities cannot enter the dependency-free core without removing its useful
`no_std + alloc` boundary. Conformance also serves language-independent implementation
authors and should not acquire a facade dependency.

Version 0.1.0 has not been published, so crate names and public entry points can still be
chosen for users rather than preserved for compatibility.

## Decision

The product has three layers:

1. `jlreq` is the MSRV 1.88 high-level facade. It accepts UTF-8 text and explicit font
   bytes, performs grapheme fallback, shaping, UAX #9, UAX #14, and physical placement, and
   returns a self-contained draw-ready `TextLayout`.
2. The former low-level library becomes `jlreq-core`, preserving dependency-free
   `no_std + alloc` composition and MSRV 1.85. It is also available as `jlreq::core`,
   without flattening all low-level names into the facade root.
3. `jlreq-conformance` remains binary-only and retains
   `jlreq.conformance/1` and the existing specification identifier.

The facade uses Fontique for opt-in system discovery, HarfRust for shaping, ICU4X for line
and grapheme segmentation, and unicode-bidi for UAX #9. Upstream types stay private.
Explicit font bytes are the reproducibility boundary; OS font choice is not.

Rendering, rasterization, GPU backends, a layout CLI/service, a WASM-specific facade, and a
website are outside the decision.

## Consequences

Ordinary callers can reach glyph placements from text and fonts without constructing
low-level clusters or breaks. Engine implementers retain the small deterministic core, and
protocol implementers retain a process boundary independent of Rust types.

The package and release order is `jlreq-core`, registry visibility, then `jlreq` and
`jlreq-conformance`. API, semver, package, mutation, coverage, fuzz, license, SBOM, and
attestation gates cover all applicable layers. Publication itself remains a separate
explicit maintainer action.
