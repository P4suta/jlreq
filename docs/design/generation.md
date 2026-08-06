# Specification data: generation and attestation

[ADR 0009](../adr/0009-generated-data-and-attested-transcription.md) splits specification
data in two, because W3C publishes JLReq in two forms. This document is the pipeline.

## The two categories

**Derived** data comes from a machine-readable source and is regenerated deterministically:
Appendix A's 1133 keys across 25 tables, every legend, every appendix note, §C.3's four
strictness levels, §3.8.3's and §3.8.4's ladders, the rule inventory, the policy space, the
cl-19 ideograph predicate, the compatibility folding table, and the script property behind
§C.2 note 3's small-kana fallback. A hand edit to a derived file is a bug even when it is
correct, because the next specification revision will not carry it forward.

**Captured** data is the cells of Tables 1 through 6, and one much smaller family described
below. Measured in the published document, the entire range covering Appendices B through E
contains prose, legends, notes lists, and anchors of the literal form `See "…" (PDF)` — and
exactly one `<table>` element, which renders the two substitution examples inside §B.2 note
14. Roughly 5400 cells exist only as PDF, and the reduction and expansion priority ordinals
are encoded as cell background color with the color-to-ordinal key published as a raster
image, so the ordering exists nowhere as text at all.

The smaller family is the arrangements JLReq states only in a figure. §3.4.3 says a warichu
that will not fit "will wrap onto the following line, and will be set as shown in Figure 148
or Figure 149", and the reading order and the per-line lengths are in those two images and
nowhere in the text; the same is true of §3.7.2's furiwake arrangement. Measured in the
snapshot, both figures are `<img>` elements whose only text is a caption. These are captured
for exactly the reason the matrices are — the content is not machine-readable — and they
carry the same per-datum provenance, naming the figure they were read from. They are a
handful of rows rather than thousands, and double entry applies to them unchanged.

Scraping those PDFs was considered and rejected. The most important derived fact would
still be read by eye from an image; a re-issue with different colors would make the
extractor silently wrong rather than loudly broken; and the result would let a bad
extraction pass as machine-derived, which suppresses exactly the scrutiny an honest
transcription invites.

## Layout

```text
spec/
  PROVENANCE.toml            URLs, retrieval dates, SHA-256 of every upstream file
  snapshot/
    index.html               the JLReq snapshot            (derived source)
    ucd/
      PropList.txt           Unified_Ideograph             (derived source)
      Scripts.txt            Hiragana / Katakana           (derived source)
      UnicodeData.txt        Wide / Narrow decomposition   (derived source)
  derived/
    appendix-a.tsv           1133 keys, class, Remarks (both locales)
    anchors.tsv              rendered section number → anchor id → heading
    notes.tsv                every note of §B.2, §C.2, §D.2, §E.2, both locales
    rules.tsv                the rule inventory: address, standing, statement
    questions.tsv            the policy space: question, choices, permitting section
    defects.tsv              the recorded defects of the published document
  captured/
    table1.en.tsv            transcribed from tables/table_en2.pdf
    table1.ja.tsv            transcribed from tables/table_ja2.pdf
    ...                      through table6
    figures.tsv              the arrangements published only as images (§3.4.3, §3.7.2)
    invariants.tsv           each cross-table check, with the sentence justifying it
  upstream/                  gitignored; the PDFs, if a developer fetched them
tools/jlreq-gen/             excluded workspace, stage 1, may parse HTML
crates/*/src/generated/*.rs  committed output of stage 2
data/manifest.toml           SHA-256 of every generated file and every input
docs/decisions/*.toml        this project's published readings of silences
```

Two crates take their generated tables in a module of their own rather than under a
`generated/` directory: `jlreq-spec`'s rule inventory is emitted into `src/rule.rs` and its
policy space into `src/policy.rs`, beside the vocabulary that indexes each. A rule address,
its inventory and the identifier that reads it are one module's worth of subject. Everywhere
else the emitted tables are several files and the directory is what keeps them together, so
`generate --check` reads the two shapes and `docs/design/api-spine.md`'s file maps state the
same layout this list does.

