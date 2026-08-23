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

§3.2.5's tate-chu-yoko contributed three, all found by the `tate-chu-yoko` census
rather than by any conformance case:

- **§3.2.5's prose is the whole of the spacing beside a run, and its own Note is
  not.** The section states four amounts — a half em after a comma (cl-07), a
  closing bracket (cl-02) or a mid-line full stop (cl-06), a half em before an
  opening bracket (cl-01), solid otherwise — and then says that "the details … are
  described as a complete table in §B". Table 1's cl-30 row and column state those
  four *and six more*: a quarter em against a middle dot (cl-05) in both directions,
  and against cl-21, cl-24, cl-25 and cl-27 in both directions. The reference engine
  sets the four and not the six, so the prose wins over the sentence that points at
  the table. Neither sentence says which of them is the exception.
- **The reduction and expansion ladders read their matrices at face value at the
  same coordinates.** Table 3 states `1/4-0 stage 4` at (cl-30, cl-05) and Table 6
  states `1/4-1/2 stage 2` at (cl-30, cl-27), and both apply — even though §3.2.5
  put no space at that boundary for §3.8.3 to take back. The observable consequence
  is that a run on a line that had to give space back ends up a quarter em *inside*
  the character before it. The expansion ceiling, by contrast, is measured against
  the space §3.2.5 actually set and not against Table 1's; the census pins that one
  at 156 requests.
- **A break stated inside a run is refused, not declined.** §C.2 note 13 says there
  is no line break opportunity between two characters of one run, which an engine
  could implement by never taking the opportunity. The reference engine refuses the
  request instead, with `input.break-inside-construct`, for an `allowed` break and a
  `mandatory` one alike — and in horizontal composition too, where a tate-chu-yoko
  construct changes nothing else at all. §3.3's base character groups are refused the
  same way and at a narrower coordinate: a break inside one *run* of a ruby construct
  is refused, and one at a run boundary — which §C.2 note 8 grants a jukugo compound
  outright — is answered. §C.2 note 6 and §3.7.1 refuse one inside a `script` or a
  `reference-mark` construct, §3.7.3 one inside a jidori, and §3.7.4 one inside a
  formula that is not beside a math symbol or a math operator. Two structures divide
  and the built-in suite states breaks inside both: a warichu (§3.4.2) and a furawake
  (§3.7.2). An emphasis run divides too, because §3.3.9 makes each of its base
  characters a complex of its own.

§3.3's ruby contributed six, all found by the `ruby` census rather than by any
conformance case. Ruby is the one subject where the matrices and the prose overlap
most — sixty of Table 1's cells carry §B.1's `hang` annotation, and every rule
§3.3.8 states has a cell — so most of these are about which of the two the reference
engine reads at a coordinate where they differ:

- **§3.3.8 rule 2's kana neighbor is read by script and not by class.** The rule
  names "hiragana (cl-15), katakana (cl-16), prolonged sound mark (cl-10) or small
  kana (cl-11)", which is two scripts and two classes spelled in them, and Table 1
  carries a `ruby hang` cell for each of the four. The reference engine reads
  `spec/derived/scripts.tsv` instead, and the two readings part at exactly the marks
  the scripts and the classes disagree about: U+30FC, the prolonged sound mark, is
  cl-10 — which the rule names — and Script=Common, and a reading is *not* set over
  it; U+30FD and U+30FE, the katakana iteration marks, are cl-09 — which the rule
  does not name — and Script=Katakana, and a reading is. `ruby.overhang_kana`'s `jis`
  answer takes katakana out of the same test, so U+309D, a *hiragana* iteration mark,
  still gets a reading over it there. Table 1's four kana cells select nothing at all.
  Nothing before M6 could see this: cl-09 and cl-10 are the same row and the same
  column in all six matrices except at those four ruby coordinates.
- **A `hang` term measured from the ruby object's own em is not a space a reading may
  go over.** Table 1 marks ten of its cl-22 and cl-23 cells `<amount> be|af hang`, and
  the side names whose em the amount was taken from. Where it is the neighbor's — the
  half em after a closing bracket before the object, the half em before an opening
  bracket after it, the quarter em beside a middle dot — the reading goes over it,
  which is what §3.3.8 describes. Where it is the ruby object's own — the quarter em at
  `(cl-22, cl-24)`, `(cl-22, cl-25)`, `(cl-22, cl-27)` and their mirrors — it does not.
  The annotation says `hang` in both.
- **A run over one base character whose reading is longer is set by §3.3.5 and not by
  §3.3.6.** §3.3.6's two methods are both stated over "the inter-character spacing
  between each adjacent base character" and the end gaps that go with it, and a run
  over one base character has no adjacent base character. The reference engine centers
  the reading on it and lets it hang over both neighbors — so `ruby.group_distribution`
  selects nothing for such a run, `flush` and `jis` alike, which is visible because the
  two answers do differ at the same ratio for a run over two.
