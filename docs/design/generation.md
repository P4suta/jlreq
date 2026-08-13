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
    classes.tsv              the thirty classes of §3.9.2: id, name (both locales), section
    appendix-a.tsv           1133 keys, class, Remarks (both locales)
    ideographs.tsv           Unified_Ideograph, the members §A.19 does not list
    folding.tsv              the Wide and Narrow decompositions, with the frame each asserts
    scripts.tsv              Script=Hiragana and Script=Katakana
    anchors.tsv              rendered section number → anchor id → heading
    notes.tsv                every note of §B.2, §C.2, §D.2, §E.2, both locales
    rules.tsv                the rule inventory: address, standing, statement
    questions.tsv            the policy space: question, constant, permitting address,
                             where the permission comes from, the answers, and JLReq's own
    defects.tsv              the recorded defects: id, where, evidence, treatment
  captured/
    table1.en.tsv            transcribed from tables/table_en2.pdf
    table1.ja.tsv            transcribed from tables/table_ja2.pdf
    ...                      through table6
    figures.tsv              the arrangements published only as images (§3.4.3, §3.7.2)
    invariants.tsv           each cross-table check, with the sentence justifying it
  upstream/                  gitignored; the PDFs, if a developer fetched them
crates/*/src/generated/*.rs  committed output of stage 2
data/manifest.toml           SHA-256 of every file the pipeline reads or writes
docs/decisions/*.md          this project's published readings of silences
```

An earlier revision of this document said `jlreq-spec` would take its two generated tables
in modules of their own — the rule inventory emitted into `src/rule.rs` and the policy space
into `src/policy.rs`, beside the vocabulary that indexes each — on the grounds that a rule
address, its inventory and the identifier that reads it are one module's worth of subject.
That is not how it is built, for two reasons that only appeared once the emitter existed.
`src/rule.rs` holds the hand-written `const fn` address parser, `Standing`, `Detail` and
`Section`, so it cannot be *wholly* generated and a partly-generated file is one no
byte-identity gate can check. And `generate`'s own `check_declarations` refuses any output
outside `crates/<crate>/src/generated/<module>.rs`, which is what lets its scan for
unclaimed modules find a hand-written file hiding among machine-written ones.

So `jlreq-spec` takes the same shape every other crate does. The inventory is emitted into
`src/generated/inventory.rs` and `src/rule.rs` reads `RULES` from it; `src/generated.rs`
declares the module beside the directory and holds, by hand, the figures the inventory was
measured against. The policy space will arrive the same way. `docs/design/api-spine.md`'s
file maps state the same layout this list does.

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

**Stage 1**, `cargo run -p xtask -- derive`, reads `spec/snapshot/` and emits
`spec/derived/*.tsv` and nothing else.

**Stage 2**, `cargo run -p xtask -- generate`, emits `crates/*/src/generated/*.rs` from
`spec/derived/` and `spec/captured/`.

Both are byte-identity gates, both are dependency-free, and both run in CI on every commit.
Adding a derived file is one entry in `DERIVATIONS`; adding a generated one is one entry in
`UNITS`; each reader and each generator lives in the `xtask` module that owns its subject.

An earlier revision of this document put stage 1 in `tools/jlreq-gen`, a workspace excluded
from the root, so that it could parse HTML with a crate. That reasoning was sound and its
premise turned out not to hold: the scanner is 500 lines of `std`, in the same hand-rolled
style as the manifest reader in `purity` and the SHA-256 in `generate`, so there is no
dependency to keep out of the workspace and no transitive duplicate for `deny.toml`'s
`bans.multiple-versions = "deny"` to charge the published crates for. What the split would
still have cost is everything workspace membership buys — Clippy, `rustfmt`, `cargo-msrv`,
and decisively `cargo nextest`, which is workspace-scoped, so the tests that prove the
scanner reads the document's actual shape would never have run in CI. The scanner is
therefore in `xtask` and this document records the change rather than the intention.

A `build.rs` generator remains not merely undesirable but impossible: the purity gate
rejects a `[build-dependencies]` table of any kind.

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

**Count rows and not character cells.** Appendix A holds 1687 data rows: 1662 whose UCS cell
is a bare hex value, and 25 written as a code-point sequence, `<304B, 309A>`. Removing the one
recorded duplicate leaves the 1686 listings the generated table holds, over 1133 distinct
keys. A scan keyed on `td.character` measures 1686 *rows* and is off by one, because §A.12's
`U+2116` NUMERO SIGN is the only row in the whole appendix written `<td class="character-latn">`;
the reader counts `<tr>` inside `<tbody>` instead, and the per-class figures in `CENSUS` are
what would catch it if that ever stopped being true. An extractor that assumes one code point
per row silently drops the 25 sequences, which is what `Member` being an ordered sequence and
not a `char` exists for.

The alternation form `<0254, 0300/0301>` is read and does not occur: measured, the rendered
snapshot holds none and neither does the pre-ReSpec editorial source — both write
`<0254, 0300>` and `<0254, 0301>` as two rows. Reading it is safe rather than permissive,
because a row that yielded two keys would move a class's total and fail the census before
anything was emitted; the doc comment on that reader records the measurement rather than
asserting the form exists.

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

**Where a derived file carries a reading, it says so.** `rules.tsv`'s
`direction_conditional` column and the whole membership of `questions.tsv` are judgments
about the document rather than properties a scanner computes, and the arrangement is the
same in both places: the reading is a `const` table in the `xtask` module that writes the
file, the document supplies the evidence, and the derivation refuses to emit unless the two
agree. §3.1.3, §3.2.5 and §3.3.5 are marked direction-conditional only if each one's own
text still names a writing mode; a policy question is emitted only if the sentence its row
quotes is verbatim in the section or note it addresses, in the rendering it names. So a
revision that resolves a question, or stops conditioning a rule on the writing mode, fails
the build with the row named rather than leaving a claim nothing supports.

That is why the policy space is derived rather than captured, and the distinction is worth
stating because it looks like a close call and is not. `spec/captured/` is for content a
machine cannot read — matrices published only as PDF, with the priority ordinals in cell
color — and double entry is the control that content needs. Every sentence `questions.tsv`
rests on is in the HTML snapshot and is extracted from it by `xtask/src/inventory.rs`; what
is not machine-readable is only *which* sentences constitute a question. Keying the
sentences in by hand and checking them against each other would be a weaker control than
extracting them and checking the reading against the document, and it would put prose that
is machine-readable into the directory reserved for prose that is not.

`questions.tsv` records where each permission comes from, in a column, because that is the
one thing about a policy question a reader most needs and the one thing prose most easily
loses: `stated` where the section states the alternatives in so many words, `divergent`
where the two renderings of one sentence do not state the same rule, `contradictory` where
the document states one rule twice in ways that are not equivalent, and `silent` where the
document decides nothing and the answers are this project's, published in `docs/decisions/`.
Only the first is a permission JLReq grants. Those map onto `Standing` — `Alternative`,
`Adjudicated`, `Adjudicated`, `Unstated` — so the column is what stops a silence being
laundered into a requirement on the way to the generated table.

Three further controls hold that file. `docs/design/api-spine.md` publishes one `Question`
constant per row and the `api` gate subtracts the two lists in both directions, so a
constant nobody read the specification for and a row no caller can name each fail the build.
`docs/api-frozen.toml`'s `[[closed_choices]]` states, for every one of the twenty-two
questions, how many answers its set may hold and what closes it there, and the same gate
holds the derived counts to them. And a `divergent` row is checked for still diverging: the
derivation compares the character classes the two renderings of its address cite and refuses
to publish the permission if they ever agree. No row carries that value today. §B.2 note 7
was read as one and is not: its English half states all four answers in so many words, which
makes the row `stated` — a permission JLReq grants — and makes the locale difference beside
it a defect of the document rather than the reason the alternatives exist. It is recorded
once, as `b2-note-7-locale-class-divergence` in `defects.tsv`, and this check is what a
future row recorded as a divergence would have to survive.

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
12. **Retired.** This item read "cl-28 and cl-29 match cl-01 and cl-02 except where §3.9.2
    and §3.1.10 state a difference," on the premise that the exception was sparse and
    per-cell. Measured against the landed transcription, it is neither: Table 1 alone holds
    51 agreements against 57 unnoted disagreements between the two class pairs, roughly 311
    across the six independently double-entered tables, and the shape is not scattered —
    cl-28's column reads `blank` in 20 of Table 1's 29 rows where cl-01's column reads a real
    amount in 19 of them. §3.9.2's own note on cl-28 and cl-29 states why, and states it once
    for the whole pair rather than cell by cell: "they are in a separate class since they
    differ from normal brackets with regard to their processing." (§3.1.10, checked directly
    against the rendering, says nothing about either class; the citation was in error.) A
    check that demands a per-cell appendix footnote for what the source licenses at the
    class level cannot pass on true data, so it is removed rather than weakened — the
    measurement above is the record of what replaced it. `spec/derived/notes.tsv` would not
    have changed this: the license is §3.9.2's body prose, not an appendix note. Left as an
    open, unadjudicated observation for whichever milestone next reads Table 1, 3, 4, 5 and 6
    together: twelve coordinates run the other way, where cl-28 or cl-29 carries a real
    amount and cl-01 or cl-02 reads `blank` — cl-06×cl-29 and cl-07×cl-29 in Tables 1, 3, 4
    and 5, and cl-12×cl-28, cl-29×cl-09, cl-29×cl-10 and cl-29×cl-11 in Table 6.
13. Table 4's line-end column reflects §3.1.9's JIS reading — half em after cl-06, solid
    after cl-02, cl-07 and cl-05 — which is stated in prose and captured independently as
    cells, so the two must agree.
14. §3.8.3's six-step prose order equals Table 3's stage ordinals, and §3.8.4's four-step
    order equals Table 6's. (§3.8.3, §3.8.4)
15. The priority ordinals read from cell color agree with the ordinals the §D.2 notes state
    in words, and each ordinal is matched against the §3.8.3 step it belongs to. §D.2 note 5
    gives the middle-dot conditional space the third priority in Table 3 where notes 1, 2
    and 3 give it the fourth, and those are two steps rather than two answers: §3.8.3 step 3
    is 行末に配置する中点類 — the middle dot *placed at the line end*, whose two quarter ems
    are set solid together — and step 4 is 行中の中点類, the one in the middle of a line.
    Note 5 is the first and notes 1 to 3 are the second. What is defective is one locale of
    one sentence: note 5's English half drops the position its Japanese half states, as
    §3.8.3 step 3's English half does, which is recorded as
    `d2-note-5-line-end-qualifier-omitted-in-english` and read from the half that states it.
    This invariant is therefore an agreement to check and not a contradiction to adjudicate;
    an earlier revision of this document asserted the contradiction and pre-committed the
    rule to [`Standing::Adjudicated`], which would have published an alternative JLReq does
    not permit.
16. Every cell any conformance case exercises agrees with that case. A boundary case may
    declare which coordinates it exercises through `cells`, a case-level, optional,
    list-valued field of `{table, before, after}` objects — spelled the way
    `spec/captured/table<N>.<locale>.tsv` and this file's own `MATRIX_COLUMNS` key a cell,
    not through the `address` grammar's `@` suffix, which cannot name one cell where a
    legend section covers several tables (§D.1 is the legend of Tables 3, 4 and 5 at once).
    The checker asserts existence, at every table a case names, and — for Table 1 alone —
    that a case's default-policy (`policy: {}`) boundary answer agrees in units with the
    captured cell. 21 of the suite's 72 boundary cases declare a coordinate this way today
    (`{B,B.2,D.1,D.2,E,E.2}.json`, 43 coordinates in all); the remaining 51 — every `A.*` and
    `3.x` boundary case, and `C.json`'s and `C.2.json`'s own Table 2 coordinates, which carry
    no amount to compare — are the invariant's own named remainder, because turning a case's
    `text` into the class pair a cell is keyed on would re-derive Appendix A's classification
    inside a gate that has no dependencies, a second implementation of a fact `jlreq-class`
    owns (ADR 0019).
17. Every amount in every table, note and ladder is an exact multiple of 1/720 em. 720 was
    chosen for exactly that property ([ADR 0007](../adr/0007-two-scalars-and-the-fixed-point-unit.md)),
    and nothing checked it: an appendix note naming a thirty-second would otherwise have rounded
    quietly at the one point in the design that is a permanently breaking change to revisit.
18. No boundary yields more than one conditional space per referent, so the "at most two"
    of [ADR 0014](../adr/0014-the-conditional-space-is-the-unit-of-spacing.md) is checked
    against the capture rather than asserted in prose. §B.2 notes 3 and 5 are the only cells
    that produce two, and both produce one per side; a transcription that read a three-term
    sum out of the legend fails here rather than losing a term at the far end.
19. No Table 6 coordinate that offers a real expansion opportunity is also a Table 1
    coordinate that carries two conditional spaces, so [ADR 0021](../adr/0021-table-6s-expansion-belongs-to-the-boundary.md)'s
    amendment — Table 6's opportunity is read once per boundary, independent of how many
    terms Table 1 gives that same coordinate — never has to choose which of two terms an
    independent expansion belongs to. Measured over the whole capture: of Table 6's 494
    non-blank, non-`×` cells, zero sit at a Table 1 coordinate with two terms; a future
    revision of either table that broke that fact fails here rather than leaving
    `jlreq_line::ladder::Site` asked to carry more expansion room than ADR-0014's own
    at-most-two-spaces bound gives it anywhere to put.

Invariant 2 is the strongest, because §E.1's legend explains a Table 6 value by reference to
Table 2 — the specification itself asserts a redundancy across two documents captured
separately, so checking it is close to a proof of correct capture for those cells.

## Recorded defects

These are data in `spec/derived/defects.tsv`, written by `xtask/src/defects.rs` in stage 1.
An earlier revision of this document put the file on the captured side, on the reasoning that
most of its rows are defects of the matrices and the appendix notes. Measured, none of them
is: every one of the twelve is a property of `spec/snapshot/index.html`, which is the
machine-readable half of the split [ADR 0009](../adr/0009-generated-data-and-attested-transcription.md)
draws, and not one is a property of a PDF matrix. So the file is derived, `derive`'s scan for
stray files requires a derivation to claim it, and it is regenerated from the snapshot like
every other table under `spec/derived/`.

Being derived is a claim with teeth. Twelve sentences in a constant, printed into a file,
would be an attestation wearing a derivation's header: `derive --check` would prove only that
the constant had not changed. So **every defect carries a detector** — a measurement over the
rendering — and the row's `evidence` is composed from what that detector measured, down to the
line numbers. A detector that no longer finds its defect fails the derivation and prints the
review procedure. That is this document's "a defect fixed upstream fails the gate and forces a
review", enforced rather than asserted, and `attest` additionally holds the file's identifiers
against `RECORDED_DEFECTS` in `xtask/src/attest.rs` — two lists rather than one shared
constant, because the gate that checks the file is not the program that writes it.

Every one of the twelve is detected; none is attested. Three of the readings look like human
judgment and turn out to have exact predicates, each firing on exactly the passage recorded:
"vertical" against 横組 is the only paragraph pair in the rendering where one half names a
writing mode, the other names the other, and neither names both; "simple-ruby" is the only
English half naming that complex where its Japanese half never names 熟語ルビ以外; and the
uniform-against-proportional reduction is one of exactly two pairs whose Japanese half says
文字サイズ比 and whose English half never says "character size" — the other twenty-five agree,
and both exceptions are step 1 of a §3.8.3 priority list, so the record covers both.

The file's fourth column, `treatment`, is this repository's sentence about what it does with
the defect rather than a property of the text — the same split `rules.tsv` makes between the
statement it quotes and the `direction_conditional` reading beside it. It states what the
pipeline does today, so a defect nothing has met yet says that rather than promising a
milestone.

The closed Remarks vocabulary in `xtask/src/classes.rs` still holds the derived stage
independently: every distinct cell of Appendix A is enumerated there with the frame,
direction and role it states, with the count of cells holding it, and with the defect it is an
instance of where it is one. A cell nobody has read fails the derivation, and a recorded cell
whose count has moved fails it too. Three of the twelve are therefore measured twice, by two
readers that share no code.

| Defect | Where |
| --- | --- |
| `U+216B` appears twice in the cl-19 body (465 rows, 464 members) | §A.19 |
| Three cl-25 Remarks cells hold bare Japanese with no locale span | §A.25 |
| Three Remarks cells state the digit-grouping role in Japanese alone — 位取りの空白, 位取りのコンマ — where the English half gives the width only. §A.24's `U+002E` does *not* diverge: both halves carry the decimal-point line | §A.24, §A.25 |
| §D.2 note 5's English half drops the 行末に配置する — placed at the line end — that its Japanese half states, so an English-only reader meets a second priority ordinal for what looks like one reduction. §3.8.3 lists the line-end reduction and the mid-line one as separate steps, and its step 3's English half makes the same omission | §D.2#5 |
| §3.8.3 step 1 of each of its two priority lists says "the same width reduction is applied to all spaces" and "reduced by equal amounts" in English against 文字サイズ比で均等に, in proportion to character size, in Japanese | §3.8.3 |
| §B.2 note 11 says "simple-ruby" where jukugo-ruby is meant | §B.2#11 |
| §B.2 note 7's English names only katakana; the Japanese names cl-16, cl-10 and cl-11 | §B.2#7 |
| §3.1.3's closing Note reads "vertical" in English against 横組 in Japanese | §3.1.3 |
| §3.1.6's fourth Note leaves a cross-reference unresolved in English, as the literal `[[[#spacing_between_characters"]]]`, where the Japanese resolves it to §B | §3.1.6 |
| §3.8.3 numbers Appendix D's tables one higher than Appendix D does | §3.8.3, §D |
| The legend anchor ids and the PDF filenames are off by one from the table numbers | §B–§E |
| §3.9.2 lists cl-28 and cl-29 as "（〔［ etc." while §A.28 and §A.29 enumerate exactly three | §3.9.2 |

Two of those rows correct an earlier revision of this table against the document. §3.1.6
publishes four Notes and the unresolved reference is in the fourth, not the second. And the
divergence at §3.8.3 step 1 is not one paragraph but two: the section states two priority
lists, its own and JIS X 4051's, and step 1 of each says in English what its Japanese half
does not.

Where a defect changes an answer, it is a `Question` as well as a defect, so the reading is
the caller's rather than ours.

## The gates

```sh
cargo run -p xtask -- derive            # reread spec/snapshot/ into spec/derived/*.tsv
cargo run -p xtask -- derive --check    # fail if rereading would change a derived file
cargo run -p xtask -- generate          # regenerate crates/*/src/generated/*.rs
cargo run -p xtask -- generate --check  # fail if regeneration would change a file
cargo run -p xtask -- attest --digests  # double entry, provenance, invariants, defects
```

`generate --check` is byte identity, not semantic equivalence: `data/manifest.toml` holds
the SHA-256 of every generated file and of every input it was generated from, and `xtask`
computes SHA-256 with `std` alone in about a hundred lines, preserving its empty dependency
table. A hand-edited generated file fails immediately. `derive --check` is byte identity in
the same sense one step earlier, and each derived file carries in its own comment header the
SHA-256 of every vendored source it was read from, so the chain from the published document
through the tab-separated tables to the emitted Rust is digest-linked at every step. Both
gates are offline and therefore sit in `ci-required`, which the CI workflow satisfies by
running the `design` recipe rather than by listing the gates a second time — the two lists
drifted once, and `derive --check`, the only gate that binds `spec/derived/` to the vendored
document, was the one that fell through the gap.

Three things the chain would otherwise not cover, and each is a link that was missing:

**Its far end.** Every derived file states the digest of the snapshot *as it sits on disk*,
so `derive --check` rederives whatever is there and agrees with itself. What anchors the
loop to the recorded upstream digest is `attest --digests`, and that is now what `just
attest` runs: the weaker form has no reason to be the default, because the implementation
hashes every recorded document present on disk, which is more than the sentence about
`spec/upstream/` claims. Its own end is closed too — `derive` requires every declared source
to be a document `PROVENANCE.toml` records, and requires every file vendored under
`spec/snapshot/` to be one, so the input directory is closed the way both output directories
already were.

**Its near end.** The generator is not the data. An editorial judgment lives in the reading
and in the emitter rather than in the document — which Remarks cell carries which frame,
which heading closes a class name — so a change to one rewrites the meaning of thousands of
rows with the source digest, the specification date and the entry count all unchanged. Every
derived file therefore states `Reader:` and `Reader SHA-256:`, every generated file states
`Generator:` and `Generator SHA-256:`, and `data/manifest.toml` records a digest for each of
those modules. This replaces a `Generator: xtask 0.0.0` line taken from the shared workspace
version, which moved on a release and never on a change to a generator: churn where
information was wanted, and the one recorded identifier that could not distinguish two
generators.

**Its claim about itself.** `specification-date` was a free string copied into the header of
every generated file and checked against nothing. The published rendering carries its own
publication date in a `<time class="dt-published">` element, so `derive` now requires the
recorded date to equal it, and to equal the `published` field of the snapshot's own
`[[document]]` block.

`data/manifest.toml` accordingly records every file the pipeline reads or writes: the six
generated modules, every derived table — including those no generation unit consumes yet,
which `spec-links`, `attest`, `api` and `conform` nonetheless read — the four vendored
documents, and every `xtask` module that produces one. The list is not written out here
because it is not maintained here: `generate` builds it from `DERIVATIONS` and `UNITS`, so a
derivation added without a manifest entry is a failure rather than a documentation lapse. A
ledger that records part of a chain records nothing about the rest of it.

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
SPDX-FileCopyrightText = "2020 W3C (MIT, ERCIM, Keio, Beihang)"
SPDX-License-Identifier = "W3C-20150513"

[[annotations]]
path = ["spec/snapshot/ucd/**"]
precedence = "aggregate"
SPDX-FileCopyrightText = "2025 Unicode, Inc."
SPDX-License-Identifier = "Unicode-3.0"
```

Each copyright line is the notice the vendored file itself prints, and nothing wider.
The snapshot's footer reads "Copyright © 2020 W3C (MIT, ERCIM, Keio, Beihang)" and
names no earlier year — the only prior edition it links is the 2012 Note — and the
Unicode Character Database extracts read "© 2025 Unicode, Inc.", the year of the
17.0.0 release `spec/PROVENANCE.toml` pins. An earlier revision of this document gave
both as ranges, `2011-2020` and `1991-2024`, which were written before either file was
measured and which reproduce no notice that exists. Reproducing the notice on the work
is what [REUSE](https://reuse.software) asks for, and a range nobody can point at in the
file is the same species of unfounded claim as a scraped matrix.

Both licenses need a `LICENSES/*.txt` file, and `reuse lint` fails on an unused one, so
they are added in the same commit as the first snapshot file. `Unicode-3.0` is already on
`deny.toml`'s allow list; `W3C-20150513` applies to committed documents rather than to a
crate in the dependency graph, so `cargo deny` never sees it.

Three files under `spec/derived/` are read out of the Unicode Character Database rather
than out of the W3C document, so they carry Unicode's notice and not W3C's. `REUSE.toml`
names them one by one with `precedence = "override"`, ahead of the `spec/derived/**` block,
because a glob that covered both would attribute the ideograph predicate to W3C.

`reuse lint` flags *untracked* files, so a new derived or generated file must be `git add`ed
in the same change that writes it. And `taplo.toml` includes `**/*.toml`, so
`spec/PROVENANCE.toml` and `data/manifest.toml` are format-checked even though they are
outside every cargo workspace. `docs/decisions/` is Markdown rather than TOML, for the reason
its own README states: a published reading is an argument from the specification's own words,
nothing reads one mechanically, and TOML would either flatten the argument into one quoted
string or invent a record format for paragraphs.

Everything outside the cargo workspace would escape clippy, `cargo-deny`, `cargo-msrv`,
`cargo-fmt` and `cargo-nextest`, all of which are workspace-scoped, while remaining covered
by `typos`, `reuse` and `taplo`. That is the reason both stages of the pipeline are `xtask`
subcommands: the readers and the emitters are held to exactly the gates the code they write
is held to. A nested workspace would need five coordinated edits or a gate breaks — the
directory in the root `exclude`, its own `[workspace]` table, its `target/` in `.gitignore`,
its TOML reachable by taplo, and SPDX headers on its `.rs` and `.toml` files — and `fuzz/`
is the one place this repository pays that price, because `cargo-fuzz` requires nightly.