`spec/captured/figures.tsv` emits to `crates/jlreq-line/src/generated/figures.rs`, which is
the module the earlier revision of this document did not name. The arrangements of §3.4.3
and §3.7.2 are consumed by the segment code, so they belong beside the line layer's other
generated data and are byte-checked by `generate --check` like every other emitted file.

Note the filename hazard, which is recorded rather than absorbed: the PDF for Table 1 is
`table_en2.pdf`, off by one, and the HTML legend anchors are off by one in the same
direction — `legend_of_table_2` renders "B.1 Legend of Table 1". Any tool keyed on the
anchor id misnumbers a table. `derived/anchors.tsv` is therefore built from each heading's
*rendered* section number, with the anchor id as a secondary column, and the generator
asserts the known off-by-one so a corrected upstream fails loudly.

## Two stages

**Stage 1**, `tools/jlreq-gen`, lives in its own workspace excluded from the root. It may
parse HTML, because it is not published and its dependency tree does not tax the crates
that are. It emits `spec/derived/*.tsv` and nothing else. It runs only when the
specification revises.

**Stage 2**, `cargo run -p xtask -- generate`, is dependency-free — TSV is trivial to
read — and emits `crates/*/src/generated/*.rs` from `spec/derived/` and `spec/captured/`.
It runs in CI on every commit.

The split exists for two reasons. `xtask` has an intentionally empty dependency table
because it is the tool that enforces the core's emptiness, and parsing 2.3 MB of HTML with
`std` alone is fragile. And an in-workspace generator would put an HTML parser into a
dependency graph where `deny.toml` sets `bans.multiple-versions = "deny"` with empty skip
lists, making one transitive duplicate a permanent tax on the *published* crates for the
sake of a build tool. A `build.rs` generator is not merely undesirable but impossible: the
purity gate rejects a `[build-dependencies]` table of any kind.

The intermediate TSV is itself a deliverable. A JLReq reader can review
`spec/derived/appendix-a.tsv` and `spec/captured/table1.en.tsv` without reading a line of
Rust, which is the same audience [ADR 0006](../adr/0006-conformance-suite-as-artifact.md)
is written for.

## What the derived stage must get right

**Read both locale columns.** Appendix A's Remarks and the appendix legends are published
in English and Japanese, and they are not in one-to-one correspondence. Measured: three
cl-25 Remarks cells contain the bare string `プロポーショナル` with no locale span, so an
English-locale extraction yields an empty remark for three rows that mean
"proportionally-spaced"; the cl-24 and cl-25 role lines (位取りの空白, 位取りのコンマ) exist
only in Japanese; and §E.1's Japanese legend permits a third em — `又は三分アキまで` — where
the English gives only a half; and §3.8.3's first reduction step reads "the same width
reduction is applied to all spaces on the target line at the same time" in English against
`文字サイズ比で均等に` in Japanese, which is a different operation on a line of mixed sizes.
The extractor emits both columns and **fails** on a divergence that is not in
`defects.tsv`, rather than picking one.

**Detect defects rather than absorbing them.** `U+216B` appears twice in the cl-19 body, so
cl-19 has 465 rows and 464 members. That is the only duplicate in Appendix A, it is a
defect in the published table, and the generator fails on an unrecorded duplicate.
`defects.tsv` records each known one with its evidence, and `attest` requires the detected
set to equal the recorded set — so a defect fixed upstream fails the gate and forces a
review instead of changing behavior quietly.

