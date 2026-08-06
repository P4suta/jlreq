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
- The vendored specification: the W3C published rendering of JLReq at
  `spec/snapshot/index.html`, the three Unicode Character Database extracts it is read
  against, and `spec/PROVENANCE.toml` recording where each was retrieved and its SHA-256.
  `just attest` verifies the files on disk against those digests, so every table below
  names the bytes it was read from rather than a URL that may since have moved.
- Stage 1 of the specification-data pipeline, `just derive`: a `std`-only scanner that
  reads the bilingual snapshot into eight tab-separated files under `spec/derived/` —
  Appendix A, the class list, the ideograph predicate, the compatibility folding, the two
  kana scripts, the document skeleton, the rule inventory and the appendix notes. Each
  derived file states the digest of every source it was read from *and* of the modules that
  read it, because a semantic column is the reader's reading of the document rather than a
  column of it. `just derive-check` fails when rereading the snapshot would change a byte.
- The rule inventory ADR 0013 addresses: 106 rules generated into `jlreq-spec`, numbered
  from the document's own rendered numbering and never from an anchor slug, which is off by
  one for the appendix legends. `spec-links`, `direction` and `conform` now close over that
  data instead of reporting that they had none to close over.
- `jlreq-class`, complete for M0: Appendix A's 1133 keys as 1686 listings, 473 of them named
  by more than one class — the measurement ADR 0008 turns on, since it is why no total
  function from a code point to a class exists to write. Classification takes an occurrence;
  a key is an ordered code-point sequence matched longest-first, because 25 of Appendix A's
  rows key on a pair; and `Text::new` refuses a stream this crate could not answer for
  rather than guessing at it. `Text`, `classify`, `resolve`, `members`, `usage` and the
  thirty class names of §3.9.2 are implemented to `docs/design/api-spine.md`.
- `crates/jlreq-conform/cases.schema.json`, the conformance case format contract. The suite
  is written milestone by milestone; the format it validates against is fixed now, so the
  cases and the implementation can be authored independently.
- `docs/decisions/`, with the first three readings this project publishes where JLReq is
  silent: an unlisted code point, an ambiguous context, and the compatibility ideographs.
  Each carries a standing other than `Normative`, so an answer resting on one says so.

### Changed

- The layout core is seven crates rather than five. `just purity` now checks the crate
  graph as adjacency rather than as membership, so a permitted core crate reaching another
  core crate it has no row for is a failure.
- Documents corrected against the frozen design: a character class is a property of an
  occurrence rather than of a code point, a spacing amount is not a function of the two
  adjacent classes alone, and ruby overhang is placed after line adjustment rather than
  resolved before it. ADR 0001 and ADR 0005 carry superseded-in-part notes.
- Stage 1 of the generation pipeline lives in `xtask` rather than in `tools/jlreq-gen`, a
  workspace excluded from the root. The scanner reads the snapshot with `std` alone, so
  there is no dependency tree to keep out — and everything outside the workspace escapes
  Clippy, `rustfmt`, `cargo-msrv` and, decisively, `cargo nextest`.
  `docs/design/generation.md` records the change and the reasoning.
- The CI design job runs `just design` rather than the gates enumerated by hand. The two had
  already drifted: `derive-check`, the only gate binding `spec/derived/` to the vendored
  document, was in the aggregate and not in the list.
- `conform` treats an absent case directory as an operand that does not exist rather than as
  an empty one, so declared coverage is reported as a check that did not run — naming how
  many rules it would have closed over — instead of failing on a schedule. Creating the
  directory turns it on, empty or not.
- The `typos` pre-commit hook passes `--force-exclude`. Without it `typos` ignores the
  exclusions in `typos.toml` for paths named on the command line, and `{staged_files}` names
  every path, so `--write-changes` would have "corrected" the vendored specification and
  broken the digests that prove it is upstream's.
- Twelve recorded upstream defects rather than ten: the cl-24 Remarks role stated only in
  Japanese, and §3.1.6's second Note, whose English leaves a cross-reference as the literal
  placeholder the Japanese resolves to §B.