- **A group run's leading and trailing shares are spacing on the line and never an
  overhang.** For a run over two or more base characters §3.3.6's `1 : 2 : … : 2 : 1`
  puts one unit before the first base character and one after the last, and those two
  are inserted even where §3.3.8's own permission would have let the reading over the
  neighbor instead. A mono run's two shares are the opposite: always an overhang, as
  far as the permission goes, and spacing only for what is left.
- **§3.3.5's centering takes the lower half of an odd difference, and the space its
  overflow forces takes `adjustment.remainder`.** A reading 1665 units wide on a
  1000-unit base character opens 333 units before that base character and 332 after
  — the remainder answer's own order — while the reading itself starts 332 back from
  it rather than 333. One is a center and the other is two adjustment sites, and
  §3.3.5 and §3.3.8 rule 1 are silent about both roundings.
- **§F.3's total is the least one the compound fits at.** §F.3 states it as a formula:
  "Total inter-character spacing = (the sum of the length of those ruby characters
  forced out from the corresponding base character) - (the sum of the length of those
  ruby characters which overhang other base characters) - (the sum … which overhang
  other non-base characters)." The second and third terms are geometric facts about a
  compound whose base characters have already been pushed apart *by the total being
  computed*, so the formula refers to its own result and an engine cannot evaluate it
  in the order it is written. The reference engine's answer is the smallest total at
  which every reading has somewhere to go — a ruby character's em into the base
  character beside it, and §3.3.8's own allowance outside the compound — which is
  what this engine finds by bisection. At a ruby em that divides the base character
  exactly the two are the same number, which is why half the suite's own §F cases
  cannot tell them apart.

§3.3.9, §3.7.1, §3.4, §3.7.3 and §3.7.4 contributed seven more, six of them found by
the `constructs` census rather than by any conformance case. The four classes those
sections build — cl-20, cl-21, cl-28 and cl-29 — have no Appendix A key, so nothing
before M7 could reach their rows and columns at all:

- **§3.3.9's "center of the base characters" is the center of the advance the line
  gave it, spacing and all.** The mark is half its base character and centered on it,
  and the two readings of what it is centered on — the character's own em box, or what
  the character occupies on the line — part wherever Table 1 states a space after the
  base character. An emphasis run is cl-21, so a quarter em stands after it before an
  ideograph, and the mark sits an eighth of an em later than the em-box reading would
  put it. The same reading decides where §3.7.1's annotation is centered.
- **§3.3.9 makes each base character its own ornamented character complex, and §3.7.1
  makes the whole construct one.** §B.2 note 9, §C.2 note 6 and §E.2 note 5 are all
  stated about "two consecutive characters belonging to the same ornamented character
  complex (cl-21)", and JLReq never says how many complexes an emphasis run is. The
  reference engine answers one per character: Table 6's quarter em opens between two
  emphasized characters of one run and never inside one superscript's complex, and a
  break stated inside an emphasis run is answered while one inside a `script` or a
  `reference-mark` construct is refused.
- **§3.7.1's annotation is centered on its complex, hangs over both neighbors where it
  is longer, and opens the line nowhere.** §3.7.1 says the geometry is "implementation
  definable" and says the annotation is "set after the base character"; the reference
  engine sets it across the complex instead and lets it overhang without §3.3.8's kind
  of permission and without §3.3.6's kind of spacing. `ruby.alignment` selects nothing
  there either — §3.3.5's question is about a reading and this is not one.
- **§3.4.2's "a position where line breaking is permitted" is a position the caller
  stated.** A warichu divides at one of the request's own break opportunities where it
  offers any, and at whichever cluster boundary balances the two lines best where it
  offers none — so the sentence is a restriction where the caller made one and nothing
  where the caller did not. Table 2 is not consulted: a warichu divides after an
  opening bracket, which §C.3 forbids a line to.
- **§3.4.2's "the length of the second line should not be longer than the length of the
  first line" is a preference among the stated positions and not a bound on them.**
  Where every position the caller offered leaves the second line longer, the least
  unbalanced of them is taken rather than the note being left undivided. Two positions
  that balance equally are settled by the earlier one.
- **A stacked structure's own last character carries no space, and the structure does.**
  The bracket that closes a warichu is the last character of the *structure* rather
  than of the line, so the space Table 1 states after it stands after the whole block
  and is no part of the bracket's reported advance — which is visible at
  `(cl-29, cl-05)`, the quarter em a middle dot takes after a warichu. The same holds
  at the end of every subline: the character that ends one takes nothing after it, and
  Table 1's line-end column is asked of the line and of nothing else.
- **§3.7.4's two named break classes are the whole of where a formula may break.**
  "A line break in a mathematical formula is done, when possible, at an equals sign
  (cl-17) ... or at an operator (cl-18)" reads as a preference, and the reference
  engine reads it as a rule: a break with a math symbol or a math operator on either
  side of it is answered and every other break inside a formula is *refused*, for a
  display formula and an inline one alike.