**Derive cl-19 honestly.** §A.19's header says the table lists only the *non-ideographic*
members, so the ideographs come from the UCD. `Unified_Ideograph=Yes` is the base: it
deliberately excludes `U+3005` and `U+303B`, which JLReq puts in cl-09, which is exactly
right. `Ideographic=Yes` over-covers (Tangut, Nushu, Khitan). `Script=Han` over-covers
differently (it includes `U+3005`). The treatment of the Compatibility Ideographs is a
decision, not a derivation, and is recorded in `docs/decisions/` with
[`Standing::Unstated`]. `U+4EDD` 仝 is listed explicitly in cl-19 *and* is an ideograph, so
"listed" and "is an ideograph" are not disjoint and the union must not assume they are.

**Fold, but not with NFKC.** Real text contains `U+FF08`; Appendix A keys `U+0028`. Only
the `Decomposition_Type=Wide` and `Narrow` mappings are used. Full compatibility folding
would fold `U+2160` Ⅰ, a genuine cl-19 member, onto `I`.

**Vendor the UCD.** The extracts are committed, so regeneration needs no network and the
byte-identity check can sit in `ci-required` as a hard `git diff --exit-code` rather than a
job that can fail for connectivity.

## What the captured stage must get right

### Double entry

W3C publishes each matrix twice — `tables/table_enN.pdf` and `tables/table_jaN.pdf` — as
independently typeset documents with the same content. Each is transcribed separately and
`attest` requires them to agree cell for cell. A slip must occur twice, identically, in two
readings of two documents to survive.

Independence is procedural and the gate says so rather than pretending otherwise. Each file
carries a `[capture]` block naming the author and date, `attest` fails when the two files
of a pair share an author or have byte-identical row ordering, and `attest`'s report states
plainly that double entry is a procedural control and the invariants below are the
mechanical one. Presenting a procedure as a gate would be the same species of overclaim as
calling a PDF scrape "generated".

### Provenance per cell

Every row records the source file, the table number, the row label and the column label,
and the legend token verbatim. A cell without provenance fails the build. Because the axis
types differ per appendix — Table 1 and Tables 3 through 5 are 31 × 31 with a `line-head`
row and a `line-end` column, Table 2 and Table 6 are 30 × 30 with no line-edge axes at all
— a cell whose labels do not exist in that appendix has no provenance to record and is
rejected.

The two line-edge labels are written `line-head` and `line-end`, hyphenated, which is how
the address space spells them and therefore how they are spelled in `rules.tsv`, in a
`JLReq:` line and in a case file. The published matrices print them as prose labels and
give them no identifier, so one spelling is chosen once, here, and
[ADR 0013](../adr/0013-rules-are-addressed-by-specification-address.md)'s claim that a rule
has one spelling in all four artifacts holds with nothing left to translate. A uniform 33 × 33 shape would have required roughly 890 fabricated
provenance entries across the six matrices.

```text
# spec/captured/table1.en.tsv
source	table	before	after	token	note
table_en2.pdf	1	cl-05	cl-05	1/4 be + 1/4 af	B.2#3
table_en2.pdf	1	cl-02	line-end	1/2 be	B.2#2
```

### Cross-table invariants

Agreement between two transcriptions catches a slip. A systematic error survives it, so the
capture must additionally satisfy invariants derived from prose that *is* machine-readable.
Each cites the sentence justifying it, and each is also a conformance case, so the
redundancy is published rather than private.

1. A `×` at a coordinate is a `×` at that coordinate in every table that has it — the
   legend defines it identically in each. (§B.1, §C.1, §D.1, §E.1)
2. A blank in Table 6 faces a "not" in Table 2: §E.1's legend defines the blank as
   "expansion is not allowed because there is no line break opportunity". (§E.1)
3. Tables 3, 4 and 5's non-reducible amounts equal Table 1's: "the default unadjusted
   spacing shall be determined according to §B". (§D.1)
