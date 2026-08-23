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
just ocaml-build                    # build the engine
just ocaml-test                     # build it, then run the unit tests
just conform-ocaml                  # run all eighty-nine built-in cases against it
just ocaml-gate                     # run the cumulative suite milestones/CURRENT claims
just ocaml-milestone 3              # run the cumulative suite M1..M3
```

The executable lands at a fixed path, which is what the runner is given:

```bash
cargo run -p jlreq-conformance -- \
  run target/dune/default/engines/ocaml/bin/jlreq_ocaml_engine.exe
```

`.exe` is dune's name for a native executable on every platform, Unix included.

**Why `target/dune` and not `_build`.** Dune copies every file a rule depends on
into its build directory, and these rules depend on `spec/captured/table*.tsv`, so
a build under the default `_build/` leaves copies of the transcription at the
repository root. `xtask attest` reports them: it confines the capture to
`spec/captured/` so that a reviewer can read it as one directory, and it skips
only `.git` and `target` while it looks (ADR 0009). The `Justfile` therefore
exports `DUNE_BUILD_DIR` into `target/`, where both stay true at once — and where
`.gitignore` and `cargo clean` already know to treat the contents as build
output. Running `dune build` by hand instead still writes `_build/`, which is
gitignored but *is* scanned, so `just attest`, `just design` and `just ci` will
report those copies until a `dune clean`. Use the recipes.

Nothing else in the Rust gates is affected: `engines/` is outside the Cargo
workspace, and `purity`, `api`, `direction`, `derive`, `generate`, `conform` and
`repository` are all green with the engine present.

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
  lib/              num utf8 tsv tables spec model normalize
                    style construct layout paragraph pipeline  library jlreq
  proto/            json protocol                              library jlreq_proto
  bin/              the NDJSON loop                            jlreq_ocaml_engine.exe
  probe/            development probes, on no gate's path       diffcase.exe census.exe
  test/             dune runtest
  milestones/       M1.ids … M9.ids, and CURRENT
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

## Development probes

`jlreq-conformance run` reports a wrong case as `DIFF <case-id>`. That is the
right report for a gate and the wrong one for the person fixing it, so `probe/`
holds two tools that say more. Neither is the engine, neither is on any gate's
path, and — unlike the engine — both may read a file at run time and start a
subprocess, because nobody hands them a request from an undisclosed working
directory.

```bash
just diffcase quick-start/two-lines          # against the suite's own `expected`
just diffcase quick-start/two-lines --rust   # against the Rust engine's live answer
just census spacing                          # generate, run both engines, diff
just census break
just census-classes                          # the representative chosen for each class
```

**`diffcase`** runs one case and compares the two responses field by field,
naming each difference by its JSON path. It exits `0` on a match, `1` on a
difference and `2` when it could not get an answer at all.

```text
lines[1].clusters[0].inline: expected 0, got 250
DIFF quick-start/two-lines: 1 difference(s) against crates/jlreq-conformance/suite.ndjson
```

**`census`** generates a synthetic suite that isolates one mechanism. It picks
one representative code point per character class out of Appendix A — fewest
classes listing the key first, then a single scalar over a sequence, then an
empty Remarks cell, then document order — and walks the ordered pairs.
`just census-classes` prints the chosen table. Twenty-three of the thirty
classes get one: cl-17 and cl-18 carry no adjacency on any matrix axis, and
cl-20 through cl-23 and cl-30 are what a character *becomes* inside a construct,
so Appendix A lists no code point in them.

| Census | Requests | What one answer is |
| --- | ---: | --- |
| `spacing` | 2,116 | 529 pairs × `pair`, `head`, `end`, `interior`, on a line too wide to break or adjust: Table 1 read back out |
| `break` | 2,116 | 529 pairs × the four §C.3 levels, on a one-cluster line with every boundary `allowed`: Table 2 read back out |
| `reduction` | 3,174 | 529 pairs × Tables 3, 4 and 5, the trailing remainder, the line end and hanging punctuation, on a line exactly as wide as the four ems it holds: §3.8.3's ladder read back out |
| `expansion` | 3,174 | 529 pairs × two measures and the three ceilings, plus `table-5` and one line with the trailing member at half the em, justified with a line after it: §3.8.4's ladder read back out |

The registry that names them is `kinds` in `census.ml`, and nothing else in the
file knows how many there are. A census is a name, a sentence and a function that
emits requests.

Two things the reduction and expansion censuses are deliberately shaped for. Every
`expansion` line carries three interior boundaries, because §3.8.4's stages and
their ceilings are indistinguishable from "hand the whole shortfall to the only
place that will take it" on a line with one — which is the shape all six of the
built-in suite's expansion cases have. And the `mixed-em` variant sets the pair's
trailing member at half the em, because Table 6 names a class pair and no neighbor
(ADR 0021), so an engine has to decide whose em a quarter of an em is a quarter of,
and on a line of one size that decision is invisible.

Both answer streams are canonicalized before `diff` sees them, because key order
is not part of an answer — the Rust side's `serde_json` sorts the keys, this side
writes `lines` before `diagnostics` — and a raw textual diff would call every
line different. Everything a run generates lands in `target/census/`: the
requests, both raw answer streams, both canonical ones, and the diff.

One thing the census surfaced immediately. `spec/derived/questions.tsv` carries
an `excludes` column, and §C.3's `kinsoku.level: very-strict` excludes both
`kinsoku.grouped_numeral_before_western: breakable` and
`kinsoku.relaxation_mechanism: reclassify` — which are exactly what the
`jlreq-2020` profile answers. A request stating nothing but the level is a
contradiction and an engine is right to refuse it. `census.ml` reads the column
and states the forced answers rather than hard-coding those two, so whatever the
M2 and M3 censuses exclude is already handled.

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

### Observable policies with no written source

The exception the rule leaves open is a policy that is *observable* — two engines
have to agree on it to pass the same case — and stated in no sentence of JLReq and
no file under `docs/`. Those are read from `crates/jlreq/src/pipeline.rs`, never
transcribed, and listed here so that a reviewer can see the whole set at once. Each
is a candidate for `docs/decisions/`.

§3.8.4's ladder contributed two, both found by the `expansion` census rather than
by any conformance case:

- **The Japanese–Latin ceiling is asked at cl-19 against cl-27 and nowhere else.**
  §3.8.4 step (b) names three Japanese classes (cl-15, cl-16, cl-19) and three Latin
  ones (cl-24, cl-25, cl-27), which is nine coordinates in each direction, and
  §3.8.4's own Note — the sentence the `rigid` answer comes from — names
  `漢字等（cl-19）など` and the same three Latin classes in Japanese while expanding
  that to all three Japanese classes in English. `adjustment.japanese_latin_expansion_ceiling`
  is consulted at `(19, 27)` and `(27, 19)` alone; at the other sixteen stage-two
  coordinates Table 6's own half em stands whatever the style answers. Neither
  sentence says this, and `spec/derived/questions.tsv` carries no scope column.
- **Step (d) re-levels the second and third stages' boundaries and the residual
  cells, but a Western word space only when Table 6's own cl-26 row makes that
  boundary residual too.** §E.1 says the fourth step adds space "to equalize the
  spacing of 1st, 2nd, 3rd and 4th steps", which reads as all four stages including
  step (a)'s word spaces; the reference engine excludes a first-stage site that is
  not independently residual.

Two more were checked against the reference engine and found to be *readings*
rather than policies, and are recorded here because a later reader will otherwise
re-derive them:

- §E.1 states that the `1/4–1/2` cells "shall not be expanded" when Table 5 is
  adopted as the reduction method. Neither engine implements it, and
  `spec/derived/questions.tsv`'s `excludes` column — the file both engines read for
  exactly this kind of cross-question constraint — carries no such pair. Adding one
  is the concrete change that would make it real for both engines at once.
- `adjustment.expansion_order` selects nothing today. Its `implementation` answer is
  §3.8.4 step (d)'s Note, whose only coordinate is cl-27 against cl-27, which
  `docs/conformance-deferrals.toml` classifies `[[non-observable]]`. Both engines
  answer the same layout for both answers.

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
just ocaml-milestone 3   # select M1..M3 out of the built-in suite and run them
```

