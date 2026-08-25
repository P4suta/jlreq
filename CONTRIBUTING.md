# Contributing

## Setup

```sh
mise install            # toolchain and gate tooling
mise run hooks          # install the git hooks
mise exec -- just       # list the available commands
```

`mise install` covers everything ordinary Rust development needs. Working on an
independent reference engine under [`engines/`](engines/) additionally needs that engine's
own toolchain — OCaml and `dune` for `engines/ocaml/`, Racket and `raco` for
`engines/racket/` — which `mise.toml` deliberately does not manage
([ADR 0024](docs/adr/0024-independent-reference-engines.md)). The pinned version of each is
recorded in `.github/workflows/ci.yml`, which is the single source of truth for it; without
the toolchain installed, `just conform-engines` (part of `just ci`) prints a `SKIPPED` line
and stays green rather than failing.

## The loop

```sh
mise exec -- just check # fast deterministic gates
mise exec -- just ci    # everything CI runs, locally
```

Gates are run as `mise exec -- just <recipe>`, never as a bare `just`. `mise.toml` pins the
version of every gate tool, and `mise exec` is what puts those pinned versions in front of
whatever else the shell can see. Measured on a developer machine: a bare `just` found no
`zizmor` at all and reached a stray `typos` binary ahead of the pinned one. Either way the
answer is not the answer CI gets, which makes it worthless in both directions.

`mise exec -- just ci` is also the pre-push hook. If it passes locally it passes in CI; if
it fails, fix the cause rather than narrowing the gate.

`just fuzz-check` compiles the three separate nightly harnesses on Windows and runs each for
30 seconds with libFuzzer and sanitizers on Unix. They isolate input validation,
composition/arithmetic, and protocol parsing. Curated inputs live under `fuzz/seeds/`;
runtime corpora live under `target/fuzz-corpus/` and never dirty the source tree.

Full mutation runs cover both handwritten products. Only generated table files and exact
mutants proven equivalent in [docs/mutation-ledger.toml](docs/mutation-ledger.toml) may be
excluded. `just mutation-ledger` binds every such entry to its source SHA-256 and rejects an
undocumented or broad cargo-mutants exclusion.

## Rules that are not negotiable

- **No `allow` and no `ignore`.** Every gate is strict on purpose. Make the code pass
  instead of suppressing the finding. If a lint is genuinely wrong for this codebase,
  change the shared configuration and say why in the commit message.

- **The core stays pure.** `jlreq` must not gain `std`, I/O, font, or
  floating-point dependencies. `mise exec -- just purity` enforces this; see
  [ADR 0001](docs/adr/0001-no-std-no-io-no-font-in-core.md),
  [ADR 0005](docs/adr/0005-integer-layout-units.md), and
  [ADR 0007](docs/adr/0007-two-scalars-and-the-fixed-point-unit.md). The gate reads
  `[dev-dependencies]`, `[build-dependencies]`, and `[target.'cfg(..)'.dependencies]`
  exactly as it reads `[dependencies]`, so a core crate declares no dev-dependency either
  and its unit tests live in `#[cfg(test)]` modules inside the crate.

- **A per-OS difference is a bug, never a tolerance.** Layout arithmetic is integer
  throughout, so `mise exec -- just test-ci` is bit-identical on Linux, Windows, and macOS,
  and the three-OS matrix in CI exists to catch exactly the case where it is not. Find the
  difference. An epsilon is not a fix and there is no tolerance to widen.

- **Specification data is generated wherever JLReq is machine-readable.** Appendix A's 1133
  keys, the legends, every appendix note, the strictness levels, the adjustment ladders,
  and the rule inventory are produced by a generator from the committed HTML snapshot and
  the vendored Unicode Character Database extracts. A hand edit to a generated file is a
  bug even when it is correct, because the next specification revision will not carry it
  forward; `generate --check` catches one by regenerating and comparing bytes. Nothing
  under `crates/*/src/generated/` is ever edited by hand.

