<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# The OCaml reference engine

An independent implementation of the [conformance
protocol](../../docs/design/conformance.md), written in OCaml. It shares no code
with the Rust engine and is not part of the Cargo workspace.

It exists for two reasons.

- **The protocol claims to be language independent.** A second implementation, in
  a language with a different runtime, a different integer type and a different
  standard library, is the only way to find out.
- **N-version cross-checking.** Appendices B through E are published only as PDF,
  so their roughly 5,400 cells were transcribed twice, once from each locale's
  rendering. The Rust engine reads `spec/captured/table*.en.tsv`. This engine
  reads `spec/captured/table*.ja.tsv`. Two engines agreeing on all eighty-nine
  cases is evidence only when they were not fed the same keystrokes.

## Versions

| Tool | Version |
| --- | --- |
| OCaml | 5.5.0 (the `.opam` file accepts `>= 5.1`) |
| dune | 3.24.2 (the `dune-project` declares `lang dune 3.16`) |
| opam packages | none |

Every computation in the engine is exact integer arithmetic, so the output does
not depend on the OCaml version. The versions are pinned for reproducibility of
the build, not of the answers.

## Building and running

The dune project root is the **repository root**, not this directory: a dune rule
cannot depend on a file above its project root, and this engine reads the
specification tables out of `spec/`. Run everything from the repository root.

```bash
dune build engines/ocaml            # build the engine
dune runtest engines/ocaml          # run the unit tests
dune build                          # both, plus everything else dune owns
```

The executable lands at a fixed path, which is what the runner is given:

```bash
cargo run -p jlreq-conformance -- \
  run _build/default/engines/ocaml/bin/jlreq_ocaml_engine.exe
```

`.exe` is dune's name for a native executable on every platform, Unix included.

> **Known conflict with `just attest`, to be settled when the engines are wired
> into CI.** Dune copies every file a rule depends on into its build directory, so
> a build leaves copies of `spec/captured/table*.tsv` under `_build/default/spec/`.
> The confinement check in `xtask attest` walks the whole working tree, skipping
> only `.git` and `target`, and reports those copies as a transcription outside
> `spec/captured/`. Until `_build` joins `SKIPPED_DIRECTORIES` in
> `xtask/src/attest.rs` (or the build directory is redirected with
> `DUNE_BUILD_DIR`), run `dune clean` before `just attest`, `just design` or
> `just ci`. Nothing else in the Rust gates is affected: `engines/` is outside the
> Cargo workspace, and `purity`, `api`, `direction`, `derive`, `generate`,
> `conform` and `repository` are all green with the engine present.

The engine takes no arguments and reads no files at run time. It reads NDJSON
request envelopes from stdin, writes one response envelope per request to stdout,
and exits `0`. It exits `2` — after printing one message to stderr — when the
specification tables do not pass the startup census, when a line is not JSON, or
when an envelope names a protocol or a specification revision it does not
implement. A *wrong answer* is not an error here: the runner reports it as `DIFF`
and exits `1` itself.

## Layout

```text
engines/ocaml/
  lib/specdata/     the TSV files, embedded at build time      library jlreq_specdata
  lib/              num utf8 tsv tables …                      library jlreq
  proto/            json protocol                              library jlreq_proto
  bin/              the NDJSON loop                            jlreq_ocaml_engine.exe
  test/             dune runtest
  milestones/       M1.ids … M9.ids
```

The three libraries stack in one direction, `jlreq_specdata → jlreq → jlreq_proto`,
and OCaml's ban on cyclic modules is what enforces it — the same job
`xtask direction` does on the Rust side. `jlreq` does not know that JSON exists,
mirroring the Rust `jlreq` crate, which carries no serialization.

`lib/specdata/dune` pastes each TSV file into a generated module as one quoted
string literal, so `cat` is the whole encoder and no generated source is
committed. Embedding rather than reading from disk is forced by the contract: the
runner starts the engine with no arguments, from a working directory it never
discloses, so there is no path for the engine to resolve.

## The independence rule

This is the point of the exercise, so it is worth stating precisely.

**May be read and relied on** — the public contract:

- `crates/jlreq-conformance/protocol.schema.json`, which is the format contract;
- `docs/design/conformance.md`;
- the field names and value vocabularies the sample engine puts on the wire;
- everything under `spec/` — the W3C snapshot, the derived tables and the
  captured matrices — which is the *specification*, not an implementation of it;
