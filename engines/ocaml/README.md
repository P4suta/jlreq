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
just census tate-chu-yoko
just census ruby
just census constructs
just census tabs
just census widow
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
cl-20 through cl-23, cl-28, cl-29 and cl-30 are what a character *becomes* inside a
construct, so Appendix A lists no code point in them — those rows and columns are
reached by *building* the construct, which is what the `tate-chu-yoko`, `ruby` and
`constructs` censuses are for. Its last column is the class §3.9.2
actually gives that key at the census's own full-em frame, which is not always the
class the census addresses it as: U+0020 is cl-26 whichever of §A.24, §A.25 and
§A.26 lists it, U+2127 and U+0022 are cl-27, and U+3014 and U+3015 are cl-01 and
cl-02. A census that could not say where a representative resolves would be
measuring a coordinate it cannot name.

| Census | Requests | What one answer is |
| --- | ---: | --- |
| `spacing` | 2,116 | 529 pairs × `pair`, `head`, `end`, `interior`, on a line too wide to break or adjust: Table 1 read back out |
| `break` | 2,116 | 529 pairs × the four §C.3 levels, on a one-cluster line with every boundary `allowed`: Table 2 read back out |
| `reduction` | 3,174 | 529 pairs × Tables 3, 4 and 5, the trailing remainder, the line end and hanging punctuation, on a line exactly as wide as the four ems it holds: §3.8.3's ladder read back out |
| `expansion` | 3,174 | 529 pairs × two measures and the three ceilings, plus `table-5` and one line with the trailing member at half the em, justified with a line after it: §3.8.4's ladder read back out |
| `vertical` | 5,290 | 529 pairs × upright, rotated, quasi-Japanese, and §3.1.3's two roles, each on a wide line and on a line with room for one cluster: the same tables asked in the other writing mode, and §3.2's orientation of every placement |
| `tate-chu-yoko` | 4,761 | 529 pairs standing before and after a run of one, two or three members, two runs side by side, and the same line reduced, justified, justified with a neighbor at half the em, and broken: §3.2.5's geometry and every cl-30 coordinate of Tables 1 through 6 |
| `ruby` | 37,030 | 529 pairs on either side of a ruby construct, in seventy variants: mono, group and jukugo; both alignments, both distributions, both jukugo layouts and all four overhang answers; a reading shorter than its base and longer; readings of unequal advance, of a second size and at an em that tiles nothing; the paragraph indent; vertical composition; a justified line, a reduced one and a broken one — §3.3.5 through §3.3.8, §B.2 notes 1, 7, 8, 10 and 11, §C.2 notes 7 and 8, §E.2 notes 6 and 7, and §F |
| `constructs` | 15,870 | 529 pairs beside the other five structures, in thirty variants: emphasis dots of two sizes, a superscript shorter than its complex and longer, a reference mark, a warichu with brackets and without and one that straddles two lines, a furawake of two columns and of three, and a jidori of two characters in four cells and of three in five — vertical, justified and reduced throughout. The cl-20, cl-21, cl-28 and cl-29 rows and columns of all six matrices, §3.3.9, §3.4.2, §3.7.2, §3.7.3, §B.2 notes 9 and 13, §C.2 note 6 and §E.2 note 5 |
| `tabs` | 24,334 | 529 pairs across a tab sign, in forty-six variants: §3.6.2's four kinds of stop; stops the line reaches and stops it has gone past; one sign and two; stops listed ascending, descending and in surplus; the sign at the line head, at the line end and with nothing after it; a measure too tight for the stop and one wide enough for two lines; the caller's breaks stated and unstated; every alignment and none; both writing modes; the paragraph indent; and the sign beside a construct and inside an emphasis run, a superscript, a jidori and a tate-chu-yoko run — §3.6.1 through §3.6.3, and §3.8.1 on the line a cut leaves short |
| `widow` | 13,225 | 529 pairs on a paragraph whose last line is one cluster short of the minimum, in twenty-five variants: minima the paragraph can meet and cannot; the pair on the line that gives a cluster up and on the line that gains one; break sets that leave the search one choice and none; both remainder and both preference settings; the indent; vertical composition; and every alignment and none — §3.5.3, §3.5.4 and §3.8.1 |