The selection is checked: the recipe fails if an identifier in an `.ids` file
names no case in the suite, so a typo or a renamed case cannot shrink the gate
quietly.

`milestones/CURRENT` holds one number — the milestone the engine claims today.
`just ocaml-gate` runs the cumulative suite through it, and the `conform-ocaml`
job in CI runs `just ocaml-gate`, so that one digit is what a merge is held to.
Advance it in the pull request that makes the milestone pass, never before.
At `9` the cumulative suite is the whole suite and the gate is
`just conform-ocaml`.

`just conform-engines`, which `just ci` runs, is the same gate with a check for
the toolchain in front of it: a developer with no opam switch gets a loud
`SKIPPED` line and a green run, because a local gate that fails for a missing
toolchain is a gate that gets routed around. CI has the toolchain and enforces
it.

## Where M3 stands

M1 is the composition core: classification (§3.9.2 and Appendix A), Table 1
spacing, Table 2 breakability with §C.3's four conventions, whole-paragraph break
optimization, and line geometry. M2 is §3.8.3's reduction ladder — Tables 3, 4 and
5, and §3.8.2's hanging punctuation — and M3 is §3.8.4's expansion ladder: Table 6,
the Western word space, the Japanese–Latin ceiling, and step (d)'s residual.
Every one of the eighty-nine requests is parsed completely — a construct this
engine cannot yet *set* is still read, validated and classified — so the milestones
that follow change what the pipeline does with a structure and not whether the wire
layer knows it is there.