- the published legend-token grammar for the six matrices.

**May not be read into this tree** — the Rust implementation:

- the layout logic in `crates/jlreq/src/` (`pipeline.rs` above all);
- `crates/jlreq/src/generated/`, and the xtask code generators that write it.

Where the two engines disagree, the disagreement is settled by returning to JLReq
and to `spec/`, and recorded in `docs/decisions/`. It is never settled by reading
the other engine and copying the answer, and never by majority vote. A rule that
is observable in the Rust engine but written down nowhere is exactly the finding
this second implementation is for.

## Milestones

`milestones/M1.ids` through `M9.ids` partition the eighty-nine built-in cases
into nine disjoint groups; every case is in exactly one file. Each file is one
case identifier per line, in the order `jlreq-conformance list` prints them, with
`#` lines and blank lines as comments.

| M | Subject | Cases | Cumulative |
| --- | --- | ---: | ---: |
| M0 | Num / Utf8 / Tsv / Tables / Json / Protocol, startup census, `validate` green | 0 | 0 |
| M1 | Classification, Table 1 spacing, Table 2 breakability, paragraph optimization, line geometry | 18 | 18 |
| M2 | Reduction: Tables 3, 4, 5, and hanging | 7 | 25 |
| M3 | Expansion: Table 6, justification, reclassification | 10 | 35 |
| M4 | Vertical composition, rotation, orientation | 5 | 40 |
| M5 | Tate-chu-yoko | 9 | 49 |
| M6 | Ruby: mono, group, jukugo, association, overhang | 23 | 72 |
| M7 | Emphasis dots and the ornamented complex | 4 | 76 |
| M8 | Warichu, furawake, jidori, formulae | 10 | 86 |
| M9 | Tab stops, widows, indentation | 3 | 89 |

Because the groups are disjoint, the cumulative suite `M1..Mn` grows
monotonically and `M1..M9` is the whole suite. A milestone is complete when the
cumulative suite through it runs to exit `0`:

```bash
# The cumulative identifier set through Mn.
cat engines/ocaml/milestones/M{1..3}.ids | grep -v '^#' | grep -v '^$' > /tmp/ids
# Select those cases out of the built-in suite into a partial suite, then:
cargo run -p jlreq-conformance -- run ENGINE /tmp/partial.ndjson
```

A `just ocaml-milestone <n>` recipe that does the selection is part of the CI and
tooling integration, not of this milestone.

## Where M0 stands

M0 is the skeleton. The engine answers every request with

```json
{"lines": [], "diagnostics": []}
```

which is well formed, passes `jlreq-conformance validate`, and is the wrong
answer for all but an empty paragraph. So the expected result today is:

```text
cargo run -p jlreq-conformance -- run _build/default/engines/ocaml/bin/jlreq_ocaml_engine.exe
→ 89 DIFF lines, "89 conformance case(s) differed", exit 1
```

Exit `1` with no protocol error is what M0 aims at: the transport, the envelope,
the JSON and the specification tables are all correct, and only the layout is
missing. Exit `2` would mean something in that list is broken.

The startup census in `lib/tables.ml` is checked against the real files and holds
today:

| Table | Measured |
| --- | --- |
| `appendix-a.tsv` | 1,687 rows; 1,686 distinct (class, key) listings; 1,133 distinct keys; 14 distinct (en, ja) Remarks pairs |
| duplicate listing | exactly one, `cl-19 216B`, recorded rather than resolved |
| `folding.tsv` | 226 |
| `ideographs.tsv` | 16 ranges |
| `scripts.tsv` | 22 ranges |
| `classes.tsv` | 30 classes (cl-17 and cl-18 are listed but carry no adjacency) |
| `questions.tsv` | 22 |
| Tables 1, 3, 4, 5 | 841 cells each, on a 29-entry axis that includes the line edge |
| Tables 2, 6 | 784 cells each, on a 28-entry axis with no line edge |
| amounts | every one in `[0, 720]` 1/720-em units |
| axis classes | `0` (line edge), or 1–30 excluding 17 and 18 |

`test/test_tables.ml` additionally builds the **English** transcriptions and
compares all 4,932 cells and their note citations against the Japanese ones the
engine runs on. They agree at every coordinate today. That comparison is the
third independent reading of those six PDF pages, and it turns a transcription
divergence into a named coordinate in a test failure instead of an unexplained
`DIFF` nine milestones from now.