4. No reduction opportunity in the `line-head` row of any of Tables 3, 4 and 5. (§D.1)
5. Table 4 additionally has none in the `line-end` column. (§D.1)
6. Table 2 and Table 6 have no line-edge axes at all. (§C.1, §E.1)
7. Row cl-01 of Table 2 is "not" throughout, and columns cl-02, cl-06 and cl-07 are "not"
   throughout: §C.3's preamble says these are "prohibited at all levels".
8. §3.1.7's ten line-start-prohibited classes are exactly the Table 2 columns that are
   "not" throughout at the Very strict level. (§3.1.7, §C.3)
9. §3.1.8's two line-end-prohibited classes are the corresponding rows. (§3.1.8)
10. The five-rule punctuation pattern §3.2.4, §3.2.5 and §3.2.6 state verbatim three times
    holds in Table 1's cl-19, cl-30 and cl-27 rows and columns, with the single documented
    variation against cl-15, cl-16 and cl-19.
11. A `hang` token sits only on a cell whose amount is `1/2` or `1/4`; `ruby hang` sits
    only on a solid cell. (§B.1)
12. cl-28 and cl-29 match cl-01 and cl-02 except where §3.9.2 and §3.1.10 state a
    difference.
13. Table 4's line-end column reflects §3.1.9's JIS reading — half em after cl-06, solid
    after cl-02, cl-07 and cl-05 — which is stated in prose and captured independently as
    cells, so the two must agree.
14. §3.8.3's six-step prose order equals Table 3's stage ordinals, and §3.8.4's four-step
    order equals Table 6's. (§3.8.3, §3.8.4)
15. The priority ordinals read from cell color agree with the ordinals the §D.2 notes state
    in words. This is where the specification contradicts itself: §D.2 note 5 says the
    middle-dot conditional space is third in Table 3 while notes 1, 2 and 3 all say fourth.
    The invariant does not silence it — the rule is recorded with
    [`Standing::Adjudicated`] and both readings appear in a conformance case.
16. Every cell any conformance case exercises agrees with that case.
17. Every amount in every table, note and ladder is an exact multiple of 1/720 em. 720 was
    chosen for exactly that property ([ADR 0007](../adr/0007-two-scalars-and-the-fixed-point-unit.md)),
    and nothing checked it: an appendix note naming a thirty-second would otherwise have rounded
    quietly at the one point in the design that is a permanently breaking change to revisit.
18. No boundary yields more than one conditional space per referent, so the "at most two"
    of [ADR 0014](../adr/0014-the-conditional-space-is-the-unit-of-spacing.md) is checked
    against the capture rather than asserted in prose. §B.2 notes 3 and 5 are the only cells
    that produce two, and both produce one per side; a transcription that read a three-term
    sum out of the legend fails here rather than losing a term at the far end.

Invariant 2 is the strongest, because §E.1's legend explains a Table 6 value by reference to
Table 2 — the specification itself asserts a redundancy across two documents captured
separately, so checking it is close to a proof of correct capture for those cells.

## Recorded defects

These are data in `defects.tsv`, each with a conformance case:

| Defect | Where |
| --- | --- |
| `U+216B` appears twice in the cl-19 body (465 rows, 464 members) | §A.19 |
| Three cl-25 Remarks cells hold bare Japanese with no locale span | §A.25 |
| §D.2 note 5 contradicts notes 1–3 on a priority ordinal | §D.2 |
| §3.8.3 step 1 says "the same width reduction is applied to all spaces" in English against 文字サイズ比で均等に, in proportion to character size, in Japanese | §3.8.3 |
| §B.2 note 11 says "simple-ruby" where jukugo-ruby is meant | §B.2#11 |
| §B.2 note 7's English names only katakana; the Japanese names cl-16, cl-10 and cl-11 | §B.2#7 |
| §3.1.3's closing Note reads "vertical" in English against 横組 in Japanese | §3.1.3 |
| §3.8.3 numbers Appendix D's tables one higher than Appendix D does | §3.8.3, §D |
| The legend anchor ids and the PDF filenames are off by one from the table numbers | §B–§E |
| §3.9.2 lists cl-28 and cl-29 as "（〔［ etc." while §A.28 and §A.29 enumerate exactly three | §3.9.2 |