```text
just ocaml-milestone 3    → exit 0    (35 cases)
just conform-ocaml        → 45 DIFF lines, exit 1
```

Exit `1` with no protocol error is the contract: the transport, the envelope, the
JSON, the specification tables and the request model are all correct, and only
the layout of the structures M2 onward own is missing. Exit `2` would mean
something in that list is broken.

Where the whole built-in suite stands against `milestones/`:

| M | Subject | Passing |
| --- | --- | --- |
| M1 | classification, spacing, breakability, geometry | 18 / 18 |
| M2 | reduction (Tables 3–5), hanging | 7 / 7 |
| M3 | expansion (Table 6), justification, reclassification | 10 / 10 |
| M4 | vertical composition, rotation, orientation | 5 / 5 |
| M5 | tate-chu-yoko | 0 / 9 |
| M6 | ruby | 0 / 23 |
| M7 | emphasis dots, ornamented complexes | 0 / 4 |
| M8 | warichu, furawake, jidori, formulae | 3 / 10 |
| M9 | tab stops, widows, indentation | 1 / 3 |

M4 falls out of M1's work rather than being claimed: vertical composition is one
orientation rule over the same geometry. `milestones/CURRENT` claims only what the
milestone sequence has reached.

All four censuses agree with the Rust engine at every request:

```text
just census spacing     → 2116 request(s), 0 differing response(s)
just census break       → 2116 request(s), 0 differing response(s)
just census reduction   → 3174 request(s), 0 differing response(s)
just census expansion   → 3174 request(s), 0 differing response(s)
```

That is 529 class pairs read back out of Table 1 in four line positions, out of
Table 2 at all four §C.3 levels, out of Tables 3 through 5 on a line that has to
give the spacing back, and out of Table 6 on a justified line with room left over
— from two independent transcriptions of the same six PDF pages, agreeing bit for
bit. 10,580 requests in all, and no answer differs by one unit.

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
