<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# The Racket reference engine

An independent implementation of the [conformance
protocol](../../docs/design/conformance.md), written in Racket. It shares no code
with the Rust engine or with the [OCaml one](../ocaml/README.md), and is not part
of the Cargo workspace.

It exists for the two reasons `docs/design/conformance.md` gives, and for one more
that only a *third* engine can supply.

- **The protocol claims to be language independent.** OCaml tested that against a
  language with a different runtime and a different native integer. Racket tests it
  against one with no native integer at all: every number here is an exact rational
  by default and unbounded by default, so every bound Rust gets from its types this
  engine has to put there by hand. Where OCaml's risk was a 63-bit `int` stopping
  one bit *early*, Racket's is arithmetic that never stops at all.
- **N-version cross-checking.** Three implementations answering all eighty-nine
  built-in cases identically is evidence the answer is right, not evidence that one
  implementation's mistake was copied into another.
- **A second reader of the same transcription.** See *Locale* below.

## Versions

| Tool | Version |
| --- | --- |
| Racket | 9.3 (CS), `distribution: minimal` |
| collections | `base` at run time; `compiler-lib` and `rackunit-lib` to build and test |

The version of record is the `RACKET_VERSION` env of the `conform-racket` job in
[`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) and this table. Racket
is deliberately outside `mise.toml`: the asdf-style plugins build from source,
which is slow and reproduces poorly, and pinning a full version number in the
workflow is the same thing `ocaml-compiler: "5.5.0"` does one job above.

Every computation in the engine is exact integer arithmetic, so the output does not
depend on the Racket version. The version is pinned for reproducibility of the
build, not of the answers.

### Installing it locally

There is no `mise` task and no PATH manipulation in the `Justfile`: the recipes call
`racket` and `raco` and leave finding them to the caller, exactly as the OCaml
recipes call `dune`. A user-level install of the same version CI uses:

```bash
curl -LO https://download.racket-lang.org/installers/9.3/racket-minimal-9.3-x86_64-linux-buster-cs.sh
sh racket-minimal-9.3-x86_64-linux-buster-cs.sh --in-place --dest "$HOME/.local/racket"
export PATH="$HOME/.local/racket/bin:$PATH"      # in your shell profile
raco pkg install --skip-installed --batch --auto --no-docs compiler-lib rackunit-lib
racket --version                                  # Welcome to Racket v9.3 [cs].
```

`compiler-lib` is not optional and is not a convenience: minimal Racket ships `raco
pkg`, `raco setup` and `raco link` and nothing else, so `raco make`, `raco exe` and
`raco test` — every `raco` command the recipes use — arrive with it. The full
distribution has them already and works without this line.

## Building and running

Everything is run from the repository root.

```bash
just build-engine-racket            # raco make, then raco exe
just test-engine-racket             # run the unit tests
just conform-racket                 # run all eighty-nine built-in cases against it
just racket-gate                    # run the cumulative suite milestones/CURRENT claims
just racket-milestone 3             # run the cumulative suite M1..M3
just census-racket spacing          # one synthetic census, both engines, diffed
```

The executable lands at a fixed path, which is what the runner is given:

```bash
cargo run -p jlreq-conformance -- run engines/racket/bin/jlreq-engine-racket
```

The engine takes no arguments and reads no files at run time. It reads NDJSON
request envelopes from stdin, writes one response envelope per request to stdout,
and exits `0`. It exits `2` — after printing one message to stderr — when the
specification tables do not pass the startup census, when a line is not JSON, or
when an envelope names a protocol or a specification revision it does not
implement. A *wrong answer* is not an error here: the runner reports it as `DIFF`
and exits `1` itself.

`engines/racket/bin/` and every `compiled/` directory `raco make` writes are
gitignored. Nothing generated is committed.

The loop is streaming — read one line, answer it, read the next — which is what the
OCaml engine does and what the transport headroom allows. The runner writes *every*
request before it reads *any* answer, so the arrangement is deadlock-free only while
the whole request stream fits in one pipe buffer: it is 53,518 bytes today against a
64 KiB Linux pipe, and the answer stream is 106,472. An engine that blocks writing
its answers is harmless, because the runner has by then finished writing and gone on
to read; a request stream that outgrew the buffer would not be, for either engine.
Worth knowing before the suite grows by half.

## Layout

```text
engines/racket/
  embed.rkt       the compile-time file paste
  specdata.rkt    the TSV files, embedded at build time
  arith.rkt       the integer contract
  tsv.rkt         the reader for spec/derived/ and spec/captured/
  tables.rkt      the six matrices, Appendix A, and the startup census
  model.rkt       the request, read and closed
  style.rkt       the twenty-two questions, resolved
  classes.rkt     §3.9.2: the class of one occurrence
  spacing.rkt     Appendix B: the amount at one boundary
  kinsoku.rkt     Appendix C: whether a line may break there
  adjust.rkt      Appendices D and E: the floor, the ceiling, the stage
  ruby.rkt        §3.3 and Appendix F: where a reading goes
  ornament.rkt    §3.3.9 and §3.7.1: the ornamented complex
  structure.rkt   §3.4, §3.7.2 and §3.7.3: the structures that set text elsewhere
  compose.rkt     §3.8: the paragraph, as lines
  protocol.rkt    the envelope, in and out
  main.rkt        the NDJSON loop                      → bin/jlreq-engine-racket
  info.rkt        the collections this directory needs
  tests/          raco test
  milestones/     M1.ids … M9.ids, and CURRENT
```

The modules stack in one direction — `embed → specdata → tsv → tables → {model,
style} → classes → spacing → {kinsoku, adjust, ruby, ornament, structure} → compose
→ protocol → main` — and Racket's ban on cyclic `require`s is what enforces it, the
same job `xtask direction` does on the Rust side. Nothing below `protocol.rkt` knows
that JSON exists, mirroring the Rust `jlreq` crate, which carries no serialization.

### How the specification tables get into the executable

This is the one build decision worth writing down, because Racket offers two
answers and only one of them satisfies the runner contract.

`docs/design/conformance.md` states that contract: the runner starts an engine with
`Command::new(engine)`, no arguments, from a working directory it never discloses,
so **every reference table has to be inside the executable**.

- **`define-runtime-path`** records a path and leaves the file where it is. `raco
  exe` alone copies nothing; only a subsequent `raco distribute` gathers
  runtime-path files next to the binary, and without that step the executable
  resolves the absolute path it was built at. That works on the machine that built
  it and stops being true the moment the checkout moves — which is precisely the
  run-time file resolution the contract rules out.
- **A compile-time paste** puts the bytes in the compiled code. `embed.rkt`'s
  `embed-file` macro reads the file during expansion and expands to its contents as
  a string literal; `raco make` writes that into the `.zo`, `raco exe` embeds the
  `.zo`, and the executable has no file to find and no distribution step to forget.

This engine takes the second, which is also the shape `engines/ocaml/lib/specdata/dune`
takes: build-time paste, run-time parse, nothing generated into the source tree and
nothing committed. The cost is that Racket's compilation manager cannot see a
dependency a macro opened by hand, so `embed-file` calls `register-external-file`
from `compiler/cm-accomplice` to record each table in the module's `.dep` — which is
what makes `just build-engine-racket` correct after a `just derive` rather than
quietly stale.

The check that this actually worked is `tables.rkt`'s startup census: a truncated
paste would still compile, and the census is what refuses to answer on one.

### Locale

The Rust engine reads `spec/captured/table*.en.tsv`. The OCaml engine reads
`table*.ja.tsv`. **This engine reads the English ones**, and its own tests build the
Japanese ones and compare all 4,932 cells and their note citations against them.

`docs/design/conformance.md` describes the two-engine case, where the second engine
is by definition the one that reads the other locale. With three engines the locales
cannot all differ, and what has to stay true is the property that sentence is
protecting: **both hand transcriptions of those six PDF pages are read by something,
and the agreement between them is checked rather than assumed.** OCaml answering all
eighty-nine cases from the Japanese side is what keeps the first half true; the
cell-for-cell comparison in `tests/test-tables.rkt` — the mirror image of the one
`engines/ocaml/test/test_tables.ml` makes — is what keeps the second, and turns a
transcription divergence into a named coordinate in a test failure instead of an
unexplained `DIFF` nine milestones from now.

The two locales' files are keyed in different row orders, so that comparison is by
coordinate and never by row.

## The independence rule

The rule is stated in `docs/design/conformance.md` and is not restated here. What is
worth recording is what it means for *this* engine, which is being written after the
OCaml one is already finished and green on all eighty-nine cases.

**May be read and relied on** — the public contract, plus the OCaml engine's
*shape*:

- everything `docs/design/conformance.md` lists: `protocol.schema.json`, that
  document, the vocabularies the sample engine puts on the wire, everything under
  `spec/`, and the published legend-token grammar for the six matrices;
- `docs/decisions/`, which is where a disagreement between two engines is settled
  and recorded;
- `engines/ocaml/README.md`, and in particular its "Observable policies with no
  written source" and "Where the two engines disagree" — those are findings about
  JLReq and about the Rust engine, published so that the next implementation does
  not have to rediscover them one `DIFF` at a time;
- `engines/ocaml`'s module split, its TSV reader specification, its arithmetic
  contract and its test structure. A layout is not encoded in a directory listing.
- `engines/ocaml/probe/` — `census` and `diffcase` are shared development tools, not
  implementations of JLReq, and `just census-racket` reuses `census` rather than
  growing a second copy.

**May not be read into this tree:**

- the layout logic under `crates/jlreq/src/` (`pipeline.rs` above all), and
  `crates/jlreq/src/generated/`;
- **the bodies of `engines/ocaml/lib/`**, `pipeline.ml` and `spec.ml` above all. A
  line-by-line translation of a finished engine is not an independent
  implementation: it would agree with the OCaml engine exactly where the OCaml
  engine is wrong, and the whole value of a third engine is that it disagrees for
  reasons of its own. Each milestone here is written from JLReq and from `spec/`,
  and the OCaml answer is consulted only after this engine has produced one.

Where two engines disagree, the disagreement is settled by returning to JLReq and to
`spec/`, and recorded in `docs/decisions/`. Never by reading the other engine and
copying the answer, and never by majority vote among the three.

Two coordinates are already known to be places the Rust engine is not self-consistent
— both about a tab sign standing inside a structure that does not set its text along
the line — and `engines/ocaml/README.md`'s "Where the two engines disagree" records
the reading the OCaml engine took. This engine takes the same reading, because it is
the one the specification supports and not because the OCaml engine took it.

## Milestones

`milestones/M1.ids` through `M9.ids` partition the eighty-nine built-in cases into
nine disjoint groups; every case is in exactly one file. They are the **same
partition** `engines/ocaml/milestones/` uses, which is deliberate: the split is a
statement about which cases depend on which mechanisms of JLReq, so two engines
walking the same order can be compared milestone by milestone, and a case that is
hard here and easy there is a finding rather than a coincidence.

| M | Subject | Cases | Cumulative |
| --- | --- | ---: | ---: |
| M0 | Arith / Tsv / Tables / Protocol, startup census, transport green | 0 | 0 |
| M1 | Classification, Table 1 spacing, Table 2 breakability, paragraph optimization, line geometry | 18 | 18 |
| M2 | Reduction: Tables 3, 4, 5, and hanging | 7 | 25 |
| M3 | Expansion: Table 6, justification, reclassification | 10 | 35 |
| M4 | Vertical composition, rotation, orientation | 5 | 40 |
| M5 | Tate-chu-yoko | 9 | 49 |
| M6 | Ruby: mono, group, jukugo, association, overhang | 23 | 72 |
| M7 | Emphasis dots and the ornamented complex | 4 | 76 |
| M8 | Warichu, furawake, jidori, formulae | 10 | 86 |
| M9 | Tab stops, widows, indentation | 3 | 89 |

Because the groups are disjoint, the cumulative suite `M1..Mn` grows monotonically
and `M1..M9` is the whole suite. A milestone is complete when the cumulative suite
through it runs to exit `0`:

```bash
just racket-milestone 5   # select M1..M5 out of the built-in suite and run them
```

The selection is checked: the recipe fails if an identifier in an `.ids` file names
no case in the suite, so a typo or a renamed case cannot shrink the gate quietly.

`milestones/CURRENT` holds one number — the milestone the engine claims today.
`just racket-gate` runs the cumulative suite through it, and the `conform-racket`
job in CI runs `just racket-gate`, so that one digit is what a merge is held to.
Advance it in the pull request that makes the milestone pass, never before. At `9`
the cumulative suite is the whole suite and the gate is `just conform-racket`.

It stands at **`9`** today: all eighty-nine cases are claimed, so the cumulative
suite is the whole suite and `just racket-gate` and `just conform-racket` are the
same run.

`just conform-engines`, which `just ci` runs, is both engines' gates with a check for
each toolchain in front of it: a developer with no opam switch or no Racket gets a
loud `SKIPPED` line and a green run, because a local gate that fails for a missing
toolchain is a gate that gets routed around. CI has both and enforces both.

## Where the engine stands

M1 is the composition core: §3.9.2's classification against Appendix A, Table 1's
spacing at every boundary and at the two line edges, Table 2's breakability under
§C.3's four levels, whole-paragraph break optimization and line geometry. M2 is
§3.8.3's reduction ladder — Tables 3, 4 and 5, and §3.8.2's hanging punctuation — and
M3 is §3.8.4's: Table 6, the Western word space, the Japanese–Latin ceiling and step
(d)'s residual. M4 is §3.2's orientation, which fell out of M1's work rather than
being claimed: vertical composition is one rule about the transform over the same
geometry. M5 is §3.2.5's tate-chu-yoko, one thing on the line for spacing, breaking
and adjustment alike. M6 is §3.3's ruby in all three kinds and §3.3.8's overhang; M7
is the ornamented complex; M8 is the four structures that set text somewhere a line
does not; M9 is §3.6's tab setting, §3.5.4's widow adjustment and §3.5's indentation,
which closes the suite.

Two things the protocol carries are two different numbers whenever a line was
adjusted, and both are answered here. A placement's `advance` is the advance the
line was **composed from** — the character's own, plus what Table 1 states after it —
and its `inline` is where the character **stands** once §3.8.3's or §3.8.4's ladder
has run. `inline_extent` is the second of the two.

```text
just racket-milestone 9   → exit 0    (89 cases)
just conform-racket       → exit 0    (0 DIFF, 0 protocol errors)
just racket-gate          → exit 0    (CURRENT = 9, so the same run as conform-racket)
just test-engine-racket   → 10185 tests passed
```

Five of the ten censuses agree with the Rust engine at every request, and the sixth
at all but ten:

```text
just census-racket spacing        → 2116 request(s), 0 differing response(s)
just census-racket break          → 2116 request(s), 0 differing response(s)
just census-racket reduction      → 3174 request(s), 0 differing response(s)
just census-racket expansion      → 3174 request(s), 0 differing response(s)
just census-racket vertical       → 5290 request(s), 10 differing response(s)
just census-racket tate-chu-yoko  → 4761 request(s), 0 differing response(s)
```

That is 529 class pairs read back out of Table 1 in four line positions, out of
Table 2 at all four §C.3 levels, out of Tables 3 through 5 on a line that has to
give the spacing back, and out of Table 6 on a justified line with room left over —
from the English transcription of those six PDF pages against the Japanese one the
OCaml engine reads, agreeing bit for bit. Then the same pairs again in vertical
composition, and again with a tate-chu-yoko run standing between them, which is the
only way to reach the cl-30 row and column of all six matrices at all. 20,621 of the
20,631 requests agree bit for bit.

The ten that do not agree are one coordinate family, and none of them is reachable
from the eighty-nine cases:

```text
census/vertical/cl-24+{cl-05,cl-12,cl-24,cl-26,cl-28}/rotated-break
census/vertical/cl-26+{cl-05,cl-12,cl-24,cl-26,cl-28}/rotated-break
```

A rotated run of cl-24 or cl-26 that has to break loses the quarter or half em the
Rust engine pulls it back by, so the line it is on measures wider and the break
lands one cluster early. It is §3.2.5's amount surviving a wrap — the same reading
this engine already takes for an opening bracket, and not yet taken here for a
rotated Western run. `ruby`, `constructs`, `tabs` and `widow` are not clean either;
those four and this one are the next round's work.

Six things the censuses settled that JLReq does not state, and that this engine
answers the way the Rust one does because returning to §3.8 and Appendix D did not
settle them. `docs/decisions/` has since published its own reading of several —
[`tate-chu-yoko-spacing-sources`](../../docs/decisions/tate-chu-yoko-spacing-sources.md)
covers the third of them — and where it does, the reading below is the same one,
which is what a census at zero differences means:

- **§B.2 note 13's collapse reaches the boundary after a line-head word space and
  not the one before a line-end word space.** A closing bracket that ends up before a
  trailing space keeps the half em Table 1 states after it; the character after a
  leading space sees the line head instead.
- **A word space the line *end* collapsed offers §D's own first stage nothing, and
  one the line *head* collapsed offers it the whole width it would have had.** §D.2
  note 4 says there is no reduction opportunity at either edge "since there is
  supposed to be no visible space"; only the line end reads that way here.
- **Appendices D and E measure their room in the Table 1 cell as transcribed, even
  where a section withdrew the space.** §3.2.5 takes six of Table 1's cl-30 cells out
  of the spacing and the ladders go on reading them, so a run on a line that had to
  give space back ends up a quarter em inside the character before it. The same holds
  for a collapsed word space.
- **§3.2.5's half em before an opening bracket belongs to the run and stays when the
  line ends between the two.** Every other amount is withdrawn at a wrap, and Table
  1's `line-end` column answers instead.
- **§B.2 notes 14 through 16 change the class for Table 2 and for nothing else.**
  Tables 1 and 3 through 5 state the same cell for cl-09, cl-10 and cl-11 as for
  cl-19, cl-16 and cl-15 wherever a line without a warichu bracket can reach, and
  Table 6 states a different one at eight coordinates — where the reference engines
  answer at the class the character has.
- **§C.3's `very-loose` level is the two prohibitions its own opening paragraph
  names and nothing else, except that §C.2 note 12's pair of Western characters is
  relaxed only where the two are the same character.**

The startup census in `tables.rkt` is checked against the real files and holds
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

`tests/test-tables.rkt` additionally builds the **Japanese** transcriptions and
compares all 4,932 cells and their note citations against the English ones the
engine runs on. They agree at every coordinate today.