§3.6's tab setting and §3.5's paragraph end contributed seven more. They were found
by probing the reference engine directly with tab stops the eighty-nine cases never
state, and all seven then held across the 37,559 requests of the `tabs` and `widow`
censuses:

- **A tab sign whose stops the line has gone past ends the line, and that cut
  answers to no character class.** §3.6.3's fourth case — "if there is no tab
  position corresponding to the target string, the string should be set from the tab
  position of the next line" — says where the string goes and not that the line ends
  before the sign rather than the sign taking some default width where it stands.
  The reference engine ends the line there, and does so at boundaries Table 2 would
  never let a line end at: a line whose last character is an opening bracket (cl-01)
  is the answer when the sign follows one. The cut is §3.6's and not §3.1's.
- **A tab sign standing at the line head with no stop left keeps its line and takes
  one em.** It is the one place §3.6.3's fourth sentence has nothing to say, because
  there is no earlier boundary to send the sign to. The width taken is one em of the
  paragraph's own size, which is a number §3.6 never mentions.
- **A sign standing inside a construct keeps its line too, for the same reason.**
  Every construct is at least one object on the line, and §3.6.3's cut is not a break
  opportunity that a rule about characters could permit or forbid — it is a line
  boundary, so the only thing that can withhold it is there being no boundary at that
  point. A sign inside an emphasis run, a superscript, a reference mark, a jidori, a
  formula or a base character group runs its stops out and takes one em where it
  stands; a construct that begins or ends exactly at the sign leaves the cut
  available, because the sign is then beside the construct rather than in it.
- **Stops are taken in the order they stand along the line, not the order the request
  lists them.** A request may list them descending; each sign takes the nearest stop
  ahead of the cursor either way. §3.6.3 says "in order" and the only order a line
  has is position.
- **A stop must lie strictly inside the measure, and that is checked whether or not
  the source holds a tab sign.** A stop at the measure exactly is refused. §3.6 says
  a stop is a position in the line and says nothing about the ends of one.
- **§3.6.1's count of stops is enforced between mandatory breaks, and a surplus is
  allowed.** "If there is more than one tab sign, it is necessary to set the same
  numbers of tab positions and tab types as the number of tab signs" counts signs
  *in a line*, and which line a sign lands on is what composition decides — so the
  only division into lines that validation can see is the caller's own mandatory
  breaks. The reference engine refuses a stretch between two of them that holds more
  signs than there are stops, and accepts one that holds fewer, so "the same number"
  is read as a floor and not as an equality.
- **A request that states no alignment is justified, and that is not the same answer
  as `start`.** The protocol schema gives `alignment` four values and no default.
  §3.8.1 is what an unstated one means: "within a paragraph, lines are created by
  separating character sequences at places where line breaking is not prohibited",
  and every line but a short last one is then adjusted to the measure — while
  `start` is one of §3.5.3's four answers, which a caller who wants a flush short
  line asks for. It is observable wherever a non-last line comes up short and Table 6
  offers it a site, which of the eighty-nine cases only
  `3.5.4/widow-keeps-two-clusters-on-last-line` reaches: the line a widow minimum
  shortened is opened back out to the full measure.

One bullet of §3.7.3 is not a policy but a divergence between the two locales of the
same sentence, and is recorded here because a reader will otherwise re-derive it:

- §3.7.3's own list says of an inserted Western word space (cl-26) or ideographic
  space (cl-14), in English, to "add the same spacing to those space characters as is
  being added to the other characters", and in Japanese the opposite — 空白の前及び
  後ろの2箇所ではなく，空白の前（又は後ろ）だけとする, one of the space's two sides and
  not both. The reference engine opens both, which is the English reading, and this
  engine matches it. `spec/derived/` carries the two texts and settles neither.

One more is about Appendix A rather than about any section:

- **A Remarks cell naming only an advance the protocol cannot express excludes its
  listing, rather than qualifying nothing.** `字幅は四分角` (a quarter em) and
  `字幅は三分角` (a third of an em) name widths the `frame` vocabulary — `full-em`,
  `half-em`, `proportional` — has no word for. Reading them as "no width stated"
  makes the listing available at every frame; reading them as "a width no caller can
  declare" makes it available at none. Two keys tell the readings apart, and the
  reference engine takes the second at both. U+0020 SPACE is listed as a grouped
  numeral (§A.24) and a unit symbol's character (§A.25) at a quarter em and as the
  Western word space (§A.26) unqualified, so it stays cl-26 however the caller
  labels the occurrence; U+2010 HYPHEN is listed as a hyphen (§A.03) at a quarter em
  and as a Western character (§A.27) proportional, so a proportional hyphen is
  cl-27.

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

### Where the two engines disagree

Two coordinates are not policies this engine adopted but places the two engines
answer differently, and the rule above is what to do about them: return to JLReq,
record the disagreement, and do not settle it by copying. Both are the same
question — what a tab sign is doing when it stands inside a structure that does not
set its text along the line.

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
