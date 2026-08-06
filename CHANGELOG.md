# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Workspace bootstrap: crate skeletons, quality gates, and the day-one architectural
  decision records. No layout logic yet.
- Fourteen further decision records (0007 through 0020) and the three design notes they
  were argued from: the API spine, the specification-data generation pipeline, and the
  conformance suite format.
- `jlreq-unit`, the quantity and item vocabulary every later layer speaks through. Two
  kinds of length that never mix — a stated fraction of the ideographic em (全角, zenkaku)
  in a 1/720 fixed-point unit, and a caller-supplied advance — inline and block axes with
  no conversion between them, and the item, run, and seam types. No `core::ops` trait is
  implemented for any of them, so a bare `+` on a length is a compile error rather than a
  lint finding.
- `jlreq-spec`, the specification-reference vocabulary: the address grammar JLReq's own
  numbering is written in, the provenance an answer carries, and a policy space that
  refuses a self-contradictory policy at construction rather than at every entry point.
- Eight design gates beside `purity` — `ops`, `placeholder`, `api`, `spec-links`,
  `direction`, `generate-check`, `attest`, and `conform` — run together as `just design`
  in the loop and as one CI job. Each reports which of its checks had no data to run over
  instead of reporting a pass, so a gate awaiting the generated tables never states that a
  check it could not run held.
- The control files those gates read — `docs/api-frozen.toml`, `docs/direction-sites.toml`
  and `docs/scalar-sites.toml` — each guarded by `CODEOWNERS`, which is what makes them
  controls rather than documentation.

### Changed

- The layout core is seven crates rather than five. `just purity` now checks the crate
  graph as adjacency rather than as membership, so a permitted core crate reaching another
  core crate it has no row for is a failure.
- Documents corrected against the frozen design: a character class is a property of an
  occurrence rather than of a code point, a spacing amount is not a function of the two
  adjacent classes alone, and ruby overhang is placed after line adjustment rather than
  resolved before it. ADR 0001 and ADR 0005 carry superseded-in-part notes.