The registry that names them is `kinds` in `census.ml`, and nothing else in the
file knows how many there are. A census is a name, a sentence and a function that
emits requests.

Six things the censuses are deliberately shaped for. Every `expansion` line carries
three interior boundaries, because §3.8.4's stages and their ceilings are
indistinguishable from "hand the whole shortfall to the only place that will take
it" on a line with one — which is the shape all six of the built-in suite's
expansion cases have. The `mixed-em` variants set one cluster at half the em,
because Table 6 names a class pair and no neighbor (ADR 0021), so an engine has to
decide whose em a quarter of an em is a quarter of, and on a line of one size that
decision is invisible. The `tate-chu-yoko` runs carry members of unequal and odd
advances, because §3.2.5 centers the whole string and says nothing about which way
half of an odd width rounds, and members of unequal block ems, because what a run
takes up along the line and what it takes up across one are two different numbers
that a square member makes look like one. And the `vertical` census states §3.1.3's
two roles on every class in turn, because the section names two marks and the
engines have to agree about the twenty-one classes it does not name. And the `ruby`
census sets some of its readings at two fifths of the base character's em rather
than at §3.3.3's half, because a reading whose characters divide the base character
exactly *tiles* the base characters it is set over — and §F.2's own questions, which
base character a reading may reach into and how far, are then answered by the tiling
instead of by the rule. Half the built-in suite's §F cases are at that ratio. And the
`constructs` census gives a warichu four half-em characters in one variant and five in
another, because §3.4.2's "the length of the second line should not be longer than the
length of the first line" is satisfiable at four and not at five, which is where it
stops being a bound and starts being a preference.

Not every census can offer every boundary as a break. A run is indivisible — §C.2
note 13 for a tate-chu-yoko run, §3.3.5 and §3.3.6's shared sentence and §C.2 note 8
for a base character group, §C.2 note 6 and §3.7.1 for an ornamented character
complex, §3.7.3's own subject for a jidori, and §3.7.4's two named classes for
everywhere else inside a formula — and a request that states a break inside one is
*refused* rather than answered, which would end the census rather than measure
anything. `tate-chu-yoko`'s break variants name the three boundaries that exist;
`ruby`'s give every construct one base character, so that every boundary they offer is
one between two groups; and `constructs` offers breaks only inside the two structures
that divide, a warichu and a furawake, plus inside an emphasis run, which is one
complex per character and divides between any two of them.

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
transcribed.

Twenty-six of them were found while this engine was written: two from §3.8.4's
ladder and three from §3.2.5's tate-chu-yoko, found by the `expansion` and
`tate-chu-yoko` censuses rather than by any conformance case; six from §3.3's ruby,
seven from the constructs of §3.3.9, §3.7.1, §3.4, §3.7.3 and §3.7.4, and seven from
§3.6's tab setting and §3.5's paragraph end. Every one of them is now published in
`docs/decisions/`, bundled by subject, with the census that observed it and the
sentence whose silence permits it:

| Reading | What it settles |
| --- | --- |
| [expansion-ladder-scope](../../docs/decisions/expansion-ladder-scope.md) | Which coordinates §3.8.4's Japanese–Latin ceiling is asked at, and which sites step (d) re-levels |
| [tate-chu-yoko-spacing-sources](../../docs/decisions/tate-chu-yoko-spacing-sources.md) | Whether §3.2.5's prose or Table 1's cl-30 row states the space beside a run, and whether the ladders read cl-30 cells §3.2.5 set no space at |
| [construct-break-refusal](../../docs/decisions/construct-break-refusal.md) | Whether a break stated inside an indivisible construct is refused or declined, at which coordinate for each construct, and where §3.7.4 lets a formula break |
| [ruby-overhang-permission](../../docs/decisions/ruby-overhang-permission.md) | Whether §3.3.8 rule 2's kana neighbor is a script or a class, and whose em a Table 1 `hang` term was measured in |
| [ruby-distribution-and-rounding](../../docs/decisions/ruby-distribution-and-rounding.md) | What §3.3.6 does for a run of one, what its outer units are, which way an odd unit falls, and what §F.3's self-referring total evaluates to |
| [ornamented-complex-geometry](../../docs/decisions/ornamented-complex-geometry.md) | What an emphasis mark is centered on, how many complexes an emphasis run is, and where §3.7.1's annotation sits |
| [stacked-structure-geometry](../../docs/decisions/stacked-structure-geometry.md) | Which positions a warichu may divide at, whether its balance sentence is a bound, and whose advance a structure's trailing space is part of |
| [tab-line-correspondence](../../docs/decisions/tab-line-correspondence.md) | What a tab sign with no stop left does, whether §3.6.3's cut answers to §3.1, and what §3.6.1's count is counted over |
| [unstated-alignment](../../docs/decisions/unstated-alignment.md) | What a request that states no `alignment` asks for |
| [inexpressible-advance-remarks](../../docs/decisions/inexpressible-advance-remarks.md) | Whether an Appendix A Remarks cell naming only an advance the protocol cannot express excludes its listing |
| [jidori-inserted-space-locale-split](../../docs/decisions/jidori-inserted-space-locale-split.md) | How many sides of an inserted space §3.7.3 opens, where its two renderings state opposite rules |