- **Where JLReq is not machine-readable, data is transcribed and the transcription is
  attested.** The cells of Tables 1 through 6 exist only as PDF, and their reduction and
  expansion priority ordinals are encoded as cell background color whose key is published
  as a raster image, so roughly 5400 cells have no machine-readable form to generate from.
  They are keyed by hand, and the transcription rather than the PDF is this repository's
  primary source ([ADR 0009](docs/adr/0009-generated-data-and-attested-transcription.md)).
  Five controls are what make that trustworthy, and none of them is optional:
  1. the transcription lives in one directory, `spec/captured/`, and nowhere else;
  2. every matrix is entered twice, independently, from the English and the Japanese
     rendering W3C publishes as separate documents, and the two must agree cell for cell;
  3. every cell records the source file, table, row label, and column label it was read
     from, and a cell without provenance fails the build;
  4. the capture satisfies the cross-table invariants derived from prose that *is*
     machine-readable, each citing the sentence that justifies it;
  5. every amount in every table, note, and ladder is an exact multiple of 1/720 em, which
     is the property that unit was chosen for.

  `attest` runs all five and names them for what they are: double entry is a procedural
  control, the invariants are the mechanical one. Scraping the PDF instead is not an
  option, because the ordinal would still be read by eye from an image and the error would
  present itself as machine-derived. [generation.md](docs/design/generation.md) is the
  pipeline.

- **Every observable rule gets a protocol case.** A mechanically observable rule without a
  request/response case in `crates/jlreq-conformance/suite.ndjson` is incomplete. Editorial
  or non-observable statements belong in
  [docs/conformance-deferrals.toml](docs/conformance-deferrals.toml) with primary evidence;
  an empty case is not coverage. `just conform` checks the inventory in both directions.

## Code and comments are in English

The repository — including comments and documentation — is written in English so that the
spell checker works and so that adopters outside Japan can read it. Japanese terms of art
(kinsoku, mojikumi, oikomi) are used as loanwords with the kanji in parentheses on first
use, because they have no accurate English equivalents. "First use" means first use *per
module*, because rustdoc renders modules in an order the author does not control, so a gloss
that lands after the unglossed uses is a gloss nobody reads. A term introduced by its
English translation instead carries the kanji and the romanization — "hanging punctuation
(ぶら下げ, burasage)" — and a term introduced as a loanword carries the kanji alone —
"warichu (割注)".

Use `ADR-0013` inside source comments and a Markdown link such as `docs/adr/0013` in prose.

The workspace is prepared at `0.1.0` and both product manifests are publishable. Release
automation remains externally inert: ordinary development must not publish a crate, create
a tag or GitHub Release, configure Trusted Publishing, or change repository settings.
`just release-check` performs the full non-publishing acceptance suite on a clean candidate.

Run `just semver` for public API changes. At 0.1.0 it verifies the network-free release
contract in `docs/public-api.toml`; after the initial publication it additionally compares
each 0.1.x candidate with the latest published jlreq release in patch-compatibility mode.
Changing `baseline_version` or `compatible_series` is a release-policy change, not a way to
waive an individual finding.

Tracked UTF-8 files use LF, including on Windows, and local links in tracked Markdown are
part of the repository contract. Keep links relative so they work in a checkout and run
`just repository`; the gate holds the release-ready state and rejects CR bytes, missing
targets, and links that escape the repository while leaving binary files, external URLs,
and in-page anchors alone.
Use canonical JLReq addresses from `spec/derived/rules.tsv` in protocol case metadata.

## Commits

Conventional Commits, validated by `committed` in the commit-msg hook:

```text
feat(class): add cl-01 through cl-05 determination
fix(line): stop hanging a closing bracket past the line end
docs(adr): record the integer-unit decision
```

## Where discussion belongs

Disagreements about what JLReq requires belong in the protocol-v1 suite as a black-box case
with the section reference, not only in an issue thread. Where JLReq permits alternatives,
the answer is a typed caller-visible option, not a default chosen in code review.
