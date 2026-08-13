# Roadmap to 1.0

The public architecture is in place, but publication remains disabled. Work proceeds
test-first: add an observable failing case, verify Red, implement through the unified
pipeline, verify Green against both the new API and retained regression assets, then remove
the migrated legacy surface.

## Completed foundation

- dependency-free `no_std + alloc` `kumihan` candidate with no feature flags;
- UTF-8/cluster validation and Appendix A pair normalization;
- typed sizes, frames, roles, writing modes, paragraph builder, and infallible composition;
- one whole-paragraph optimizer with mandatory/discretionary breaks, widow control, and
  integrated tabs;
- opaque constructors for all nine inline structures and renderer-ready horizontal/vertical
  placements;
- all 22 Style choices as dedicated enums plus five dated profiles;
- binary-only protocol-v1 runner, JSON Schema, built-in case, and sample engine;
- compile- and run-tested ICU4X byte-offset and HarfRust cluster adapters, kept as
  conformance-product dev dependencies and verified on Rust 1.85;
- exact public-name allowlist and typed Style/specification mapping;
- all former crates marked `publish = false` and retained as migration assets.

The deferral ledger retains the historical milestone identifiers below as stable schedule
keys while their behavior is moved behind the unified API.

## M0 — Classification and specification data

Completed in the retained implementation. Migrate the generated Appendix A data,
occurrence-sensitive classification behavior, provenance, and cases into `kumihan` without
making classes or rule identifiers public.

## M1 — Line feasibility and adjustment

Migrate kinsoku boundaries, reduction/expansion ladders, indentation, tabs, widow handling,
and Appendices C through E into the single paragraph optimizer. Every migrated rule gets a
public or protocol-level regression before its legacy path is removed.

## M2 — Mojikumi spacing

Migrate the generated spacing matrices and Appendix B choices into private normalization
and spacing stages. Finish exact spacing/classification parity for mixed frames, roles, and
sizes through black-box expectations.

## M3 — Whole-paragraph composition

Extend the existing dynamic program to all construct-aware contributions and physical line
geometry, preserving checked integer behavior and deterministic results across targets.

## M4 — Inline constructs and Appendix F

Complete ruby overhang fixpoints, long group ruby, phonetic jukugo, Appendix F, and the
remaining eight structures, including construct-aware breaking, expansion, placement, and
warichu straddling in both writing modes.

## M5 — Vertical composition

Complete the deferred vertical-only classification, spacing, transform, and physical
placement rules, including local direction changes such as tate-chu-yoko.

## Remaining conformance work

The legacy inventory currently reports 22 deferred rules and five evidence-backed
editorial/non-observable classifications. Highest-priority implementation groups are:

1. complete spacing/classification parity with generated Appendix tables in the unified
   pipeline;
2. ruby overhang fixpoint, long group ruby, phonetic jukugo, and Appendix F;
3. full lowering, line interaction, and physical placement for warichu, furawake, jidori,
   reference marks, scripts, formulae, and tate-chu-yoko;
4. construct-aware expansion/reduction and warichu straddling in paragraph search;
5. remaining vertical-writing-specific rules and exact physical transforms;
6. translate all applicable legacy cases to protocol-v1 black-box requests and responses;
7. classify editorial and non-observable JLReq statements with evidence.

Each group begins with a failing public or protocol test. The retained old implementation
is an oracle only for behavior it already covers; specification-derived expected values
remain authoritative.

## Integration and portability

- extend the checked ICU4X and HarfRust adapters with more mixed-script and vertical cases
  without adding either dependency or feature to `kumihan`;
- require `thumbv7em-none-eabi`, `wasm32-unknown-unknown`, docs, and the one-screen doctest;
- add property/fuzz coverage for invalid UTF-8 boundaries, duplicate/crossing ranges,
  Appendix-key splits, bad frames, overflow, and construct-internal breaks;
- keep integer results bit-identical across targets.

## Migration cleanup

Only after differential and protocol coverage is complete:

- remove the eight old crates from the workspace and repository;
- remove the legacy facade and `docs/api-frozen.toml` controls tied to it;
- reduce the workspace product graph to `kumihan`, `kumihan-conformance`, and `xtask`;
- generate the final public allowlist/baseline and enable post-1.0 semantic-version checks.

## Release gates

1. zero mechanically implementable deferrals;
2. the public quick start fits on one screen;
3. users never connect normalize/lower/feasible/place stages manually;
4. no old-crate type appears in any public signature;
5. an external process runs the complete suite using only protocol-v1 JSON.

Until all five hold, every product manifest remains `publish = false` and the repository
does not claim 1.0 conformance.

## Permanent non-goals

Font I/O, shaping, UAX #14 discovery, bidi resolution, rasterization, and drawing stay
outside the library. The target is JLReq 2020-08-11 plus Unicode 17.0.0. Alternatives JLReq
records from JIS are supported as choices; complete JIS X 4051 conformance is not claimed.