The last of those is not a silence but a divergence between the two locales of one
sentence, so it is published as `Adjudicated` rather than `Unstated`. Two further
questions were checked against the reference engine and found to be *readings* rather
than policies — §E.1's `1/4–1/2` cells under Table 5, and `adjustment.expansion_order`,
which selects nothing today — and both are recorded in
[expansion-ladder-scope](../../docs/decisions/expansion-ladder-scope.md) so that a later
reader does not re-derive them.

The comments in `lib/` and the cases in `test/` that name this section name the file
that now carries the argument. A policy found from here on is recorded here first and
promoted the same way.

### Where the two engines disagree

Two coordinates are not policies this engine adopted but places the two engines
answer differently, and the rule above is what to do about them: return to JLReq,
record the disagreement, and do not settle it by copying. Both are the same
question — what a tab sign is doing when it stands inside a structure that does not
set its text along the line. Both are filed against this repository, with the
protocol-v1 request that reproduces each one and both engines' answers:

- [#12](https://github.com/P4suta/kumihan/issues/12) — a tab sign that is the first
  character of a tate-chu-yoko run.
- [#13](https://github.com/P4suta/kumihan/issues/13) — a tab sign inside a warichu or
  a furawake.

A reading reaches `docs/decisions/` once one of them is settled; until then
[tab-line-correspondence](../../docs/decisions/tab-line-correspondence.md) publishes
what the two engines *do* agree on about §3.6.3 and names these two as excluded.

§3.6.3 corresponds the signs of a line with the stops of that line. A warichu's and
a furawake's sublines run beside the line and a tate-chu-yoko run runs across it;
each is one position on the line however many characters it holds, so a coordinate
inside one is not a position a stop could name. This engine therefore has a sign
there take no stop and set the advance it was shaped with, and measures the stop of
the *next* sign from the line's own walk, where the whole structure is one step. It
is the only reading under which the width a line is measured at and the width it is
set at are the same number. Where the sign stands strictly inside a tate-chu-yoko
run the two engines agree on exactly that, and the `tabs` census holds them to it.

**A sign that is the first character of a tate-chu-yoko run.** This engine reads a
run's first character as being in the run like every other, so the sign takes no
stop and §3.6.3's cut is never chosen there. The reference engine ends the line
before it — which only §3.6.3's fourth case does, and only to a sign of the line —
and then, on the next line, sets it as a member of the run taking no stop. Its two
answers to "is this a sign of the line" are not the same answer.

**A sign inside a warichu or a furawake.** Given `A⇥B⇥C` — all proportional, half an
em each, the first two characters and the sign between them inside a warichu, an
allowed break inside it, a four-em measure and stops at 1200 and 3000 — the
reference engine reports the block at its own width of 1000 and then sets the second
sign at 1000 with an advance of 1800, which is the distance to the stop at 3000
measured from a cursor of 1200. Neither 1000 nor 1200 is the other's answer to where
the second sign stands. Reading the same paragraph with stops at 2000 and 2500 it
gives the second sign the advance it was shaped with and no stop at all. This engine
answers the first paragraph with an advance of 200 — the distance from 1000 to the
stop at 1200 — and the second with 1000.

The same divergence is visible with one sign and no second: a line the reference
measures wider than it sets is a line it reduces when it did not need to, so `〉⇥〉`
inside a warichu in a three-em measure comes back 500 units narrower from the
reference than the geometry the reference itself reports.

Because a census is a gate and a gate that is red is not one, the `tabs` census
covers a sign beside a construct and inside an emphasis run, a superscript, a
jidori and a tate-chu-yoko run, and deliberately covers neither of these two shapes.
`test/test_pipeline.ml` pins this engine's answer instead, and names this section.

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
just ocaml-milestone 5   # select M1..M5 out of the built-in suite and run them
```

The selection is checked: the recipe fails if an identifier in an `.ids` file
names no case in the suite, so a typo or a renamed case cannot shrink the gate
quietly.

`milestones/CURRENT` holds one number — the milestone the engine claims today.
`just ocaml-gate` runs the cumulative suite through it, and the `conform-ocaml`
job in CI runs `just ocaml-gate`, so that one digit is what a merge is held to.
Advance it in the pull request that makes the milestone pass, never before.
At `9` the cumulative suite is the whole suite and the gate is
`just conform-ocaml`. It stands at `9` today, so the two runs are the same run and
the engine is held to every case there is.

`just conform-engines`, which `just ci` runs, is the same gate with a check for
the toolchain in front of it: a developer with no opam switch gets a loud
`SKIPPED` line and a green run, because a local gate that fails for a missing
toolchain is a gate that gets routed around. CI has the toolchain and enforces
it.

## Where the engine stands

M1 is the composition core: classification (§3.9.2 and Appendix A), Table 1
spacing, Table 2 breakability with §C.3's four conventions, whole-paragraph break
optimization, and line geometry. M2 is §3.8.3's reduction ladder — Tables 3, 4 and
5, and §3.8.2's hanging punctuation — and M3 is §3.8.4's expansion ladder: Table 6,
the Western word space, the Japanese–Latin ceiling, and step (d)'s residual. M4 is
§3.2's orientation: a proportional cluster rotated a quarter turn (§3.2.6), a
fixed-width Western character standing up as quasi-Japanese (§3.2.4), and §3.1.3's
two vertical-only exceptions. M5 is §3.2.5's tate-chu-yoko — a horizontal string set
solid and centered across the vertical line, one thing on the line for spacing,
breaking and adjustment alike (§C.2 note 13, §E.2 note 12). M6 is §3.3's ruby, in
all three kinds: §3.3.5's mono ruby set solid against one base character, §3.3.6's
group ruby distributing whichever of the base and the reading is shorter across the
other, §3.3.7's jukugo ruby — one run per base character while every run is short
enough, and the whole compound at once when one is not, by §3.3.6's own method or by
§F's — and §3.3.8's overhang, which is what decides whether a reading that does not
fit its base pushes the line apart or is set over what stands beside it.

M7 is the ornamented complex: §3.3.9's emphasis dots, half the size of the base
character they mark and centered on it, and §3.7.1's superscripts and reference
marks, which are one complex for breaking (§C.2 note 6) and for expansion (§B.2
note 9, §E.2 note 5) alike while an emphasis run is one complex per character. M8 is
the four structures that set text somewhere a line does not: §3.4's warichu, divided
into two lines as near the same length as they can be made and centered across the
main one; §3.7.2's furawake, divided where the caller said and as many ways;
§3.7.3's jidori, spread evenly over a declared number of full-em cells; and §3.7.4's
formulae, whose two named classes are where a break may fall and nowhere else.

M9 is what a line does at its two ends: §3.6's tab setting — the four kinds of stop
(§3.6.2), the correspondence between the signs of a line and its stops (§3.6.3), and
§3.6.3's fourth case, where a sign that has run the stops out takes the rest of the
line with it to the next one — and §3.5.4's paragraph end, where a last line shorter
than the caller's minimum is bought off by shortening the line before it. §3.5.2's
indent runs through both: it is what makes the same stop reachable on the first line
and not on the rest.

Every one of the eighty-nine requests is parsed completely and answered bit for bit.

```text
just ocaml-milestone 9    → exit 0    (89 cases)
just conform-ocaml        → exit 0
just ocaml-gate           → exit 0    (CURRENT = 9, so the same run)
just ocaml-test           → 689 check(s), 0 failure(s)
```

Where the whole built-in suite stands against `milestones/`:

| M | Subject | Passing |
| --- | --- | --- |
| M1 | classification, spacing, breakability, geometry | 18 / 18 |
| M2 | reduction (Tables 3–5), hanging | 7 / 7 |
| M3 | expansion (Table 6), justification, reclassification | 10 / 10 |
| M4 | vertical composition, rotation, orientation | 5 / 5 |
| M5 | tate-chu-yoko | 9 / 9 |
| M6 | ruby | 23 / 23 |
| M7 | emphasis dots, ornamented complexes | 4 / 4 |
| M8 | warichu, furawake, jidori, formulae | 10 / 10 |
| M9 | tab stops, widows, indentation | 3 / 3 |

M4 fell out of M1's work rather than being claimed — vertical composition is one
orientation rule over the same geometry — and it stayed green through M5's, M6's,
M7's, M8's and M9's changes to the same code.

All ten censuses agree with the Rust engine at every request:

```text
just census spacing        → 2116 request(s), 0 differing response(s)
just census break          → 2116 request(s), 0 differing response(s)
just census reduction      → 3174 request(s), 0 differing response(s)
just census expansion      → 3174 request(s), 0 differing response(s)
just census vertical       → 5290 request(s), 0 differing response(s)
just census tate-chu-yoko  → 4761 request(s), 0 differing response(s)
just census ruby           → 37030 request(s), 0 differing response(s)
just census constructs     → 15870 request(s), 0 differing response(s)
just census tabs           → 24334 request(s), 0 differing response(s)
just census widow          → 13225 request(s), 0 differing response(s)
```

That is 529 class pairs read back out of Table 1 in four line positions, out of
Table 2 at all four §C.3 levels, out of Tables 3 through 5 on a line that has to
give the spacing back, and out of Table 6 on a justified line with room left over
— from two independent transcriptions of the same six PDF pages, agreeing bit for
bit. Then the same pairs again in vertical composition, where §3.9.2 reads the
frame differently and every placement carries an orientation, and again with a
tate-chu-yoko run standing between them, which is the only way to reach the cl-30
row and column of all six matrices at all. Then the same pairs a third time on
either side of a ruby construct, which is what reaches the cl-22 and cl-23 rows and
columns — and, because §3.3.8's own permission is stated in the same cells as the
spacing, is the only place where reading a matrix cell wrong and reading the prose
wrong look different. Then a fourth time beside the five remaining structures, which
is what reaches the cl-20, cl-21, cl-28 and cl-29 rows and columns — the last four
coordinates of the six matrices that no Appendix A key can name. Then a fifth and
sixth time across a tab sign and on a paragraph whose last line is a widow, where
what varies is not a cell of a matrix but which line the pair ends up on and how far
the line it is on then has to be opened. 111,090 requests in all, and no answer
differs by one unit.

The two vertical censuses found three things the eighty-nine cases do not reach:
two readings of Appendix A's `字幅は四分角` that differ at U+0020 and U+2010, and
the fact that §3.2.5's prose rather than Table 1's cl-30 cells is what the
reference engine sets. The ruby census found six more, every one of them a place
where §3.3's prose and Appendix B's own annotation of it can be read two ways, and
the `constructs` census six more again — including one, the space a warichu's own
closing bracket carries, that only `(cl-29, cl-05)` makes visible at all. Seven more
came out of §3.6 and §3.5 — six about what a tab sign does once its stops have run
out, and one about what a request that states no alignment at all is asking for; the
`tabs` and `widow` censuses are where they were checked at scale rather than where
they were first seen, because two of the six are refusals and a census only ever
sends requests both engines accept. Those twenty-two, with the two §3.8.4's ladder
contributed, the one Appendix A did and the one §C.2 note 13 did, are the
twenty-six of "Observable policies with no written source" above; the one coordinate
where the two engines do not agree is in "Where the two engines disagree" below it.

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