Where a defect changes an answer, it is a `Question` as well as a defect, so the reading is
the caller's rather than ours.

## The gates

```sh
cargo run -p xtask -- generate          # regenerate crates/*/src/generated/*.rs
cargo run -p xtask -- generate --check  # fail if regeneration would change a file
cargo run -p xtask -- attest            # double entry, provenance, invariants, defects
cargo run -p xtask -- attest --digests  # additionally verify spec/upstream/ if present
```

`generate --check` is byte identity, not semantic equivalence: `data/manifest.toml` holds
the SHA-256 of every generated file and of every input it was generated from, and `xtask`
computes SHA-256 with `std` alone in about a hundred lines, preserving its empty dependency
table. A hand-edited generated file fails immediately. A regeneration job also runs the real
stage-1 generator from the vendored snapshot and `git diff --exit-code`s the result, which
is offline and therefore fits in `ci-required`.

`attest` needs no network and no PDF: double entry and the invariants run over the committed
transcriptions. `--digests` additionally verifies any PDFs a developer placed in the
gitignored `spec/upstream/` against `PROVENANCE.toml`. `xtask` never fetches, because CI
must not depend on w3.org being reachable.

## The generated Rust must pass the same gates as hand-written Rust

The emitter writes `#[must_use]`, a doc comment on every public item including every enum
variant, a `JLReq:` citation line, underscore-separated numeric literals, no `as` casts, and
LF line endings — `rustfmt.toml` sets `newline_style = "Unix"` and a CRLF file is a hard
failure on Windows. Tables are `static` arrays and never functions, because
`clippy::too_many_lines` has a threshold of 100.

## Licensing and REUSE

`REUSE.toml` gains two annotation blocks:

```toml
[[annotations]]
path = ["spec/snapshot/index.html", "spec/derived/**", "spec/captured/**"]
precedence = "aggregate"
SPDX-FileCopyrightText = "2011-2020 W3C (MIT, ERCIM, Keio, Beihang)"
SPDX-License-Identifier = "W3C-20150513"

[[annotations]]
path = ["spec/snapshot/ucd/**"]
precedence = "aggregate"
SPDX-FileCopyrightText = "1991-2024 Unicode, Inc."
SPDX-License-Identifier = "Unicode-3.0"
```

Both licenses need a `LICENSES/*.txt` file, and `reuse lint` fails on an unused one, so
they are added in the same commit as the first snapshot file. `Unicode-3.0` is already on
`deny.toml`'s allow list; `W3C-20150513` applies to committed documents rather than to a
crate in the dependency graph, so `cargo deny` never sees it.

Two further traps, both measured. `reuse lint` flags *untracked* files, so
`tools/jlreq-gen/target/` must enter `.gitignore` in the same commit as the tool or every
build artifact becomes a violation. And `taplo.toml` includes `**/*.toml`, so
`spec/PROVENANCE.toml`, `data/manifest.toml` and `docs/decisions/*.toml` are format-checked
even though they are outside every cargo workspace — as is `tools/jlreq-gen/Cargo.toml`.

A nested workspace needs five coordinated edits or a gate breaks: the directory in the root
`exclude`, its own `[workspace]` table, its `target/` in `.gitignore`, its TOML reachable by
taplo, and SPDX headers on its `.rs` and `.toml` files. `fuzz/` is the existing worked
example.

Everything outside the cargo workspace escapes clippy, `cargo-deny`, `cargo-msrv` and
`cargo-fmt`, all of which are workspace-scoped, while remaining covered by `typos`, `reuse`
and `taplo`. The generator is therefore unlinted but not unchecked, and that is stated here
so nobody discovers it as a surprise.
