# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-08-15

### Changed

- Updated security support, issue forms, mutation reporting, current decision ownership,
  and CI wording to describe the 1.0 repository rather than its retired crate graph.
- Replaced the pre-1.0 multi-crate facade with the dependency-free `no_std + alloc`
  `kumihan` library and its single validated paragraph composition pipeline.
- Added all nine inline constructs, horizontal and vertical placement, optimal paragraph
  breaking, integrated tabs, diagnostics, and all 22 typed JLReq 2020 Style choices.
- Added the binary-only `kumihan-conformance` CLI, versioned NDJSON protocol, JSON Schema,
  sample engine, and 88 black-box cases covering all 100 observable inventoried rules.
- Removed the eight unpublished legacy crates and their compatibility-only controls.

### Added

- Restored the §3.3.6 single-character `flush` group-ruby reading as an explicit
  protocol-v1 black-box case, bringing the built-in suite to 89 cases.
- Added a repository gate for broken local Markdown links, publishable-package checks in CI,
  crate-specific package READMEs, and ADR 0022 for the unified 1.0 product boundary.
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
- `spec/derived/questions.tsv`, the policy space as data: the twenty-one places JLReq permits
  more than one answer, each with the address that permits it, the sentence it rests on
  quoted from the rendering it names, the answers, and the one `Policy::JLREQ` selects. A
  `permission` column records *why* an alternative is permitted — fourteen `stated`, six
  `silent`, one `contradictory` — because that is the distinction ADR 0009 exists for and the
  one prose loses first: only `stated` is a permission JLReq grants, and the column is what
  stops the others being laundered into it. The reading is a table in `xtask/src/policy.rs`
  and the derivation refuses to emit a row whose quoted sentence is not verbatim in the
  section or note it addresses, so a revision that resolves a question fails the build with
  the row named. `docs/api-frozen.toml` states the size of every one of the twenty-one answer
  sets and the `api` gate holds the derived counts and the published `Question` constants to
  it in both directions. Stage 2, which turns the file into `jlreq_spec::QUESTIONS`, is still
  to come: `Question::ALL` remains empty.
- `spec/derived/defects.tsv`, the twelve recorded defects of the published document, each
  with the measurement that must still find it. Being derived rather than transcribed is a
  claim with teeth: twelve sentences in a constant printed into a file would be an
  attestation wearing a derivation's header, so every defect carries a detector over the
  rendering, the row's `evidence` is composed from what that detector measured down to the
  line numbers, and a defect fixed upstream fails `derive` and prints the review procedure.
  `attest` holds the file's identifiers against its own list — two lists, because the gate
  that checks the file is not the program that writes it.
- The Appendix A conformance cases: 30 files, 391 cases, 27 inventoried rules. Every case is
  a published artefact rather than an internal test (ADR 0006), so each names the
  specification address it turns on, states its input as an occurrence with a declared frame
  and role, and records both readings under `permitted` wherever JLReq decides nothing.
  `jlreq-conform` gains the reader, the `Compose` trait, `run`, `run_file` and `Report`, and
  a `Kumihan` implementation that answers the classification question and reports the other
  two as not attempted — which is the non-obligation ADR 0006 is built on, measured rather
  than described.
- `docs/conformance-deferrals.toml`, the coverage ledger, guarded by `CODEOWNERS`. The rule
  inventory is generated whole and the suite is written milestone by milestone, so
  "every rule has a case" has a remainder that is nothing but the schedule. An inventoried
  rule is now in exactly one of three states — covered, deferred to a named milestone with a
  reason, or uncovered, which fails — and `conform` prints the census on every run: 27
  covered, 79 deferred (M1 37, M2 16, M3 1, M4 23, M5 2), 0 neither. A `[[deferred]]` entry
  expires by itself, because the moment a case covers the rule the entry is a violation; and
  an `[[owned]]` entry is held to the opposite invariant, so a case cannot credit a rule to
  nobody. `spec-links` subtracts the same file, which is the same debt seen from the citation
  side.
- `docs/decisions/grouped-numeral-qualification.md`, the fourth published reading: whether
  the width or the job §A.24's Remarks cell names is what reaches cl-24. The cell states
  both, §3.9.2 scopes the class by the job alone, and an occurrence with the width and not
  the job — a quarter-em comma between two hiragana — is described by neither and excluded by
  neither.
- The `jlreq` facade re-exports the three layers that exist, so a caller depends on one crate
  and names one path for a type wherever it lives.
- Appendices B through E's six matrices — Table 1 (spacing), Table 2 (line-breaking), Tables
  3 through 5 (reduction priority: JLReq's own, JIS X 4051's, and book practice's) and Table 6
  (expansion) — transcribed independently from the English and Japanese PDF renderings into
  `spec/captured/table1.en.tsv` through `table6.ja.tsv`, the one CAPTURED (attested) category
  ADR 0009 carves out for data W3C publishes only as PDF. `xtask attest` cross-checks the two
  locales cell for cell, requires every cell's provenance (source PDF, table number, row and
  column label, legend token), and holds the transcription against the cross-table invariants
  `docs/design/generation.md` derives from prose that *is* machine-readable: 4,932 cells
  double-entered across the six tables, 841 of 961 in Table 1, 3, 4 and 5's 31 × 31 grid and
  784 of 900 in Table 2 and 6's 30 × 30. One invariant retired on measurement rather than kept
  unchecked: cl-28 and cl-29 were assumed to track cl-01 and cl-02 except for scattered
  per-cell exceptions, and the landed data holds roughly 311 unnoted disagreements across the
  six tables — a class-level license §3.9.2's own prose states once for the pair, not a
  per-cell footnote, so the invariant is removed and the measurement is recorded in its place.
- Stage 2 of the policy-space derivation. `spec/derived/questions.tsv` now carries, for each
  of twenty-two places JLReq permits more than one answer — one more than at M0:
  `spacing.line_end_full_stop_comma`, §B.2 note 6's own preferred/JIS split for full stops and
  commas at the line end, distinct from §B.2 note 2's closing-bracket question beside it —
  every answer's own sentence and citing rule, whether JLReq calls one preferred, the answer
  each of the five presets selects, and the exclusions between answers. `xtask generate` turns
  the file into `crates/jlreq-spec/src/generated/policy.rs`, closing `jlreq_spec::QUESTIONS`
  and `Question::ALL`, both empty since M0. `Policy::BOOK`, `MAGAZINE`, `NEWSPAPER` and
  `JIS_READING` are no longer four names for one empty answer set; each now diverges from
  `Policy::JLREQ` at exactly its documented questions and nowhere else.
- `jlreq-spacing`, the mojikumi (文字組み) evaluator ADR 0014 specifies:
  `ConditionalSpace`, `Boundary` and `evaluate::boundary`, which answer one adjacency of two
  character classes against everything Table 1, Table 2 and Appendix D/E's reduction and
  expansion ladders state about it. The atom is the conditional space per referent (`be`/`af`)
  and not the table cell, so a note like §B.2#3's middle-dot pair — two quarter-em
  contributions from two different characters' ems, at two different reduction priorities in
  Appendix D — is two `ConditionalSpace` values on one `Boundary` rather than one number.
  §3.1.3's vertical-writing withdrawal of the conditional space around an ideographic comma
  used as a digit separator and a katakana middle dot used as a decimal point is the crate's
  one direction-conditional site, registered in `docs/direction-sites.toml`. §3.7.4's
  math-formula spacing (cl-17, cl-18) is out of scope: neither class appears in any of the six
  matrices by the specification's own axis, so the crate answers "no table constrains this"
  rather than the quarter-em §3.7.4 states in prose. Kinsoku relaxation and line breaking
  proper stay `jlreq-line`'s, the next milestone.
- `jlreq-class` applies §C.2's three reclassification notes, dormant since M0-b published
  `RECLASSIFICATIONS` empty pending the policy space: note 1 moves `々` alone into cl-19 under
  `kinsoku.iteration_mark_at_line_head = permitted`; note 2 moves every prolonged sound mark
  (cl-10) into katakana (cl-16), and note 3 moves every small kana (cl-11) into hiragana or
  katakana by its own Unicode script, both under `kinsoku.relaxation_mechanism = reclassify` —
  `Policy::JLREQ`'s own default for both. `Subject::ClassInScript` is the new variant note 3
  needed: one subject class with two destinations picked by the member's own script, which no
  existing `Subject` shape could state.
- The mutation-testing gate, baseline only: `.github/workflows/mutants.yml` runs
  `cargo-mutants` weekly and on demand over the four crates with logic to mutate today
  (`jlreq-unit`, `jlreq-spec`, `jlreq-class`, `jlreq-spacing`), and `just mutants` runs the
  same thing locally. Neither `check` nor `ci` runs it yet: it is a report, not a
  kill-everything threshold, until the next milestone's independently-authored cases give
  kinsoku and line adjustment the discipline classification already has.
- `jlreq-line` fills §C.2 notes 6 through 8 and 13's same-run break refusal:
  `feasible::same_run_refusal` reads a caller-declared `jlreq_unit::Runs` overlay directly,
  refusing a break inside one ornamented complex (cl-21), one simple-ruby complex (cl-22),
  one tate-chu-yoko run (cl-30), and one jukugo-ruby base-and-ruby group (cl-23, at the
  level `jlreq_unit::Construct::group` carries below the run), and permitting one between
  two different runs or two different groups. An occurrence with no declared group is this
  pass's own adjudication — permitted, absent positive evidence of shared indivisibility —
  recorded as a published reading in `docs/decisions/jukugo-ruby-unset-group.md`. Scope
  limit: reachable today only through the public `Feasible::compute`, called directly with
  a real overlay; `crate::compose::compose` still composes plain text, passing
  `Runs::none()` unconditionally.
- `jlreq_spacing::Boundary::expansion_rule() -> Option<RuleId>`, the citation Table 6's own
  row states for a boundary's expansion opportunity, carried independently of
  `Boundary::expansion` because `Expansion` is a kind and not a record (ADR 0010): `None`
  when no Table 6 row exists at this coordinate, `Some` when one does — including when what
  the row states is `Expansion::None`, a note's own denial of an opportunity rather than the
  table's silence about the coordinate. `rules_fired` reports it too, in a new sixth slot;
  an earlier revision of that function advanced its running index past every write except
  the delegation's, which the new slot would have silently overwritten at a boundary
  carrying both a delegation and two conditional spaces — the fix and its own regression
  test (`rules_fired_reports_two_spaces_a_delegation_and_an_expansion_without_clobbering_
  any_of_them`) land together. `crates/jlreq-conform`'s `CaseExpansion` and `ExpectExpansion`
  carry the identical citation as `rule: Option<String>`, and `check_expansion` compares it
  under its own semantics: silent when the expectation states no `rule`, passed over — never
  failed — when the expectation states one and the answer publishes none, and a real
  disagreement only when both sides publish different addresses at the same coordinate — the
  identical right `check_class`'s own doc already grants a classification answer's whole
  provenance chain, now extended to one field of a boundary answer instead.
  `docs/adr/0021-table-6s-expansion-belongs-to-the-boundary.md` records the decision as an
  amendment to its own original text rather than a new ADR, because the carrier this
  amendment gives the citation is the identical boundary-level carrier that ADR's own
  Decision already gave the amount. Two further citation surfaces stay out of this round's
  scope, and unwired for two different reasons rather than one: `ExpectBoundary.rules`
  already has an answer to compare against — `CaseBoundary.rules` is populated from
  `jlreq::rules_fired` — but `check_boundary` never reads either side's `rules` field, so
  only the comparison itself is missing there; `ExpectSpace.rule` has no answer-side value
  to compare against yet at all, because `CaseSpace` carries no `rule` field for
  `check_spaces` to read. `docs/conformance-deferrals.toml`'s `B.2#13`, `B.2#17` and `3.1.6`
  entries already name these same two holes as their blocker, unchanged by this round.
- Three `E.2.json` cases closing over §E.2 notes 8 and 9's boundary coordinates now that
  `Boundary::expansion_rule` publishes their citation:
  `E.2/grouped-numeral-percent/the-main-clause-denies-expansion` and
  `E.2/grouped-numeral-degree-celsius/the-alternative-is-scoped-to-the-percent-sign-alone`
  read the cl-24-against-cl-13 coordinate at the two fixtures
  `A.13/grouped-numeral-percent/line-break` and `A.13/grouped-numeral-degree-celsius/
  line-break` already use for the breakability question, each answering `expansion: {
  kind: "none", rule: "E.2#8" }` from Table 6's own `(24, 13)` cell; `E.2/grouped-numeral-
  then-western-character/the-alternative-is-an-unfilled-policy-slot` reads the
  cl-24-against-cl-27 coordinate and answers the identical shape citing `E.2#9`. All three
  are `standing: "normative"`, because `spec/derived/questions.tsv` addresses no question to
  either note, and all three carry a `forbidden` entry naming the ceiling a reading of the
  note's own alternative clause alone — without checking Table 6's captured cell or, for
  the percent-sign case, this workspace's own reclassification path — would wrongly publish.
  The percent-sign case's own alternative rung does not ship: drafted as a three-rung ladder
  mirroring `A.13/percent-sign/kinsoku-loose-reclassification`'s own classify-side one, it
  was cut once checking whether this workspace could produce a cl-19 reading here found two
  independent reasons it cannot on any policy — `crates/jlreq-class/src/classify.rs`'s own
  `RECLASSIFICATIONS` table carries no percent-sign entry, and independently,
  `crates/jlreq-spacing/src/evaluate.rs`'s own `class_of` resolves every item's class under
  a hardcoded `Policy::JLREQ` rather than the `policy` parameter `boundary()` itself
  receives, confirmed directly by a scratch probe (`boundary(adjacency, Policy::MAGAZINE)`
  over the fixture, run and removed before commit) that still answers `Expansion::None`
  citing `E.2#8`. `A.13/percent-sign/kinsoku-loose-reclassification`'s own second and third
  rungs publish the identical, now-verified-unreachable reading on the classify side, where
  the reference suite's own single-declared-policy run never exercises a non-default rung
  and so never caught it — this round's own cases do not repeat the claim. `crates/jlreq-
  conform/tests/suite.rs`'s `appendix_e_2` count rises from `[4 attempted, 0 not attempted]`
  to `[7 attempted, 0 not attempted]`.
- `Question::LINE_HEAD_OPENING_BRACKET` reads a policy for the first time anywhere in this
  workspace: `jlreq_spacing::evaluate::boundary`'s new `line_head_opening_bracket_space`,
  called from `spaces_of` outside the per-term loop for the identical structural reason
  `sentence_medial_dividing_mark_spaces` already is, synthesizes a half em at Table 1's `(0,
  1)` coordinate — the line head before an opening bracket, cl-01 — when the question answers
  `pattern-2`, and answers nothing under `pattern-1`, `pattern-3` or no override at all. §B.2
  note 17's own parenthetical names §3.1.5 by section title as the place its "conditional
  half em spacing" alternative is laid out ("see § 3.1.5 Positioning of Opening Brackets at
  Line Head including methods of positioning of opening brackets at the beginning of
  paragraphs"), which is what identifies the amount: Figure 71 pattern ②'s own wrapped-line-head
  half, 折返し行頭の字下げは二分アキ, is that alternative, and patterns ① and ③ are both the
  note's own preferred zero. No built-in preset answers `pattern-2` (`Policy::BOOK` answers
  `pattern-3`, every other preset `pattern-1`), so no existing test or case can regress; the
  half em is reachable only through an explicit `Policy::with` override.
  `docs/decisions/line-head-opening-bracket.md` records the three things this synthesis had to
  adjudicate rather than read verbatim — the referent (`Referent::Trailing`, the bracket being
  this boundary's only possible neighbor), the reduction (`Reduction::Rigid`, stated directly
  rather than routed through Appendix D's own reduction tables, whose `(0, 1)` row is checked
  directly against the generated data and found to be the tables' own total-29-by-29-grid
  boilerplate — the same generic citation 833 to 834 of each table's 841 rows carry — not a
  stated schedule for a term Table 1 itself never states), and the citation
  (`RuleId::POSITIONING_OF_OPENING_BRACKETS_AT_LINE_HEAD`, not `RuleId::B_2_NOTE_17`, because a
  scratch probe run and discarded before this round's own gate battery confirmed the latter
  already reaches `rules_fired` through `boundary`'s own `placement` provenance regardless of
  this synthesis, while the former had zero readers anywhere in this workspace before this
  round — checked directly, its only two occurrences were its own generated constant and its
  own row of `spec/derived/rules.tsv`). `docs/conformance-deferrals.toml`'s `3.1.5` and
  `B.2#17` entries are rewritten to state precisely what is now reachable — the wrapped line
  head's own two distinguishable answers, and both paired addresses now firing in `rules_fired`
  at that one coordinate — and what still is not: the paragraph-first-line half (改行行頭) of
  Figure 71, entirely unread by `jlreq-line`; the citation itself, still unassertable through
  `crates/jlreq-conform/src/run.rs`'s own comparison surface at either granularity —
  `check_boundary` never reads an expectation's `rules` field (a fact restated more precisely
  than before: as of round 13 `check_boundary` also compares `expansion`'s own conditional
  `rule`, which cannot stand in here because §E.1 states Table 6 carries no line-edge cells at
  all), and `check_spaces` never reads a space expectation's own `rule` field either, because
  the answer side, `CaseSpace`, carries no `rule` field to compare it against at all; and the
  same single-declared-policy limit `3.1.6`'s own entry already states for its own alternative-keyed
  entries — `Kumihan::default()` declares `Policy::JLREQ`, whose own answer here is `pattern-1`,
  so a `pattern-2`-keyed reading is a statement to an implementation that declares that
  alternative, not a coordinate `cargo nextest`'s own default run exercises. Both rules stay
  `[[deferred]]`; nothing moves to `[[owned]]` this round (ADR 0006). `crates/jlreq-line/src/
  lib.rs`'s own "Slots" section gains a third entry for the paragraph-first-line half: wiring
  it would compose correctly for patterns 1 and 2 (whose first-line indents are the ordinary
  one em plus the wrapped line head's own answer, zero and a half em respectively) but not for
  pattern 3, whose own half-em first line replaces the ordinary indent rather than adding to
  it, which `Paragraph::with_first_line_indent`'s purely additive `InlineExtent` cannot
  express — stated plainly rather than buried, since `Policy::BOOK` answers `pattern-3` and is
  this project's own default book preset.
- `ExpectBoundary::rules` is compared for the first time: `crates/jlreq-conform/src/run.rs`'s
  new `check_rules`, called from `check_boundary` whenever a case declares the field, reads it
  as a *subset* of `CaseBoundary::rules` — every address the case names must appear somewhere
  among the ones the answer published, never their equality and never their order, and a
  declared address met by an empty answered list is passed over rather than failed, the
  identical third state `check_expansion`'s own conditional `rule` field already gave one
  provenance comparison. The asymmetry is argued rather than assumed:
  `jlreq_spacing::evaluate::rules_fired`'s own fixed 6-slot array repeats the identical
  fallback address in its first two slots and orders every slot by internal layout rather
  than by anything the specification states, so holding a case to that order or to that
  repetition would be exactly the "reproduce our chain of specification addresses" demand ADR
  0006 exists to keep the suite from making of a foreign implementation. `check_class`'s own
  doc, which argues that classification provenance is *not* compared, is amended to name this
  second exception and answer its own three grounds for it directly — the first now
  discriminates *for* the boundary comparison (three `docs/conformance-deferrals.toml` entries
  name `check_boundary`'s own prior gap directly and a fourth, `D.2#4`, names the same absence
  one layer further upstream, in `rules_fired` itself, a gap this round does not close; zero
  name classification provenance), the second is answered by the
  subset semantics being materially weaker than the exact-sequence reproduction the second
  ground actually rejects, and the third by scale: the twelve pre-existing boundary-level
  `rules` declarations (five in `A.16.json`, seven in `A.22.json`) were individually
  re-verified this round before the comparison went live, against `ExpectClass::rules`'s own
  413, unaudited. All twelve are `declined` today — every one sits on a boundary where at
  least one neighbor is covered by a ruby construct `jlreq-inline` (M4) does not yet exist to
  answer, confirmed against `crates/jlreq-conform/tests/suite.rs`'s own committed census
  (`A.16`'s `[25 attempted, 1 not attempted]`, `A.22`'s `[1 attempted, 11 not attempted]`)
  rather than assumed — so this round changes nothing observable for any of them; none needed
  correcting. `crates/jlreq-conform/cases.schema.json`'s own `boundary.rules` gains the
  description it was the only field of `boundary`'s eight to be missing.
  `docs/conformance-deferrals.toml`'s `3.1.5` and `B.2#17` entries are rewritten to state what
  a case can now positively assert under the default policy — `rules: ["B.2#17"]` at cl-01's
  line-head boundary, checked on every `cargo nextest` run, since `rules_fired` puts that
  citation into its own placement slot regardless of `spacing.line_head_opening_bracket`'s own
  answer — while keeping `check_spaces`'s own unread `ExpectSpace::rule` (and `CaseSpace`'s own
  missing `rule` field) stated as still open, a published API-surface change and a round of its
  own. `B.2#13`'s entry is rewritten the identical way — its own placement citation,
  unconditionally read regardless of Table 1's empty terms at cl-26's line-head and line-end
  coordinates, is now assertable too — but `D.2#4`'s is not: that note's own citation lives
  only in a reduction table's per-term loop, which never runs where no term exists, so
  `rules_fired` never puts it in any slot at all and this round's comparison has nothing there
  to reach. Coverage stays at 67/106; no rule moves from `[[deferred]]` to `[[owned]]`.
- `crates/jlreq-conform/cases/3.1.5.json` (new) and `crates/jlreq-conform/cases/B.2.json`
  (one case appended) are the independent case phase task #42 (round 15) and task #44
  (round 16) were both forbidden from writing, ADR 0006's own discipline: derived from
  §3.1.5's and §B.2 note 17's own words and from the generated tables before this round's
  own suite run, not from what the evaluator was already known to answer. `3.1.5`'s own
  three cases pin Figure 71's own wrapped-line-head pattern and its own scope to opening
  brackets (cl-01) at a line head, neither a different class nor an interior boundary;
  `B.2/opening-bracket-at-line-head/the-preferred-zero-and-the-retained-half-em` pins the
  note's own amount. Both rules' `{}` entries assert `spaces: []` together with `rules:
  ["B.2#17"]` at the line-head boundary before cl-01 — the round's own load-bearing
  measurement, since an empty `spaces` alone is the identical answer any blank cell gives.
  Reading both locales of the note's own alternative settles the one open discriminator: the
  English's "not to remove a conditional half em spacing accompanying the characters" reads
  as retaining cl-01's own class-level half em (`spec/captured/table1.en.tsv`'s own cl-01
  column carries a trailing `1/2 af` at essentially every `before` class) rather than
  synthesizing an unrelated one, while the Japanese states only a plain amount with no verb
  of retention at all — a locale framing difference this round records rather than resolves.
  Whether the retained half em is reducible does not follow from that reading alone, and is
  where this round corrects round 15's own ground rather than its answer — though not, on a
  second pass, all the way to the categorical claim first drafted for it: Appendix D's own
  preamble scopes the whole reduction mechanism to an opportunity "between two adjacent
  characters" (`spec/derived/rules.tsv`'s row for rule `D`), but a line end has the identical
  single-neighbor structure and Appendix D genuinely does reduce real terms there (§D.1's own
  legend; `3.1.9`'s and `B.2#2`'s own cases), so "only one real neighbor" cannot itself be the
  exclusion. What actually holds, confirmed rather than assumed, is narrower and purely
  empirical: the line-head row specifically, not line edges in general, is uniformly rigid
  across Tables 3, 4 and 5 — `xtask/src/attest.rs`'s own `no_reduction_at_the_line_head`
  invariant, a `Check::Whole` run over the full transcription, reports zero violations there
  (`docs/design/generation.md`'s own cross-table invariant 4). `Reduction::Rigid` is
  consequently still the corrected answer, agreeing with round 15's own value while replacing
  the narrower ground that round's own doc gave (an absent-term row being the tables'
  total-grid boilerplate, true of the one cell but not the reason the amount cannot move). `docs/decisions/line-head-opening-bracket.md` is left
  untouched, per ADR 0006's own separation of phases; the corrected ground is recorded in the
  new cases and in `docs/conformance-deferrals.toml` instead. Both rules move from
  `[[deferred]]` to `[[owned]]`; coverage rises to 69/106, 37 deferred, 0 uncovered.
  `B.2#13`'s own entry is read again rather than moved: its note states one unified fact
  about the suppression of the cl-26 item's own supplied advance at four positions, a fact
  no crate in this workspace computes at any milestone — `jlreq-spacing` only ever produces
  an inter-character `ConditionalSpace` between two neighbors, never a change to an item's
  own advance (`cases.schema.json`'s own `item.advance`, ADR 0002) — so the entry's own
  `milestone` moves from `M2` to `M4`, the same label its warichu half already carried,
  rather than being repaired in place; no case is authored for it this round.
  `crates/jlreq-conform/tests/suite.rs` gains `section_3_1_5 => "3.1.5" [3 attempted, 0 not
  attempted]` and bumps `appendix_b_2` to `[7 attempted, 0 not attempted]`.
- `Search::Optimal { tolerance: Badness }`, M3's first slice: the whole-paragraph break
  search `docs/design/api-spine.md` froze the shape of at M1, implemented in
  `crates/jlreq-line/src/compose.rs` as a forward dynamic program (`run_dp`) over the same
  `Feasible::compute` break set and the same `geometry_of` → `Ladder::of` → `adjust_line` →
  `apply_adjustment` → `demerits_of` pipeline `Search::FirstFit` already ran, minimizing the
  paragraph's own summed `Demerits` under `Preference::compare` rather than committing to one
  candidate per line before the ladder runs. `compose`'s own greedy loop moves, unmodified,
  into a new `compose_first_fit`, and `Search::Optimal` gets its own `compose_optimal`
  alongside it — two separately written pipelines sharing only the feasibility computation,
  never a per-line evaluator, so a defect in one cannot silently reach the other's already
  verified answers (this round's own C6, checked directly: all 827 existing tests and all 466
  conformance cases produce byte-identical results before and after).
  - The DP's correctness rests on lexicographic order over `Demerits`' six `u32` components
    being translation-invariant under `Demerits::add_sat` — stated and bounded in `run_dp`'s
    own doc: every component but `badness` saturates far later than `badness`'s own
    `u32::MAX / 10_000 ≈ 429_496`-line bound, which no paragraph this milestone composes
    reaches (this round's own C2).
  - `first_line` and `is_last_line` are read from an edge's own two ends
    (`start.get() == 0`, `end.get() >= item_count`), never from where the DP's own
    reconstruction happens to be, so a line's cost is identical whichever predecessor reaches
    it (C3, covered by a dedicated first-line-indent test).
  - The scan per candidate start stops after the first ladder-drained `Overfull` result, never
    on `ExpansionExhausted` (a short line the ladder could still save by growing), with the
    stated reason recorded in `run_dp`'s own doc rather than left as a magic constant (C5).
  - `tolerance` filters which edges the DP may use, exactly "discarding any line worse than
    tolerance": given this milestone's own zero-flex `Badness::of` reading (a feasible line is
    always `Badness::ZERO`, an infeasible one always `Badness::WORST`), `tolerance` has
    exactly two reachable settings, `Badness::WORST` (neutral, matching `FirstFit`'s own
    leniency) and everything below it (admitting only feasible lines) — stated in
    `Search::Optimal`'s own doc rather than left for a caller to discover empirically.
    Tolerance exhaustion (no complete arrangement stays within it) re-minimizes once more over
    the full, un-pruned edge set rather than panicking or inventing a forbidden break
    (ADR-0010); the reading is published as `docs/decisions/tolerance-exhaustion.md`
    (`Standing::Unstated`, added to `docs/decisions/README.md`'s own table) rather than left a
    `Slots` entry, because the search itself is filled — only this one open design choice was.
  - `Line::pull_up` is populated under `Search::Optimal`: `Some` exactly when a shorter,
    evaluated alternative existed for the same line's own start and the chosen, longer line
    needed real reduction to fit it — the reduction-preferring comparison ADR-0010 describes,
    applied to two candidate breaks that both actually existed, never reverse-engineered from
    what "should" look right (`compose_optimal`'s own `pull_up_of`, covered by a direct unit
    test independent of any full composition).
  - The round's own required experiment: actively constructing a paragraph on which
    `FirstFit` and `Optimal` disagree, rather than assuming the two published claims that
    they cannot. Both are now falsified and repaired. `docs/design/api-spine.md`'s former
    "[`Preference`] reaches the same answer by comparison, which is why the two searches
    agree" held only *per line, given an identical range* (both drain the identical ladder in
    the identical order once a range is fixed) — a constructed three-ideograph paragraph
    (`least_adjustment_prefers_the_shallow_but_overfull_arrangement` /
    `even_texture_prefers_the_feasible_arrangement` in `compose.rs`'s own test module) shows
    `least-adjustment` preferring a single, violating line over `FirstFit`'s own two-line,
    fully expanded answer, because `least-adjustment` ranks `expansion_depth` ahead of
    `badness`. `ROADMAP.md`'s former "the greedy search and the optimal one cannot disagree
    about when a character hangs" is narrowed the same way and repaired with a second
    constructed pair
    (`firstfit_and_optimal_disagree_about_whether_a_trailing_full_stop_hangs`): a trailing
    full stop hangs under `Optimal` (which keeps it on one line with both ideographs, needing
    only reduction and hanging to fit) but never reaches `ladder::hang` at all under
    `FirstFit` (which puts it alone on a short, exempt last line first). Both fixtures are
    hand-verified and their numbers checked against the actual test run, not asserted from
    hand math alone.
  - `crates/jlreq-line/src/lib.rs`'s own `# Status` states why `Search::Optimal` is named in
    neither the "Wired, not slotted" nor the "Slots" list — it is new logic, not another
    crate's rule table read through, and it is filled rather than an unfilled seam — and
    restates that §3.5.4's widow threshold stays a real, named gap beside it: `Search::Optimal`
    does not read `Paragraph::with_widow_threshold`, and §3.5.4 stays `[[deferred]]` to a later
    M3 round in `docs/conformance-deferrals.toml`, unchanged by this one. Every prior claim
    that `Optimal` did not yet exist — in `compose.rs`, `objective.rs` (including
    `Badness::of`'s own stale "the second value is never reached in practice because
    `crate::Fit` classifies that line infeasible first", corrected: `crate::Fit` is never
    constructed anywhere in this crate, and `compose`'s own `demerits_of` reaches
    `Badness::WORST` on every violating line either search composes) and `lib.rs` — is repaired
    in place rather than left to mislead a reader who trusts the prose over the code.
- The conformance suite can now ask `Search::Optimal` a question at all, and one case does,
  per ADR-0006's independent-phase discipline: authored against §3.1.12's own words, not
  against what `compose_optimal` happens to produce.
  - `cases.schema.json` gains `input.search` (a `compose` case's chosen search — absent
    reads as `Search::FirstFit`, exactly what every one of the 466 prior cases already
    assumed, so none of them changed answer) and `line.pull_up` (`jlreq_line::PullUp`'s
    three fields). `pull_up` is the one field on `line` whose *absence* is a checked
    assertion — `Line::pull_up` is `None` — rather than "unchecked", on task #44 (round
    16)'s own precedent for `ExpectBoundary::rules`: the reading is safe applied
    retroactively because `Search::FirstFit`'s own doc already guarantees `None` on every
    line that search composes, so turning the comparison on changes what no pre-existing
    case is measured against. `crates/jlreq-conform/src/case.rs`, `run.rs` and
    `kumihan.rs` read and act on both fields; `xtask/src/conform.rs` gained its own
    hand-written validation of `search` (`cases.schema.json` is a contract stated twice,
    and this is the half a JSON-schema library does not run) and the `conform` census now
    reports how many compose cases name a non-default search.
  - `3.1.12/two-worked-examples/optimal-search-reports-the-pull-up-reduction-makes-available`
    is the new case, in `crates/jlreq-conform/cases/3.1.12.json` beside the two existing
    ones rather than in a file of its own: §3.1.12 ④ states the ideal, reduction-based
    repair in the same breath as the one it excuses ("Ideally, a full width spacing
    reduction would be applied, and the character... would be moved onto the first
    line... In that way, the problem could be avoided"), and the sibling case immediately
    above is deliberately built so that repair is unavailable. This entry is the missing
    half: a six-item paragraph and four candidates in which the nearer break admits no
    complete arrangement at all (verified directly, both by composing the remainder alone
    and by withholding the farther candidate) and the farther one is consequently the only
    admitted arrangement, not one preferred over a competing feasible one — the
    discriminating test this round's own brief states, applied and passed rather than
    asserted. Neither of the two published `Question::ADJUSTMENT_PREFERENCE` readings is
    named, because neither changes the outcome. `docs/conformance-deferrals.toml`'s
    `3.1.12` entry is updated to state what this case now covers; rule `3.1.12` stays
    `[[owned]]`, and no rule moves from `[[deferred]]` to `[[owned]]` this round.
  - Two stale claims this round's own work falsifies are repaired: `crates/jlreq-conform/
    src/kumihan.rs`'s module doc and its `compose` method no longer say `compose` asks only
    `Search::FirstFit`, now that a case can ask for `Search::Optimal` instead; and
    `docs/conformance-deferrals.toml`'s `3.5.4` entry no longer says widow adjustment
    "arrives with the objective" — the objective has arrived and 3.5.4 is still deferred,
    so the entry now states the real, checkable blocker instead
    (`Paragraph::with_widow_threshold`'s own doc: neither search reads the field it
    stores). A third, pre-existing claim is repaired on the same finding:
    `docs/design/conformance.md`'s "Cross-search agreement" section described, in the
    present tense, a gate that runs every case under both searches and compares them —
    true of nothing in `jlreq-conform` today, sharper now that cases naming `Search::
    Optimal` actually exist and are not run under `Search::FirstFit` too. The section is
    rewritten to say so plainly, keeping the design reasoning for when the gate is built
    rather than deleting it. A fourth, adjacent claim in the same section —
    "direction parity" composing every case both ways — was found false by the identical
    method (no such loop exists in `jlreq-conform` either) but is unrelated to this
    round's own changes and is left for the round that owns it, reported rather than
    fixed here.
- §3.5.4's widow adjustment, wired: `Paragraph::with_widow_threshold`'s own field is read
  for the first time, by `crates/jlreq-line/src/compose.rs`'s own `demerits_of` — the one
  cost function `compose_first_fit` and `evaluate_edge` both call, so `Search::FirstFit`
  and `Search::Optimal` are scored by one formula rather than two that could quietly
  disagree (this round's own reuse of the M3 round 19 C1 argument). `demerits_of` grows
  three parameters (`line: Range<ItemIndex>`, `is_last_line: bool`, `widow_threshold:
  u16`), threaded the same way `adjust_line`'s own signature, one call earlier in the
  identical pipeline, already threads the first two. Its own `..Demerits::ZERO`
  struct-update tail is dropped rather than kept: once `structural` is computed rather
  than left at its base value, all six of `Demerits`'s own fields are named explicitly,
  and `clippy::needless_update` (part of the default `complexity` group, not only
  `pedantic`) refuses a base that supplies nothing a literal does not already state —
  a deviation from the round's own brief, which asked for the idiom kept, stated here
  because a lint that fires is not optional.
  - A new private `WidowFacts`/`widow_facts_of` pair reads the paragraph's own last line
    (`is_last_line`, already derived identically at both call sites — `evaluate_edge`'s own
    C3) and reports how many items it carries and how far short of the threshold that
    falls, `u32::from(threshold).saturating_sub(have)` — shortfall-proportional, so an
    unsatisfiable threshold still discriminates between a nearer miss and a farther one
    rather than tying every violating arrangement. "A character" reads as an item
    (ADR-0008), and a last item `crate::ladder::hang` let hang past the measure is still
    counted: `hang`'s own `last` sits inside the line's own range, never past it.
  - `demerits_of` adds the shortfall to `Demerits::structural` on exactly the last line;
    `structural` already ranked first in both of `docs/decisions/adjustment-preference.md`'s
    own orderings, so `Search::Optimal` genuinely steers toward a widow-free last line
    when more than one arrangement admits one, ahead of every other component regardless
    of how much worse it scores there — proved by a constructed fixture
    (`optimal_steers_toward_a_widow_free_last_line_even_when_every_other_component_is_worse`)
    where the search takes an arrangement carrying two ladder violations (`badness =
    20_000`) over a fully feasible one, purely because the feasible one's own last line
    falls one item short. `Search::FirstFit` cannot do the same — it commits to one
    candidate per line and never compares arrangements — so it only ever reports the
    shortfall of the line it already greedily chose
    (`first_fit_reports_a_widow_but_never_moves_the_break_to_avoid_it`, pinning the
    asymmetry directly: the chosen breaks are byte-identical with and without a
    threshold).
  - A new `ViolationKind::Widow { have: u32, want: u16 }` variant (a minor addition under
    `#[non_exhaustive]`, ADR-0012) is pushed, once, for the last line only, in both
    `compose_first_fit`'s loop and `compose_optimal`'s own reconstruction loop, through a
    shared `push_widow_violation` so the check is written once rather than twice.
    `Violation::rule` names `RuleId::WIDOW_ADJUSTMENT_OF_PARAGRAPHS` — cited by no code
    anywhere in this workspace before this round — rather than the generic line-breaking
    rule every other violation in these two loops hardcodes, and `Violation::at` is the
    last line's own start, the break that could have moved, not the paragraph's own end,
    which is identical for every arrangement and says nothing. The violation is the point
    and not a garnish on the demerit: `Demerits` is this crate's own invented objective and
    a conformance case may never assert a demerit value as if it were JLReq's own answer
    (round 8's own brief), so `structural` alone would leave round 22 nothing JLReq-shaped
    to assert for the unsatisfiable case.
  - `docs/decisions/widow-threshold.md` (new), modeled on `tolerance-exhaustion.md`'s own
    four headings, publishes the four readings §3.5.4's silence forces and
    `docs/decisions/README.md` gains its row: what counts as "a character" (an item);
    whether a one-line paragraph can have a widow (yes, read literally — the exempting
    reading would add a condition the specification does not state, and the reading costs
    nothing because a constant addend across one candidate changes no comparison,
    `run_dp`'s own C2); the penalty's own shape (shortfall-proportional); and what an
    unsatisfiable threshold means (both remaining ADR-0010-licensed mechanisms together —
    graceful degradation through `structural`, plus the reported violation — never a
    refusal, and never the schedule-inventing relaxation `tolerance-exhaustion.md` already
    rejected by name for the identical reason). Seven new unit tests in `compose.rs`'s own
    `#[cfg(test)] mod tests` pin all of the above, including the threshold-0 no-op that is
    this round's own regression guard for all 834 pre-existing tests and all 467
    conformance cases, and the zero-item paragraph, checked directly rather than assumed,
    never growing a widow violation with nothing to report `have`/`want` for.
  - Every stale claim this round's own work falsifies is repaired in place. `compose.rs`'s
    `run_dp` doc no longer names `structural` "always 0 at this milestone" as a premise of
    its saturation bound — the replacement premise is stronger, not merely updated:
    `structural` cannot saturate at all, for any input, because the widow term lands on
    exactly one edge of any complete path and is bounded at `u16::MAX = 65,535`, so
    `badness` remains the one component the bound has to name.
    `Paragraph::with_widow_threshold`'s own doc no longer opens "Stored and still not
    read." `objective.rs`'s `Demerits::structural` field doc no longer says "Always zero
    even now that `Search::Optimal` exists." `crates/jlreq-line/src/lib.rs`'s own
    `# Status` no longer names the widow threshold "a real, named gap" beside
    `Search::Optimal` — it moves from the gap list to the filled, "neither Wired-not-
    slotted-nor-a-Slot" list `Search::Optimal` itself already occupies, and the four
    published readings replace it as what is honestly still open.
    `docs/conformance-deferrals.toml`'s `3.5.4` entry no longer blames the DP for not
    reading the threshold — it does now — and states the real, current blocker instead:
    coverage, not implementation, per ADR-0006's independently authored phases.
    `docs/design/api-spine.md` gains the new `ViolationKind::Widow` variant at its own
    frozen enum listing and a sharper one-line doc for `with_widow_threshold`.
    `docs/decisions/adjustment-preference.md` gains one sentence noting `structural`'s own
    first-rank position is now reachable rather than reserved, without reopening the
    ranking itself, which this round does not revisit. `compose_optimal`'s own
    reachability doc, `Search::Optimal`'s own "read by both search variants alike"
    sentence, and `tolerance-exhaustion.md`'s own FirstFit-comparability sentence all
    survive verbatim, checked rather than assumed: all three rest on `demerits_of` being
    the one shared cost function, a fact this round's wiring preserves rather than forks.
  - One stale claim this round's own sweep found and did not fix, on the same finding
    method as prior rounds' "direction parity" precedent: `docs/design/api-spine.md`'s own
    `ComposeError` block lists two variants (`OutOfRange`, `CandidateOutOfRange`) while the
    real enum has three (`crates/jlreq-line/src/compose.rs`'s own `InsufficientTabStops`,
    from the §3.6 tab-setting round) — pre-existing drift, unrelated to this round's own
    changes, checked directly (`api` gate parses the spine only for `Question` constant
    counts, never for enum variant listings, so nothing catches this mechanically) and left
    for the round that owns `ComposeError`, reported rather than fixed here.
  - **Phase discipline held**: no file under `crates/jlreq-conform/` changes this round,
    no conformance case is authored, and rule 3.5.4 stays `[[deferred]]` to M3 — task #58's
    entire reason to exist, per ADR-0006.
- Task #58 (round 22) is that independently authored phase, and closes M3's deferral list to
  zero.
  - `input.widow_threshold` (a non-negative integer, absent reading as `0`) reaches the case
    format for the first time: `cases.schema.json` gains its own description in `search`'s
    own long voice, `crates/jlreq-conform/src/case.rs`'s `CaseInput` gains the field, and
    `crates/jlreq-conform/src/kumihan.rs`'s `compose` reads it through a plain `if let`
    guard — not paired with `head_indent`/`end_indent`'s own `with_indents` call, because
    `Paragraph::with_widow_threshold` takes one field and has no sibling for a case to state
    without it, the doc now says so rather than leaving a reader to wonder. `xtask/src/
    conform.rs` gains `check_widow_threshold`, bounding a stated value at `0..=u16::MAX` —
    mirroring `check_search`'s own bound on `tolerance` — so a threshold this reader cannot
    hold declines `conform --check` rather than silently declining the case at runtime. No
    census line: unlike `Search::Optimal`, a whole second search variant that earned its own
    `optimal_search_census` line at round 20, this field is a scalar parameter the same shape
    as `first_line_indent`, `head_indent` and `end_indent`, none of which has one either.
  - `crates/jlreq-conform/cases/3.5.4.json` (new), three cases, derived from §3.5.4's own
    sentence and `docs/decisions/widow-threshold.md`'s own four published readings before
    this round's own suite run, on the same discipline round 20's `3.1.12.json` states in
    full. Q2 (a one-line paragraph can have a widow): a two-item, one-line paragraph
    composed with an empty `candidates` array, threshold above its own item count; the
    exempting alternative is `forbidden`. Q1 (a character is an item): a threshold-equal /
    threshold-past-the-count pair over a last line built from two Western-letter clusters on
    the proportional frame (`fi`, `if` — ADR-0018's own ligature exception), so the last
    line's item count (2) diverges from both its code-point count and its byte count (4 and
    4), discriminating item-counting from either. Q4 (violation, never refusal) is carried
    through the same channel both cases already use — a non-empty `lines` beside a real
    violation — and the Q1 pair's own discriminating case additionally rejects the
    relaxation alternative in `forbidden`, by name. Every case is `standing: "normative"`,
    not `"unstated"`: `conform --check`'s own `check_standing` requires `permitted` to carry
    more than one reading whenever standing is `unstated`, `adjudicated` or `alternative`,
    and none of these three cases has a second `Policy`-reachable answer for a second entry
    to name — the rejected alternative lives in `forbidden` instead, which carries no such
    requirement. `crates/jlreq-conform/tests/suite.rs` gains `section_3_5_4 => "3.5.4" [3
    attempted, 0 not attempted]`; all three agree with this workspace on the first run, zero
    disagreements, arithmetic derived by hand from `table1.rs`'s own cl-27×cl-27 and
    cl-19×line-end/line-head×cl-27 cells (all blank) before the suite ever ran.
  - `docs/conformance-deferrals.toml`'s `3.5.4` entry moves from `[[deferred]]` to
    `[[owned]]` at M3 (M3's deferred count: 1 → 0), with an honest scope limit rather than a
    claim of full coverage: it states which of the four readings a case reaches (Q1, Q2, Q4)
    and names the two it does not. Q3 (the penalty's own shape) is reported unconstructible
    for a reason sharper than difficulty deriving one fixture by hand — no `Policy` question
    selects a flat penalty, so no case in this format can compare the proportional reading
    against a reachable alternative, a limit of the format itself. The hanging wrinkle
    requires `adjustment.hanging_punctuation = hanging`, which `Policy::JLREQ` does not
    select, so a case exercising it would be published but never attempted, `3.8.2.json`'s
    own already-stated standing for a different rule; this round declines to publish one for
    that reason. The entry also states that this suite checks `ViolationKind::Widow`'s own
    address only, never its `have`/`want`, because `kumihan.rs`'s own `compose` discards both
    before the case format ever sees them.
  - Every stale claim this round's own work falsifies is repaired in place, on the same
    sweep discipline round 21's own entry above states: `objective.rs`'s
    `Demerits::structural` field doc and `crates/jlreq-line/src/lib.rs`'s own `# Status` no
    longer say §3.5.4 "stays `[[deferred]]`"; `docs/decisions/widow-threshold.md`'s own
    closing paragraph no longer says the suite carries none of the four readings — it now
    states which two it carries and which two it does not, in the same terms the ledger
    entry above uses.
- M1 round 11: the published conformance format's sixth `kind`, `feasible`, and the `Runs`
  overlay that answers it — closing §C.2#13's own deferral.
  - `crates/jlreq-conform/cases.schema.json` gains `"feasible"` in `input.kind`'s enum and a
    `feasible` `$def` for `expect.feasible` (`candidate`, `breakable`, `rules`), in the long
    voice `search`'s and `ruby`'s own descriptions use: which of the caller's own UAX #14
    candidates kinsoku permits and which rule refused each of the rest; why it is a separate
    kind rather than a `boundary` field (a `boundary` answer is Tables 1 and 2 at one
    adjacency, a candidate's survival is `jlreq-line`'s own refusal layer, which additionally
    reads a construct overlay no table cell can express — §C.2#6 through #8 and #13); and
    that `constructs` is the one field this kind reads as load-bearing rather than declining
    on account of, unlike every other kind.
  - `crates/jlreq-conform/src/case.rs`: `KINDS` grows to six. `Expect` gains
    `feasible: Option<ExpectFeasible>`, read by `read_feasible` on `read_boundary`'s own
    "every field optional" convention, and `Expect::is_silent` now checks it too — the quiet
    bug this round's own review caught before the gate battery could hide it: without that
    one line, every `forbidden` entry this round writes would have excluded nothing.
    `CaseConstruct` gains `style`, read from a `ruby` entry's own field, which the adapter
    needs to choose between `NonJukugoRuby` and `JukugoRuby`.
  - `crates/jlreq-conform/src/run.rs`: `Compose` gains a sixth method, `feasible`, required
    rather than defaulted for the identical reason `align` and `tab` already are.
    `CaseFeasible` (`breakable`, `rules`) answers it; `Answer::Feasible` and `ask`'s own
    `"feasible" =>` arm route it, distinctly from `align`'s and `tab`'s reuse of
    `Answer::Composed` — a candidate's own survival is nothing like a composed line.
    `check_feasible` reuses `check_boundary`'s own rules comparison, `check_rules`,
    generalized from `&CaseBoundary` to `&[String]` on both sides rather than duplicated, for
    the identical subset-not-equality reasoning stated once.
  - `crates/jlreq-conform/src/kumihan.rs`: `Compose::feasible` is the one method of the six
    that builds a real, non-`Runs::none()` overlay. The private `overlay_of` converts a
    case's declared `constructs` into one slot per item of the base stream, honestly and
    totally over the schema's nine construct arrays: `ornaments` and `tate_chu_yoko` convert
    unconditionally, `ruby` converts when `style` is `"mono"`, `"group"` or `"jukugo"`;
    `emphasis`, `jidori`, `formulae`, `warichu`, `furiwake` and `reference_marks` all decline,
    each for a reason its own doc states — no `ConstructKind` variant, an undeclarable
    `FormulaSetting`, or a declared range the schema does not pin to mean what the matching
    variant means. Every slot's `group` stays `None` (§C.2#8's own level below the run needs
    `ruby.runs`, not read this round), which `docs/decisions/jukugo-ruby-unset-group.md`'s
    own reading already treats as permitted rather than refused. One inconvertible construct
    anywhere in a case fails the whole conversion rather than leaving a silent gap in the
    overlay. The module doc's own "All five methods... every `Runs` this crate builds is
    `Runs::none()`" claims are repaired to name the sixth method and the one place a real
    overlay now exists.
  - `xtask/src/conform.rs`: `check_input` requires `candidates` (never `measure`) of a
    `feasible` case, and its `kind` match is now checked against an explicit `INPUT_KINDS`
    list instead of falling through a silent wildcard — an unrecognized `kind` is a
    violation now, rather than being quietly asked `compose`'s own required fields.
    `check_question` holds a `feasible` case to the identical "one input, one question"
    invariant `classify` and `boundary` already are. `Suite::census`'s own kind-counting line
    — extracted to `kind_census`, alongside `optimal_search_census`, to stay under
    `clippy::too_many_lines` — reports the new kind's count.
  - `docs/design/conformance.md` gains the sixth trait method and `CaseFeasible` in the same
    voice as the rest of the document; every stale "five methods"/"five questions" claim
    across `crates/jlreq-conform` and this document is repaired to six, and
    `crates/jlreq-conform/src/lib.rs` newly re-exports `CaseFeasible` and `ExpectFeasible` at
    the crate root — without which no implementation outside this crate could even name
    `Compose::feasible`'s own return type in its own `impl`.
  - `crates/jlreq-conform/cases/C.2.json` gains two `feasible` cases, derived independently
    from §C.2#13's own two sentences rather than from `jlreq_line::feasible::
    same_run_refusal`'s own match arms or its test module's fixtures (ADR 0006's own hazard
    for this route, read twice before writing either case):
    `two-characters-in-one-tate-chu-yoko-run/no-break-inside` (interior of one declared run,
    refused, citing `C.2#13`) and `two-tate-chu-yoko-runs-adjacent/break-permitted` (the
    boundary between two declared runs with nothing between them, permitted). Both sit at
    cl-15 against cl-15 (ordinary hiragana), a coordinate independently verified blank in
    Table 1, Table 2, Table 3's line-end row and Table 4's line-head row
    (`spec/captured/table1.en.tsv` through `table4.en.tsv`, read directly rather than
    inferred from this evaluator's own answer), so the refusal and the permission each case
    asserts can only be `same_run_refusal`'s own citation, never a class-pair prohibition
    coinciding by accident. `crates/jlreq-conform/tests/suite.rs`'s own committed census for
    `C.2` moves from `[8 attempted, 0 not attempted]` to `[10 attempted, 0 not attempted]`.
  - `docs/conformance-deferrals.toml`: §C.2#13 moves from `[[deferred]]` to `[[owned]]` at M1
    (M1's deferred count: 5 → 4), stating which two cases now measure it and by what
    mechanism. §C.2#6's and §C.2#7's existing `[[owned]]` entries are repaired: both
    previously said only that a boundary answer was published and "none receives yet" or
    "published as two boundary cases," without saying which cases or why none is answered;
    both now name the specific cases (`A.21/inside-one-complex/*` and
    `A.21/between-two-complexes/break-opportunity` for §C.2#6; `A.22/same-complex/
    no-break-inside` and `A.22/distinct-complexes/break-and-solid` for §C.2#7) and state
    plainly that every one of them declares a construct `Kumihan::boundary` declines per
    item, so this workspace answers none of them today — the ledger's own header already
    provides for exactly this state. §C.2#8 stays `[[deferred]]`, its own `why` rewritten to
    name the one gap this round did not close (`ruby.runs`'s own group reading in
    `read_constructs`) rather than the longer list of blockers this round's own overlay
    machinery removed.
  - The route not taken, and why: threading a caller-supplied `Runs` into
    `jlreq_line::compose` itself was rejected in favor of the `feasible` kind, for the three
    reasons `crates/jlreq-line/src/compose.rs`'s own `Runs::none()` comment and this round's
    own design already state — `jlreq_spacing::evaluate::delegation_of` would silently
    switch on §B.2#10/#11 delegation with no case behind it, `jlreq_class::resolve` stays
    construct-blind so a spacing amount inside a construct run would still answer the items'
    bare classes, and a same-run refusal is only ever observable as a differently-placed
    break, never as a cited rule — exactly the citable fact ADR 0006 needs a case to assert.
- M1 round 12: six `feasible` cases over §C.2 notes 6, 7 and 8, and the retraction of the
  former `C.2#8` deferral's own stated blocker.
  - `crates/jlreq-conform/src/kumihan.rs`: `overlay_of`'s own doc gains a new section,
    `` `ruby.runs` is a declared slot this function does not read ``, in the "Slots" sense
    `crates/jlreq-line/src/lib.rs`'s own module doc names — a seam a later, independently
    authored phase fills, not a gap this round left behind. No field is added; the paragraph
    states three facts as the reason the schema-required `annotation` and `runs` stay unread:
    a declared `GroupId` changes exactly one downstream answer (`same_run_refusal`'s own
    `JukugoRuby` arm); §C.2#8's own group is one base character and its own accompanying
    reading, never a span across two, so two adjacent base characters of one complex are
    never one group (§3.3.7's own body and §3.1.10 item 8's own Note, both quoted); and
    `Feasible::compute` sees the base item stream alone, so the level the note's third
    sentence is about is unreachable from this crate before `jlreq-inline` exists (M4-a). This
    is the only Rust change of the round — `crates/jlreq-line/**` and
    `crates/jlreq-conform/src/case.rs` are untouched, and `cases.schema.json` is unchanged,
    since `feasible`, `ruby`, `run` and `constructs` were already adequate.
  - `crates/jlreq-conform/cases/C.2.json` gains six `feasible` cases, authored as the
    independently authored phase ADR 0006 requires — verified against the note's own English
    sentences and the captured tables before being checked against this workspace's own
    answer, not derived from `same_run_refusal`'s own match arms: two for §C.2#6
    (`two-characters-in-one-ornamented-complex/no-break-inside`,
    `two-ornamented-complexes-adjacent/break-permitted`), two for §C.2#7
    (`two-base-characters-in-one-simple-ruby-complex/no-break-inside`,
    `two-simple-ruby-complexes-adjacent/break-permitted`), and two for §C.2#8
    (`two-jukugo-ruby-complexes-adjacent/break-permitted`,
    `two-base-characters-in-one-jukugo-ruby-complex/break-permitted`). Every one of the six
    sits at cl-19 against cl-19, independently verified `blank` in Tables 1 through 4
    (`spec/captured/table1.en.tsv` through `table4.en.tsv`; Table 6's own cell there,
    `0-1/4 stage 3`, is a third-order expansion opportunity named and set aside rather than
    silently omitted), so `jlreq_line::feasible::same_run_refusal`'s own citation is the only
    thing any of the six answers can be. The load-bearing pair reuses two existing fixtures
    verbatim with `kind` changed to `feasible` and one candidate added:
    `A.23/simple-ruby-complex/mono-ruby-twin`'s own input for the simple-ruby refusal and
    `A.23/jukugo-ruby-complex/first-base`'s own input for the jukugo-ruby permission, which
    (per `A.23/simple-ruby-complex/mono-ruby-twin`'s own rationale) differ in exactly one
    declared field, `style` — so the two new cases' answers, `breakable: false` against
    `breakable: true`, diverge over that one field alone, and an implementation that gave
    every same-run `ruby` construct one always-refuse rule fails the second while passing the
    first. `crates/jlreq-conform/tests/suite.rs`'s own committed census for `C.2` moves from
    `[10 attempted, 0 not attempted]` to `[16 attempted, 0 not attempted]`.
  - `docs/decisions/jukugo-ruby-unset-group.md`'s own closing paragraph is rewritten:
    `C.2/two-base-characters-in-one-jukugo-ruby-complex/break-permitted` now exercises this
    reading's own permissive outcome (both sides carry no group, exactly as `overlay_of`
    always builds them, and the case asserts the break permitted), corroborating it rather
    than merely being covered by the unit test the old paragraph named alone — but the
    refusing half of the reading, two occurrences with *equal, declared* groups, still has no
    case that can exercise it, since no case in this suite can declare a `GroupId` at all.
  - `docs/conformance-deferrals.toml`: `C.2#8` moves from `[[deferred]]` to `[[owned]]` at M1
    (M1's deferred count: 4 → 3, M1's owned count: 40 → 41), naming the two new cases and
    retracting the former entry's own stated blocker outright rather than merely closing it —
    that entry named `ruby.runs`'s unread `base`/`annotation` pairing as what stood between
    this note and a case; the actual reason is that the group level it would populate answers
    a question no base-to-base candidate this crate can construct is asking at all, verified
    against §3.3.7 and §3.1.10 item 8 directly rather than assumed from the prior entry's own
    words. The new entry states the scope limit honestly: the note's own third sentence,
    ruby-to-ruby indivisibility, is not measured and cannot be until `jlreq-inline` (M4-a).
    `C.2#6`'s and `C.2#7`'s own `[[owned]]` entries are rewritten to name the four new cases
    and keep the honest half each already carried — the A.21 and A.22 boundary cases they
    named before remain unanswered, for the identical `CaseInput::construct_covers` reason.
    `C.2#13`'s own entry, which this round also falsifies, is trimmed: its closing sentence
    used to say `ornaments` had no `feasible` case and that §C.2#8 stayed deferred for
    `ruby.runs`; both clauses are now false and are replaced with a pointer to the three
    entries above that now state their own current answer directly.
- `jlreq-inline`, M4-a round 1: mono-ruby lowering, the first coherent slice of M4-a. The
  crate is no longer a bootstrap; it depends on `jlreq-class`, `jlreq-spec` and `jlreq-unit`
  (`ARCHITECTURE.md`'s own declared row) and declares `ruby.rs`, `lower.rs` and `tcy.rs`.
  `Ruby::new` takes both the annotated text and the reading, validating that the base range
  lies inside the text, every declared `RubyRun`'s base and annotation ranges lie inside
  their own streams, the runs cover both in order without overlap, and the run count
  matches what `RubyStyle::MonoRuby`, `RubyStyle::GroupRuby` or `RubyStyle::JukugoRuby`
  requires; `Ruby::with_alignment` overrides `Question::RUBY_ALIGNMENT` per construct
  (ADR 0019's precedence rule). `Constructs::over`/`with_ruby`, `Lowered` and `Contribution`
  stand up the seam-facing half of `docs/design/api-spine.md`'s `jlreq-inline` section, and
  `lower` genuinely computes all four of `Contribution`'s outputs for `RubyStyle::MonoRuby`:
  a fresh `RunId` per base item (§3.3.5, §3.3.1's note — this is what gives two adjacent
  annotated bases §E.2 note 6's own quarter-em expansion opportunity), a `BlockDemand` per
  declared run from `Annotation::size_of` on the block-start side (§3.3.4), and a
  `Separation` wherever a base's reading is genuinely longer than its own supplied advance
  and the neighbor it would otherwise overhang resolves to cl-19 (§3.3.8 rule 1) — the
  surplus split evenly between the run's two boundaries and, where two runs' own shares
  land on one shared boundary, merged by the greater of the two rather than their sum
  (`docs/decisions/mono-ruby-separation-split.md`, a new published reading: §3.3.5(a)'s own
  centered geometry for nakatsuki, and, for katatsuki, its own second method's asymmetric
  hangover choice has nothing left to choose among once every reachable neighbor is cl-19,
  so the identical symmetric split survives under either alignment for this seam output).
  `RubyStyle::GroupRuby` and `RubyStyle::JukugoRuby` get real run identity — one shared
  `RunId` across a group-ruby's whole base range, one shared `RunId` across a jukugo-ruby
  compound with a fresh `GroupId` per base item inside it (§B.2#11, §C.2#8) — and real block
  demand, but no `Separation`: `Question::GROUP_RUBY_DISTRIBUTION` and
  `Question::JUKUGO_RUBY_LAYOUT` (with Appendix F) are named as unfilled slots rather than a
  citable zero. `Question::RUBY_OVERHANG_KANA` and `Question::RUBY_OVERHANG_INDENT` are
  unfilled slots too, for mono-ruby's own narrower scope: only rule 1's absolute cl-19
  prohibition is answered, never the permitted overhang those two questions govern.
  `lower` also resolves whether a per-construct or policy-default alignment is katatsuki in
  horizontal writing — §3.3.5's own direction-conditional recommendation, honored regardless
  and never refused (ADR 0011) — which is why it is the allowlisted `[[site]]` for §3.3.5 in
  `docs/direction-sites.toml`, retiring that file's own `[[pending]]` entry for it. The
  resolution is recorded rather than read once and dropped: `Contribution::alignment_of` and
  `Contribution::alignment_discouraged` are this round's own carrier of ADR 0019's "every
  answer records which of the two applied", pending a later round's `place()` or
  `jlreq::diagnose`'s own `AlignmentDiscouraged` to make it a caller-facing report.
  `TateChuYoko::new` states §3.2.5's own availability fact alone — no horizontal
  tate-chu-yoko exists to refuse into `NotAvailable` otherwise — added specifically because
  `docs/direction-sites.toml`'s own `[[pending]]` mechanism keys on whether a crate has
  declared *anything*, not on which item will do the reading, so the moment `ruby.rs`
  declared a `struct` both of `jlreq-inline`'s pending entries went stale at once; this round
  retires the §3.2.5 one honestly, by implementing the one sentence of tate-chu-yoko that is
  genuinely self-contained, rather than leaving it to lapse unrepaired or implementing the
  segment `Constructs::with_tate_chu_yoko` would need, which stays absent — an
  accepted-and-ignored `with_*` would be worse than the absence, so no such method exists and
  `lower` never sees a `TateChuYoko`. `place()`, `Attachment`/`Attachments`, `RubyOverhang`
  resolution, and the other eight constructs `docs/design/api-spine.md` names are unstarted,
  named as such in `crates/jlreq-inline/src/lib.rs`'s own rewritten `# Status`.
  `docs/conformance-deferrals.toml`'s `3.3.2`, `3.3.5`, `E.2#6`, `E.2#7` and `3.3.8` entries
  are rewritten against this reality: §3.3.2 and half of §3.3.5 are genuinely read now but
  still have no conformance case (no kind in this suite observes a `Contribution`, task #74);
  the other half of §3.3.5, and all of §3.3.6/§3.3.7's own distribution, remain `place()`'s
  later work; `E.2#6` and `E.2#7`'s own prior entries are corrected independently of this
  round's own reachability, not only extended by it — `jlreq_class::resolve` never took a
  construct parameter at all, so it was never accurate to say it "computes no run overlay
  until `jlreq-inline` places ruby," and `crates/jlreq-conform`'s own `Compose::boundary` and
  `Compose::compose` decline unconditionally over any declared construct regardless of run
  identity, which is the actual reason neither note's own Table 6 coordinate is reachable by
  a case yet; and `3.3.8`'s own `[[owned]]` entry now distinguishes its two halves — rule 1's
  forced separation is a real, tested evaluator mechanism as of this round, rules 2 through 6
  remain entirely unattempted. `clippy.toml` gains `enum-variant-name-threshold = 4`,
  reviewed and documented, so `RubyStyle`'s three JLReq-named variants (`MonoRuby`,
  `GroupRuby`, `JukugoRuby`) do not trip `clippy::enum_variant_names` at the workspace's own
  three-variant default.
- M4-a round 2: the `jlreq` → `jlreq-inline` facade edge, and the published conformance
  format's seventh `kind`, `lower` — closing §3.3.5's own deferral and giving §3.3.8's own
  `[[owned]]` entry its first genuine cases.
  - `crates/jlreq/Cargo.toml` gains `jlreq-inline` as a dependency, and `crates/jlreq/src/
    lib.rs` re-exports its whole public surface (`Constructs`, `Contribution`, `LowerError`,
    `Lowered`, `Ruby`, `RubyAlignment`, `RubyError`, `RubyRun`, `RubyStyle`, `TateChuYoko`,
    `NotAvailable`, `lower`) in the same `pub use jlreq_*::{…}` shape the other five layers
    already get — an edge `xtask/src/purity.rs`'s own `CRATE_GRAPH` and `ARCHITECTURE.md`'s
    own crate-boundary table already sanctioned, so this is the edge existing, not a gate
    changing. The crate's own `# What is here today` and `# Status` sections are rewritten
    against what is actually true now: six layers rather than five are re-exported; the
    reduction, hanging and expansion ladders `jlreq_line::ladder` implements are no longer
    named as unfilled slots; "every construct-bearing input is `jlreq-inline`'s, which does
    not exist yet" is repaired to state precisely what is real (mono-ruby lowering) and what
    is not (placement, the other eight constructs); and the `diagnose` sentence, which used
    to read as though the function should exist now that the crate that carries the
    constructs has arrived, is repaired to name it as still unwritten.
  - `crates/jlreq-conform/cases.schema.json`: `input.kind`'s enum gains `"lower"`, with a
    paragraph beside `feasible`'s own stating what a `lower` case asks — not a line-layer
    question at all, but what `jlreq_inline::lower` resolved for one declared `ruby`
    construct — and that it requires `constructs` and reads none of `measure`, `candidates`,
    `alignment`, `tab_starts` or `tab_stops`. A new `$defs/lower` (`construct`, `same_run`,
    `separations`, `alignment`, `alignment_discouraged`, `rules`) sits beside `$defs/
    feasible`, with `$defs/same_run` and `$defs/lower_separation` beside it — `same_run` an
    object (`{ "items": [i, j], "same": bool }`) rather than a bare triple, this format's own
    established practice; `separations` a *total* list, `boundary.spaces`'s own convention,
    so a case stating one entry asserts both that it exists and that the answer carries no
    other; `least` a bare unit count rather than a `$defs/amount` fraction, because unlike
    Table 1's own terms this amount is not a fraction of an em JLReq states anywhere. No
    alignment-override field is added to `$defs/ruby`: ADR-0019's per-construct-beats-policy
    precedence is this workspace's own bookkeeping, not something JLReq states, and a case
    asserting it would measure kumihan's own API rather than the specification — it stays
    covered by `crates/jlreq-inline/src/lower.rs`'s own unit tests, and a `lower` case
    selects between the two alignments through `permitted[].policy`'s own `ruby.alignment`
    overlay instead. `$defs/constructs`'s and `boundary.rules`'s own descriptions are
    repaired: `lower` joins `feasible` as a kind `constructs` is load-bearing for, and the
    twelve pre-existing `A.16.json`/`A.22.json` boundary-`rules` declarations are still not
    live — not because `jlreq-inline` does not exist, which is no longer true, but because
    `Compose::boundary` still declines outright over any construct-covered item, exactly as
    it did before this round and for an unrelated reason.
  - `crates/jlreq-conform/src/case.rs`: `KINDS` grows to seven. `Expect` gains
    `lower: Option<ExpectLower>`, read by `read_lower`, and `Expect::is_silent` checks it
    too. `ExpectLower`, `ExpectSameRun` and `ExpectLowerSeparation` are the new types.
    `CaseConstruct` gains `annotation` and `runs` (a new `CaseRun`), read from a `ruby`
    entry's own fields — the part `Compose::lower`'s own adapter needs and `Compose::
    feasible`'s never did, on this module's own "read here rather than reach into the raw
    JSON a second time" principle.
  - `crates/jlreq-conform/src/run.rs`: `Compose` gains a seventh method, `lower`, required
    rather than defaulted for the identical reason `feasible` already is — a breaking change
    to a published trait, exactly as adding `feasible` was. `CaseLower` (`runs`,
    `separations`, `alignment`, `alignment_discouraged`, `rules`) answers it; `Answer::Lower`
    and `ask`'s own `"lower" =>` arm route it, distinctly from `align`'s and `tab`'s reuse of
    `Answer::Composed` — one construct's own run identity, forced spacing and resolved
    alignment is nothing like a composed line. `check_lower` compares `same_run` against the
    answer's own per-item run identity, `separations` as a total list (`check_spaces`'s own
    convention), `alignment`/`alignment_discouraged` by equality when stated, and `rules`
    through the same `check_rules` `boundary.rules` and `feasible.rules` already share. Every
    "six methods"/"six questions" claim in this module's own docs, including the `Answer`
    enum's own "four variants for six questions" and the wildcard-arm hazard prose in `ask`'s
    own doc, is repaired to seven and five respectively, with `lower`'s own hazard stated
    beside `feasible`'s.
  - `crates/jlreq-conform/src/kumihan.rs`: `Compose::lower` is the second method that does
    not inherit the construct-blindness `classify`, `boundary`, `compose`, `align` and `tab`
    all share, and it is not a milder version of `feasible`'s own exception — it never calls
    `jlreq_class::resolve` or any `jlreq_line` entry point at all, only `jlreq::lower`
    (`jlreq_inline::lower`) directly. Three new staged helpers build the real `jlreq::Ruby`
    slice a case's declared `constructs.ruby` describe — `annotation_streams_of`/
    `annotations_of` (a two-phase read into `jlreq_class::Annotation`, staged because an
    `Annotation` borrows its items and scales and a temporary cannot outlive it) and
    `ruby_runs_of`/`rubies_of` (the identical staging for `jlreq::RubyRun`, which `jlreq::
    Ruby::new` also borrows) — declining the whole case the moment any declared construct is
    not `ruby`, `jlreq::Ruby::new` refuses one (`RubyError`), or `jlreq::lower` itself refuses
    the result (`LowerError`). The module's own doc is rewritten: "every kind but `feasible`
    either declines outright... or declines per item" is repaired to name `lower` as the
    second exception, and states precisely why its own exception is a different shape from
    `feasible`'s rather than a milder version of it.
  - `xtask/src/conform.rs`: `INPUT_KINDS` grows to seven; `check_input` requires
    `constructs` (never `measure` or `candidates`) of a `lower` case; `check_question` holds
    it to the identical "one input, one question" invariant `classify`, `boundary` and
    `feasible` already are, keyed on `expect.lower.construct`. `kind_census` reports the new
    kind's count, and a `MINIMAL_LOWER` fixture plus
    `the_kind_census_line_counts_a_lower_case_by_its_own_kind` mirror the identical `feasible`
    precedent (round 20's own `optimal_search_census` pattern, applied a third time).
  - `crates/jlreq-conform/tests/suite.rs` gains `section_3_3_5` (`[2 attempted, 0 not
    attempted]`) and `section_3_3_8` (`[2 attempted, 0 not attempted]`).
  - `docs/design/conformance.md` gains the seventh trait method and `CaseLower` in the same
    voice as the rest of the document; every stale "six methods"/"six questions" claim is
    repaired to seven, and `crates/jlreq-conform/src/lib.rs` newly re-exports `CaseLower`,
    `ExpectLower`, `ExpectLowerSeparation`, `ExpectSameRun` and `CaseRun` at the crate root.
  - `crates/jlreq-conform/cases/3.3.5.json` (new): two `lower` cases closing §3.3.5's own
    deferral. `ruby-alignment/policy-selects-nakatsuki-or-katatsuki` asserts
    `Contribution::alignment_of` against both of `Question::RUBY_ALIGNMENT`'s choices;
    `katatsuki-in-horizontal-writing/discouraged-but-honored` asserts `Contribution::
    alignment_discouraged` against the section's own "should not be adopted" recommendation
    — honored and reported, never refused (ADR-0011), with the resolved alignment staying
    katatsuki rather than silently reverting. Both cases' own katatsuki-selecting entries are
    published and checked but not genuinely exercised by this workspace's own committed run:
    `crates/jlreq-conform/tests/suite.rs` only ever constructs `Kumihan::default()`, whose
    declared `ruby.alignment` is `Policy::JLREQ`'s own nakatsuki default, so the katatsuki
    entries are statements to another implementation that declares the alternative (ADR
    0006) — `crates/jlreq-inline/src/lower.rs`'s own `katatsuki_is_honored_and_discouraged_
    only_in_horizontal_writing` unit test is what exercises them directly. `docs/
    conformance-deferrals.toml` moves §3.3.5 from `[[deferred]]` to `[[owned]]` at M4,
    stating this scope limit plainly and naming `place()` (task #78) as the section's own
    remaining, unstarted half.
  - `crates/jlreq-conform/cases/3.3.8.json` (new): two `lower` cases giving §3.3.8's own
    `[[owned]]` entry its first genuine coverage of rule 1's forced separation.
    `forced-separation/only-beside-ideographic-neighbors` (`standing: "normative"`) asserts
    existence and absence together — one oversized mono-ruby construct beside a cl-19
    neighbor forces a separation, an equally oversized construct beside a hiragana neighbor
    forces none — with no asserted amount, since rule 1 states the prohibition and no
    arithmetic. `forced-separation/even-split-by-remainder-policy` (`standing: "unstated"`)
    asserts the even-split amount `docs/decisions/mono-ruby-separation-split.md` reads, one
    mono-ruby construct between two cl-19 neighbors with an odd surplus, both `adjustment.
    remainder` readings published side by side and neither asserted as JLReq's own
    requirement — the discipline the §E.2#11 deferral already argues for a different
    coordinate, applied here on purpose. `docs/decisions/mono-ruby-separation-split.md`'s
    own closing sentence, which promised this task as the phase that would first exercise
    its reading against a published case, now names both cases by id in the past tense.
  - `docs/conformance-deferrals.toml`'s other stale `why` fields, repaired against what this
    round's own reading of `spec/snapshot/index.html` and `crates/jlreq-unit/src/seam.rs`
    finds rather than against what a prior round assumed: §3.3.2's own former reasoning (that
    the only blocker was no conformance kind observing a `Contribution`) is retracted rather
    than merely closed — the section's own body is entirely editorial choices an author makes
    before ever declaring a `Ruby` (general-ruby, para-ruby, and para-ruby's own first-
    instance variants), upstream of anything `lower` computes, with one mechanizable residue
    (the compound-word recommendation) that needs jukugo lowering and `jlreq::diagnose`,
    neither of which exists. §3.3.4's own former reasoning (that the physical side was "one
    answer `jlreq-inline` produces… from a single rule") is replaced with the actual, verified
    blocker: `jlreq_unit::BlockDemand::new`'s own doc defines its first extent as
    direction-abstract — "toward the ruby side," never "above" or "to the right" — and `lower`
    calls it identically at all three of its own call sites for every ruby style, so "start
    extent non-zero, end extent zero" is structurally true of every demand this crate will
    ever emit regardless of whether §3.3.4 was ever read, an observable that would pass
    whether or not the mechanism existed. §E.2#6's entry is repaired to say that two kinds now
    read a case-declared overlay (`feasible` since M1 round 11, `lower` new this round), not
    one, while stating precisely why neither reaches Table 6's own expansion amount, which is
    what the note is about. §3.3.8's `[[owned]]` entry states which two cases now measure rule
    1's forced separation, replacing the "no kind in this suite observes one" clause this
    round falsifies.
- M4-a round 3: `jlreq_inline::place`, the placement half of §3.3.5 (task #78). New,
  additive `pub fn place`, `Attachment` and `Attachments` on `jlreq-inline`, re-exported
  from `jlreq`; neither `Lowered` nor `Constructs` changes shape in a way any existing
  caller can observe (`Lowered` gains two `pub(crate)` buffers `lower()` never touches).
  - `place()` genuinely computes three of §3.3.5's four positioning cases for
    `RubyStyle::MonoRuby`: nakatsuki (中付き) centering, including a run genuinely longer
    than its base, where the centering difference and its two shares go negative and the
    run starts before its own base's placement; and katatsuki (肩付き) start-alignment
    where the run is not longer than the base. §3.3.5(a)'s own two-hiragana-exactly-fills-
    the-base case is not a fifth branch — at that ratio both alignments agree without
    either reading a character count, and a unit test demonstrates the agreement falling
    out rather than being special-cased.
  - §3.3.5(c)'s own katatsuki-with-overflow choice — the section states two methods for it
    in so many words, and no `Question` in `spec/derived/questions.tsv`'s own §3.3.5
    neighborhood resolves between them — is genuinely declined rather than guessed at:
    `place()` emits no `Attachment` for such a run and reports it through the new
    `Attachments::declined` instead. Giving that choice a policy `Question` is task #81, a
    round of its own by design.
  - `docs/design/api-spine.md`'s own `overhang: &[RubyOverhang]` parameter is a deliberate
    omission this round, argued in `jlreq_inline::place`'s own module doc and reflected
    back into the spine's sketch: nothing this round's three positioning cases reads a
    per-boundary allowance, and an accepted-and-unread parameter is the silent defect this
    crate already refuses elsewhere. The parameter returns at task #81, its first genuine
    consumer.
  - `Attachment::side` answers `Side::BlockStart` for every attachment this round produces,
    and `Attachment::block` answers `BlockOffset::ZERO` for a different reason — this
    signature carries no block-axis reference frame at all — and both accessors' own docs
    say so plainly rather than let the constant answer read as §3.3.4 settled. §3.3.4
    stays deferred to M4; its `docs/conformance-deferrals.toml` entry is repaired to say
    that `place()` now exists and still cannot state a physical side, the structurally-
    constant trap the entry already predicted rather than one it has now closed.
  - No `place` conformance kind and no case JSON this round (ADR-0006: implementation and
    conformance are separately authored phases; task #80 is the latter).
    `docs/conformance-deferrals.toml`'s §3.3.5 `[[owned]]` entry is repaired to state
    precisely what moved — three of four positioning cases implemented, one declined and
    why, and that no conformance case observes any of the placement half yet — rather than
    naming `place()` as unstarted, which this round falsifies.
  - `docs/conformance-deferrals.toml`'s §3.3.8 `[[owned]]` entry closed with a forward
    reference to "placement's own later work (task #78)" for rules 2 through 6's own
    overhang permissions over kana, half-em spaces, inseparable characters and brackets
    (`Question::RUBY_OVERHANG_KANA`, `Question::RUBY_OVERHANG_INDENT`). That reference is
    repaired now that task #78 has shipped and, per `place()`'s own module doc, deliberately
    reads neither question — the identical falsified-forward-reference repair its sibling
    §3.3.4 entry already received, missed for this entry the first time through.
  - `docs/decisions/mono-ruby-separation-split.md`'s own "Applies to" line now names
    `jlreq_inline::place` alongside `jlreq_inline::lower`: the centering difference
    `place()` splits and the §3.3.8 rule 1 surplus `lower()` splits are the identical
    `distribute(_, &[one(), one()], _)` question asked of two different inputs, not two
    readings, so `place()` cites this file rather than arguing the point a second time.
- M4-a round 4: the published conformance format's eighth `kind`, `place` — the independent
  conformance phase for `jlreq_inline::place` (task #80) — plus the falsifiable `same_run`
  `lower` case the harness carried unexercised since M4-a round 2.
  - `crates/jlreq-conform/cases.schema.json`: `input.kind`'s enum gains `"place"`, with a
    paragraph stating what a `place` case asks — not `lower`'s own alignment question
    restated, but what `jlreq_inline::place` computes once that alignment is read and
    consumed — and that the line layout `place` positions each attachment against is
    *derived* from the case's own declared item advances and `lower`'s own forced §3.3.8
    rule 1 separations rather than accepted as a further caller-declared field, stating why
    in full: a caller-declared `placements` array could assert a relationship between two
    numbers the case itself invented, with nothing in the format able to catch the two
    disagreeing — the "measuring nothing" failure §D.2#4 forbids, and a subtler one than a
    stated scope limit because it would look like a stronger assertion than it is. A new
    `$defs/place` (`attachments`, `declined`) and `$defs/attachment` (`inline`, `item`) sit
    beside `$defs/lower`; `$defs/place` states plainly that it carries no `rules` field,
    because `Attachments` publishes none (ADR-0019), so a later reader does not add one back
    as an oversight. `$defs/constructs`'s own description repairs "the two kinds this object
    is load-bearing for" to three.
  - `crates/jlreq-conform/src/case.rs`: `KINDS` grows to eight. `Expect` gains
    `place: Option<ExpectPlace>`, read by `read_place`, and `Expect::is_silent` checks it
    too. `ExpectPlace` and `ExpectAttachment` are the new types, both `size`/`side`/`run`/
    `construct`-free by design — `cases.schema.json`'s own `attachment` description states
    why each is left out.
  - `crates/jlreq-conform/src/run.rs`: `Compose` gains an eighth method, `place`, required
    for the identical reason `lower` already is — but shaped like `align`/`tab`/`compose`
    rather than `boundary`/`feasible`/`lower`: `place` answers the whole call, not one
    occurrence of it, so it takes no ordinal, and its own doc states why inventing one would
    invent a selector `place()` does not have. `CasePlace` and `CaseAttachment` answer it;
    `Answer::Place` and `ask`'s own `"place" =>` arm route it, and a misrouting regression
    pair — `a_place_case_reaches_compose_place_and_not_compose_compose`,
    `a_place_case_with_no_place_answer_is_not_attempted_even_though_compose_has_one` —
    proves the hazard the identical pair already proved for `lower`. `check_place` compares
    `attachments` as a total list (`check_lower_separations`'s own convention) and `declined`
    by full-list equality, asserting the specific declined construct ordinal rather than
    merely its non-emptiness — `a_place_declined_expectation_names_the_specific_construct_
    ordinal` pins it. Every stale "seven methods"/"seven questions" claim in this module's
    own docs is repaired to eight.
  - `crates/jlreq-conform/src/kumihan.rs`: `Compose::place` is the third method that does not
    inherit `classify`'s, `boundary`'s, `compose`'s, `align`'s and `tab`'s construct-
    blindness, reusing `lower`'s own front half verbatim through an identical `jlreq::lower`
    call, then deriving the line layout `jlreq::place` positions against: `derived_placements`
    sums the declared item advances and every forced separation that `lower` call resolved
    before each item, honest to the case's own data rather than a caller-declared restatement
    of it — its own doc states the derivation's honesty requirement (faithful only where every
    interior boundary of the declared stream is Table 1 `blank`) and that no case this round
    publishes exercises its own separations term. `docs/scalar-sites.toml` gains one entry for
    it, the bridge from a case's own plain unit counts to the `InlineOffset` sequence
    `jlreq::place` reads as `placements`.
  - `xtask/src/conform.rs`: `INPUT_KINDS` grows to eight; `check_input` requires `constructs`
    of a `place` case, merged into `lower`'s own existing match arm since the two share the
    identical requirement (`clippy::match_same_arms`); `check_question` holds `place` to its
    own `expect.place` field with no ordinal, `align`'s and `tab`'s own empty-ordinal shape
    rather than `boundary`'s, `feasible`'s and `lower`'s. `kind_census` reports the new kind's
    count.
  - `crates/jlreq-conform/cases/3.3.5.json` gains four `place` cases closing the placement
    half of §3.3.5's own `[[owned]]` entry: `one-character-nakatsuki-vs-katatsuki` (§3.3.5(b),
    the load-bearing pair — the same run, two resolved offsets, one base item so the interior
    boundary is vacuous), `two-characters-exactly-filling-the-base` (§3.3.5(a),
    alignment-independent by construction, published as one `permitted` entry rather than two
    identical ones), `three-characters-longer-than-the-base` (§3.3.5(c), both nakatsuki's own
    negative-share centering over a verified-blank cl-15/cl-19 Table 1 coordinate chosen so no
    §3.3.8 rule 1 separation entangles the derivation, and katatsuki's own decline, asserting
    the specific declined construct ordinal), and `group-ruby-placement/produces-no-
    attachment-and-is-not-declined` (the boundary between this rule's own reach and §3.3.6's,
    measured from outside it). The two existing `lower` cases' own rationales are repaired:
    both once asserted their katatsuki entry "is not genuinely exercised by this workspace's
    own committed test run," falsified by this round's own `crates/jlreq-conform/tests/
    suite.rs` addition below.
  - `crates/jlreq-conform/tests/suite.rs` factors `measure` into `measure`/`measure_against`
    and adds `section_3_3_5_is_also_measured_under_katatsuki`, a second run of `3.3.5.json`
    against a `Kumihan::new(Policy)` declaring `ruby.alignment: katatsuki` — under which every
    katatsuki `permitted` entry in that file is the selected reading rather than `{}`,
    genuinely exercised rather than only published. `section_3_3_5`'s own row moves to
    `[6 attempted, 0 not attempted]` (the two existing `lower` cases plus the four new `place`
    cases, none of which decline under either policy — decline conditions read no policy at
    all, so the census is identical under both runs and only which entry is selected moves).
  - `crates/jlreq-conform/cases/A.22.json` gains `run-identity/group-ruby-shares-a-run-mono-
    ruby-does-not`, the falsifiable `same_run` `lower` case the harness carried unexercised
    since `lower.same_run`, its reader and `check_same_run` first shipped (M4-a round 2) —
    grounded in §B.2 note 10's own "the same... run" / "two distinct... runs" language for
    cl-22 (simple-ruby, mono-ruby together with group-ruby, §3.3.7's own closing Note), not in
    §3.3.5, whose own subject this fact is not: `RubyStyle::MonoRuby` allocates a fresh
    `RunId` per base character by definition (§3.3.1's own note, the E.2#6 quarter-em
    opportunity between 鬼 and 門), so two adjacent base characters of a *declared mono-ruby
    construct* never share a run, whichever JSON shape declares them — only group-ruby (or
    jukugo-ruby) allocates one shared run across a base range. The case's one input carries
    both halves at once: two items under one `group`-ruby construct (`same: true`), two more
    under two separate `mono`-ruby constructs (`same: false`). Neither `B.2#10` nor `C.2#7`
    moves off `[[owned]]`; both were already there. `appendix_a_22`'s own row moves to
    `[2 attempted, 11 not attempted]`.
  - `docs/conformance-deferrals.toml`'s §3.3.5 `[[owned]]` entry is rewritten a second time:
    the alignment question is now genuinely *exercised* under both readings, not only
    published and checked; three of §3.3.5's four positioning cases are now cased, and the
    fourth — §3.3.5(c)'s own katatsuki-with-overflow choice — is cased as a decline,
    asserting the specific construct ordinal, pending task #81 for the `Question` that would
    resolve it in full.
  - `crates/jlreq-inline/src/place.rs`'s own "What is not here" paragraph and
    `crates/jlreq-inline/src/lib.rs`'s own `# Status` are both repaired: the `place`
    conformance kind this round authors is no longer a forward reference to task #80, and
    both name the four cases and the `Attachments` observable directly.
- M4-a round 5: `RubyStyle::GroupRuby` placement, §3.3.6 paragraphs 1 and 2 (task #84).
  `jlreq_inline::place` gains a real `RubyStyle::GroupRuby` branch — additive, no change of
  shape to any existing public item, and every existing mono-ruby offset unchanged, because
  the new geometry lives in sibling functions (`place_group_run`, `place_group_solid_run`)
  rather than in a generalization of `place_solid_run`.
  - `place_group_run` genuinely computes §3.3.6's own ruby-not-longer-than-base half, over
    both of `Question::GROUP_RUBY_DISTRIBUTION`'s answers: `jis`, a `[1, 2, 2, …, 2, 1]`
    proportional split over `n + 1` sites (`group_jis_weights`), read as §3.3.6's own "2
    units of inter-character spacing... 1 unit" ratio; and `flush`, a fixed
    `InlineExtent::ZERO` leading offset with an equal split over the `n - 1` interior sites
    alone (`group_flush_weights`), honoring the method's own leading clause by construction
    rather than by a zero-weight site — `jlreq_unit::distribute`'s own remainder machinery
    hands units out across every site a weights slice names, zero-weighted or not, so a
    zero-weight site would not have stayed zero. Both methods read the base run's own extent
    from a composed line's own `placements` (`extent_between`, a new `docs/scalar-sites.toml`
    entry), not from a re-derived sum of item advances, so the two never silently disagree
    when composition has genuinely widened the base elsewhere on the line. Paragraph 1 (equal
    length) is not a third branch — at zero surplus both methods place the run flush with the
    base's own start regardless of weight shape, the ratio paragraph 2's own arithmetic
    degenerates to. An unrecognised `Question::GROUP_RUBY_DISTRIBUTION` answer name falls to
    `jis`, every one of `Policy`'s five presets' own answer.
  - Paragraph 3 (ruby longer than base) is declined, not implemented: both of its own methods
    spread the *base* characters apart, which `place` structurally cannot do — `placements`
    is already fixed by the time `place` runs, and it emits `Attachment`s for annotation items
    only. A `RubyStyle::GroupRuby` run whose ruby is genuinely longer than its base is
    reported through `Attachments::declined` instead, exactly the discipline §3.3.5(c)'s own
    katatsuki-with-overflow choice already established; the fix belongs to
    `jlreq_inline::lower::lower_group`, which would need to emit forced `Separation`s before
    composition ever sees the base run, the mono-ruby analogue `collect_mono_separation`
    already performs for §3.3.8 rule 1 — a future round's work, not this one's.
    `Attachments::declined`'s own published meaning widens accordingly: it is no longer
    reserved for §3.3.5(c)'s choice alone, and its own doc, and `crate::place`'s own module
    doc, both now enumerate the two reasons a run reaches it. Jukugo-ruby remains a *third*,
    different kind of absence — never placed at all, never declined, because no weighing ever
    happened for a style this round's code simply does not touch.
  - The Note attached to §3.3.6 paragraph 2 — a criterion capping the leading/trailing
    spacing at one to one-and-a-half ruby ems before `jis`'s own appearance turns misleading —
    states two thresholds in one parenthesis rather than one, so it is named as a declared
    slot in `crate::place`'s own module doc rather than wired to an invented number; closing
    it needs a policy `Question` of its own or a `docs/decisions/` reading, neither built yet.
  - `docs/decisions/group-ruby-flush-single-character.md`, a new published reading
    (`Standing::Unstated`): what `flush` does for a run of exactly one ruby character, whose
    leading and trailing clauses name the same character at once and whose "rest" to space is
    empty. The reading holds that the run starts at the base's own start with the surplus
    applied nowhere — falling out of `group_flush_weights`' own empty slice at `count == 1`
    rather than a special case — and argues against falling back to `jis`'s own centering,
    which would erase the very divergence between the two methods §3.3.6 states them for.
    Confirmed direction-independent; no `docs/direction-sites.toml` entry follows.
  - `docs/scalar-sites.toml` gains two `jlreq-inline` entries: `two` (`lower.rs`, the `jis`
    method's own interior weight, twice `one`'s own — a different item from `one`, so it
    needs its own reviewed entry) and `extent_between` (`place.rs`, the base run's own extent
    read back from two already-resolved placements, `jlreq_line::tab::distance_to`'s own
    crossing one crate over).
  - `crates/jlreq-conform/cases/3.3.5.json` loses `3.3.5/group-ruby-placement/produces-no-
    attachment-and-is-not-declined`, deleted rather than retargeted: its own fixture (a
    1000-unit base against two 500-unit ruby characters, surplus exactly zero) now places two
    real attachments under this round's own §3.3.6 paragraph 1 arithmetic, falsifying the
    case's own premise that group-ruby produces no attachment. `crates/jlreq-conform/tests/
    suite.rs`'s own `section_3_3_5` and `section_3_3_5_is_also_measured_under_katatsuki` move
    from `[6 attempted, 0 not attempted]` to `[5 attempted, 0 not attempted]`. Retargeting the
    case, or authoring §3.3.6's own cases, is task #85's — ADR-0006's separately-authored
    conformance phase, not this implementation round's; §3.3.6 stays `[[deferred]]` in
    `docs/conformance-deferrals.toml`, whose own entry is rewritten to state exactly what
    moved (the implementation) and exactly what did not (a conformance case naming 3.3.6).
    `docs/conformance-deferrals.toml`'s own §3.3.5 `[[owned]]` entry is repaired to match:
    three of task #80's own four cases survive unchanged, and the fourth's own deletion is
    stated and reasoned rather than silently dropped from the count.
  - Every stale "unfilled slot" claim about `Question::GROUP_RUBY_DISTRIBUTION` this round
    falsifies is repaired: `crates/jlreq-inline/src/lower.rs`'s own module doc, `one`'s own
    doc (now naming its four consumers rather than two), `sum_advances`' own doc (three
    questions rather than two), `Lowered::declined`'s own field doc, `lower_group`'s own doc
    and its "Recording §3.3.6 here..." comment (reworded to state that `lower_group` still
    computes none of this — placement does — rather than that the geometry does not exist,
    and that `place` itself still records no `RuleId` either, ADR-0019), and its own test's
    assertion messages; `crates/jlreq-inline/src/ruby.rs`'s own `RubyStyle::GroupRuby` doc;
    `crates/jlreq-inline/src/lib.rs`'s own `# Status`; and
    `docs/design/api-spine.md`'s own `Attachments` sketch, whose `declined` description named
    only §3.3.5(c) and now names both reasons.
- M4-a round 6: the §3.3.6 group-ruby placement conformance cases (task #85), ADR-0006's own
  separately-authored phase for M4-a round 5's own implementation. No logic change to
  `crates/jlreq-inline/src/place.rs` — every number below was derived by hand from §3.3.6's
  own words and this round's own fixture advances, never read out of the implementation, its
  `#[cfg(test)]` module or a debug run.
  - `crates/jlreq-conform/cases/3.3.6.json`, four cases naming rule `3.3.6`:
    `group-ruby-placement/equal-length-both-methods-agree` (paragraph 1, one `permitted` entry
    because `jis` and `flush` are not two readings at zero surplus but one, the deleted
    `3.3.5/group-ruby-placement/produces-no-attachment-and-is-not-declined` case's own
    fixture reused as this section's own affirmative case); `group-ruby-placement/jis-versus-
    flush-distribution` (paragraph 2 at four ruby characters over a two-item, cl-19/cl-19
    Table-1-verified-blank base — `spec/captured/table1.en.tsv` line 485 — every one of the
    run's four offsets genuinely differing between the two methods, the load-bearing pair this
    file exists to publish); `group-ruby-placement/single-ruby-character-jis-vs-flush`
    (paragraph 2 at exactly one ruby character, standing `unstated` rather than `alternative`:
    `jis` is still directly derivable from the ratio sentence, but `flush` is not, and rests on
    `docs/decisions/group-ruby-flush-single-character.md`'s own published reading instead);
    and `group-ruby-placement/ruby-longer-than-the-base-declines` (paragraph 3, asserting
    `declined: [0]` rather than merely `attachments: []`, naming the specific declined
    construct ordinal the way `3.3.5/mono-ruby-placement/three-characters-longer-than-the-base`
    already does). Every fixture's per-end surplus stays comfortably under one ruby em, clear
    of paragraph 2's own unimplemented Note.
  - `crates/jlreq-conform/tests/suite.rs` gains a `section_3_3_6` `per_section!` row and a
    second test, `section_3_3_6_is_also_measured_under_flush`, on `section_3_3_5_is_also_
    measured_under_katatsuki`'s exact model: a second `Kumihan::new(Policy)` declaring `ruby.
    group_distribution: flush`, under which the runner's own selection rule picks the `flush`
    entry of every case naming one. Both tests carry an identical `[4 attempted, 0 not
    attempted]` census — `place()`'s own decline conditions are extent comparisons made before
    either alignment question is ever read, so no case becomes unanswerable under either
    policy, and only which permitted entry is selected moves.
  - `docs/conformance-deferrals.toml`: `3.3.6` moves from `[[deferred]]` to `[[owned]]`, its
    `why` naming the four cases, the genuinely-exercised `flush` reading, and the honest scope
    limit — paragraph 3 stays a cased decline rather than an implementation, and paragraph 2's
    own Note stays cased nowhere, because its own parenthesis states two thresholds rather
    than one and closing it needs a policy `Question` or a `docs/decisions/` reading that
    does not exist yet. `3.3.5`'s own `[[owned]]` `why` is repaired to match: the dangling
    promise that task #85 might author a jukugo-shaped replacement for the deleted fourth case
    is resolved, and it resolves to a decline — `crates/jlreq-inline/src/place.rs`'s own
    `RubyStyle::JukugoRuby` dispatch still never reaches `place_mono_run` or `place_group_run`
    and never appears in `Attachments::declined` either, verified against the code rather than
    assumed, so a jukugo-shaped `place` case would assert `attachments: []` alongside
    `declined: []` — satisfiable by an implementation that never implemented anything at all,
    the exact §D.2#4 trap this project's own discipline refuses to publish as coverage. What
    such a case would have asserted belongs to §3.3.7's own deferral instead, not to §3.3.5 or
    §3.3.6.
  - Every stale claim this round falsifies is repaired in place, description text only, no
    field or type changed: `crates/jlreq-conform/cases.schema.json`'s own `place` `$def`, its
    `kind` description and its `declined` property description all once said `Attachments::
    declined` names only a mono-ruby run's own katatsuki-with-overflow choice; all three now
    name group-ruby's own ruby-longer-than-base half too, which this round's own fourth case is
    the first published case to exercise. `crates/jlreq-conform/src/run.rs`'s own `Compose::
    place` doc, `CasePlace`'s own doc and `CasePlace::declined`'s own field doc, and `crates/
    jlreq-conform/src/case.rs`'s own `ExpectPlace` doc and `ExpectPlace::declined`'s own field
    doc, are repaired the same way. `crates/jlreq-inline/src/place.rs`'s own "What is not
    here" section is rewritten to record that §3.3.6 now has a conformance case and moved to
    `[[owned]]`, rather than stating it still does not. `crates/jlreq-conform/src/kumihan.rs`'s
    own module doc is repaired in two places: `place` is credited with §3.3.6's own geometry
    alongside §3.3.5's, and the "every multi-item `place` case this round publishes" and "no
    case this round publishes" sentences are reworded to name the suite rather than a round —
    durable now that `3.3.6.json`'s own second case is this suite's second multi-item `place`
    fixture, and still true that none exercises the separations term of the derivation:
    group-ruby's own base boundary here is independently blank in Table 1 *and* `lower_group`
    itself still emits no `Separation` for a group-ruby run against any neighbor, either fact
    alone already sufficient. `crates/jlreq-inline/src/lib.rs`'s own `# Status` carried the
    identical stale claim as `place.rs`'s "What is not here" section above and was missed in
    the first pass — this round's own review caught it before the gate battery could hide it —
    so it is now repaired the same way, stating that task #85 has since run and named cases and
    moved the rule to `[[owned]]` rather than that this round's own group-ruby geometry is
    implemented but not yet cased.
- M4-a round 7: `RubyStyle::JukugoRuby` placement, both of §3.3.7's own paragraphs, wiring
  `Question::JUKUGO_RUBY_LAYOUT` (task #88). No conformance case authored; §3.3.7 stays
  `[[deferred]]`, on ADR-0006's own discipline that an implementation round does not move
  its own rule to `[[owned]]`.
  - Paragraph 1 ("two or fewer ruby characters per base") delegates each declared run,
    unmodified, to the identical `place_mono_run` a `RubyStyle::MonoRuby` construct itself
    calls — decline included, so a jukugo run whose ≤2-character reading still overflows its
    base under katatsuki declines exactly as an ordinary mono-ruby run does. The ≤2 count is
    read directly off each run's own declared annotation width, a genuine character count
    rather than an extent comparison: unlike §3.3.5(a)-through-(c), paragraph 2's own
    condition is "needs three or more ruby characters," not "is longer than its base," so a
    wide-enough base character could carry three narrow ruby characters without ever
    outrunning it.
  - `crates/jlreq-inline/src/lower.rs`'s own alignment resolution is hoisted to cover
    `RubyStyle::JukugoRuby` alongside `RubyStyle::MonoRuby`: without this, `Contribution::
    alignment_of` would answer `None` for a jukugo construct and `place_mono_run`'s own
    `let Some(alignment) = ... else { return; }` would place nothing at all, silently, the
    moment paragraph 1's own condition held. The `RuleId::POSITIONING_OF_MONO_RUBY_WITH_
    RESPECT_TO_BASE_CHARACTERS` citation stays mono-only — that citation is `crate::place`'s
    to give once it has actually decided paragraph 1 governs a construct, a decision `lower`
    never makes. Settled along the way: §3.3.5's own discouraged-katatsuki-in-horizontal-
    writing flag transfers to a jukugo construct wholesale, on paragraph 1's own delegation to
    "the method described in § 3.3.5" without qualification — §F's own stated assumption of a
    katatsuki baseline governs a different method (the `phonetic` answer, declined below) and
    has nothing to unsettle for a paragraph-1 construct.
  - Paragraph 2 ("attach the ruby text to the kanji compound word as a whole") builds one
    compound-wide synthetic `RubyRun` — the whole declared base range, against the first
    declared run's own annotation start through the last's own end, `Ruby::new`'s own
    `check_runs` contiguity invariant guaranteeing the span is the compound's whole reading —
    and hands it to `place_group_run`, which gains an explicit `jis: bool` parameter in place
    of its own former internal `Question::GROUP_RUBY_DISTRIBUTION` read (moved to its one
    prior call site, `RubyStyle::GroupRuby`'s own arm in `place`, so that style's own
    behavior is unchanged, byte for byte). `Question::JUKUGO_RUBY_LAYOUT`'s own `group`
    answer passes `true` unconditionally — forcing `jis` regardless of the document's own
    `Question::GROUP_RUBY_DISTRIBUTION` answer — the published reading of a genuinely
    unstated question (`docs/decisions/jukugo-group-layout-distribution.md`): §3.3.6 itself
    names exactly one of its own two methods "the method specified in JIS X 4051," twice, and
    never its own "another way"; §3.3.7¶2's own "the layout as specified in JIS X 4051" cites
    that identical, specific method, and its own "which is similar to the group-ruby method
    described in § 3.3.6" is a comparison orienting the reader, not a second instruction
    reopening the choice the first clause already closed by name. Reusing `place_group_run`
    reuses its own ruby-longer-than-base decline too — the jukugo analogue of §3.3.6
    paragraph 3's own base-spreading blocker, structurally unclosable from `place` for the
    identical reason group-ruby's own half is. `Question::JUKUGO_RUBY_LAYOUT`'s own
    `phonetic` answer declines every compound it reaches, unconditionally: §F's own
    phonetic-structure distribution is not implemented this round, not one part of it.
  - A jukugo compound's own base range can straddle one `place` call's own `items` in a way
    `RubyStyle::GroupRuby`'s own base range structurally cannot — §C.2#8's own second
    sentence permits a break between two base characters of one jukugo complex, and
    `lower_jukugo` gives the compound one shared `RunId` but a *fresh* `GroupId` per base
    item precisely so that break survives (`docs/decisions/jukugo-ruby-unset-group.md`'s own
    reading of `same_run_refusal` is what confirms `jlreq-line` actually permits it). Such a
    straddle declines rather than silently skipping the way an ordinary out-of-range
    group-ruby run does: paragraph 2's own "as a whole" instruction has no whole left to
    attach once the line has split the compound, and JLReq states no method for that case. A
    compound split across two lines is consequently declined twice, once by each partially-
    covering `place` call — the correct per-line answer, not a double-report defect. This
    decline is unit-test-only observable for this suite, permanently: `Compose::place`'s own
    adapter always derives `items` as the case's whole declared base stream, so no
    conformance case can ever construct the straddle at all.
  - `Attachments::declined` widens from two stated reasons to four: §3.3.5(c)'s own
    katatsuki-with-overflow choice and §3.3.6 paragraph 3's own base-spreading method each
    now also catch a jukugo construct routed through the identical code, alongside the two
    new jukugo-only reasons above. Its own doc, `crate::place`'s own module doc, `crates/
    jlreq-conform/cases.schema.json`'s `kind`/`lower`/`place` descriptions, and `crates/
    jlreq-conform/src/case.rs`'s `ExpectLower`/`ExpectPlace` docs are all repaired to state
    the new count rather than the old one.
  - `docs/decisions/jukugo-group-layout-distribution.md`, a new published reading
    (`Standing::Unstated`) as argued above, with a matching `docs/decisions/README.md` row.
    Confirmed direction-independent; no `docs/direction-sites.toml` entry follows, though the
    existing `jlreq-inline`/`lower`/`3.3.5` entry gains one clause noting its read now also
    resolves a jukugo construct's alignment, the identical code path rather than a second one.
  - Four new `#[cfg(test)]` cases in `crates/jlreq-inline/src/place.rs`: a paragraph-1
    compound placing per base under both alignments; a paragraph-2 compound placing as one
    `jis`-weighted group, measured under *both* `Policy::JLREQ` and a policy answering
    `flush` for `Question::GROUP_RUBY_DISTRIBUTION` to make the forcing itself observable
    (base 2000, reading 1600 over one-then-three ruby characters, surplus 400 dividing `jis`'s
    own eight-unit weight sum exactly — offsets `[50, 550, 1050, 1550]` under either policy);
    the same compound declined under a `phonetic`-answering policy; and the same compound
    declined again with an `items` range covering only its first base item, exercising the
    straddle no conformance case can reach.
  - Every stale "unfilled slot" or "two reasons" claim this round falsifies is repaired:
    `crates/jlreq-inline/src/lower.rs`'s own module doc, `Lowered::alignments` and `Lowered::
    declined`'s own field docs, `Contribution::alignment_of` and `Contribution::
    alignment_discouraged`'s own docs, `lower`'s own doc, `lower_jukugo`'s own doc and its
    "Recording §3.3.7 here..." comment, `two`'s own doc, and a test assertion message that
    described `lower` as computing no discrimination for a reason no longer accurate (it
    computes none; `place` now does, and neither records a `RuleId`, for the reason `crate::
    lower`'s own module doc already argues for §3.3.6); `crates/jlreq-inline/src/ruby.rs`'s
    own `RubyStyle::JukugoRuby` doc; `crates/jlreq-inline/src/lib.rs`'s own `# Status`;
    `docs/design/api-spine.md`'s own `Attachments` sketch; `crates/jlreq-conform/cases.
    schema.json`'s four spots named above; and `crates/jlreq-conform/src/case.rs`'s two.
    `docs/conformance-deferrals.toml`'s own `3.3.7` entry is rewritten on §3.3.6's own
    round-5-through-6 precedent — the blocker is now the absence of a case naming `3.3.7`,
    not the absence of an implementation — stating exactly what landed and exactly what did
    not; `F`, `F.1`, `F.2`, `F.3` and `F.4`'s own entries each gain a clause distinguishing
    "`jlreq-inline` places jukugo ruby" from "applies §F's own distribution," so their own
    unchanged wording cannot be misread as claiming §F landed; `3.3.2`'s own entry, which
    cited `Question::JUKUGO_RUBY_LAYOUT` as an unfilled slot `lower`'s own module doc named,
    is corrected to name §F alone, now that the question itself is real, read by `place`.
- M4-a round 8: the §3.3.7 jukugo-ruby placement conformance cases (task #90), ADR-0006's own
  separately-authored phase for M4-a round 7's own implementation, closing coverage at 75/106
  inventoried rules (up from 74/106). No logic change to `crates/jlreq-inline/src/place.rs` —
  every number below was derived by hand from §3.3.7's own two paragraphs and this round's own
  fixture advances, never read out of the implementation, its `#[cfg(test)]` module or a debug
  run.
  - `crates/jlreq-conform/cases/3.3.7.json`, three cases naming rule `3.3.7`. The first two
    share the identical base (`亜亜`, two 720-unit cl-19 items, the cl-19/cl-19 boundary
    independently verified blank at `spec/captured/table1.en.tsv` line 485) and the identical
    four-character reading (`かかかか`, 300 units each), differing in exactly one declared
    field — how `runs` partitions the reading across the two base characters — which isolates
    §3.3.7's own discriminator as a ruby-character *count* per base character rather than the
    extent comparison §3.3.5(a)-through-(c)'s own three cases reduce to:
    `jukugo-ruby-placement/paragraph-one-per-base-mono-delegation` (2 and 2, paragraph 1,
    delegating per run to `place_mono_run` under both of `Question::RUBY_ALIGNMENT`'s
    answers, sized so neither run outruns its base and re-cases task #81's still-open choice)
    and `jukugo-ruby-placement/paragraph-two-whole-compound-attachment` (1 and 3, paragraph 2,
    the whole compound attached as one `jis`-weighted unit — offsets `[30, 390, 750, 1110]`,
    the identical arithmetic `3.3.6/group-ruby-placement/jis-versus-flush-distribution`'s own
    rationale already derives, reused here as §3.3.7¶2's own forced reading — over three
    `permitted` entries with totally-ordered key sets: the default `jis` geometry, a decline
    under `ruby.jukugo_layout: phonetic`, and the *identical* `jis` geometry again under
    `ruby.jukugo_layout: group` with `ruby.group_distribution: flush` — `decision:jukugo-
    group-layout-distribution`'s own forcing, published as a named contradiction of the
    expectation a reader would otherwise form at this non-zero surplus, where `jis` and
    `flush` genuinely diverge for an ordinary group-ruby run; the file's own first entry,
    matching every policy, would already assert the identical numbers under a flush-declaring
    policy even without this third entry, so its own second-run test is not itself proof the
    third entry was selected, unlike the `phonetic` run's). The third,
    `jukugo-ruby-alignment/katatsuki-discouraged-
    carries-through-the-delegation`, is a `lower` case for the one fact no `place` case can
    observe — `Contribution::alignment_discouraged` for a jukugo construct in horizontal
    writing — and asserts `rules: ["3.3.4"]`, not `["3.3.5"]` or `["3.3.7"]`: `lower`'s own
    alignment-hoist records §3.3.5's citation only under an explicit mono-ruby style guard, so
    a jukugo construct's own `lower` answer publishes only `RuleId::CHOICE_OF_SIDES_FOR_RUBY_
    WITH_RESPECT_TO_BASE_CHARACTERS` (§3.3.4), never §3.3.7, which belongs to `place` once it
    has actually decided which paragraph governs.
  - `crates/jlreq-conform/tests/suite.rs` gains a `section_3_3_7` `per_section!` row and three
    second-run tests — `_is_also_measured_under_phonetic`, `_under_flush` and `_under_
    katatsuki` — on `section_3_3_5_is_also_measured_under_katatsuki`'s and `section_3_3_6_is_
    also_measured_under_flush`'s exact model: a second `Kumihan::new(Policy)` apiece, under
    which the runner's own selection rule picks the entry naming that question rather than
    `{}`. All four runs (including the default) carry an identical `[3 attempted, 0 not
    attempted]` census — none of `place`'s own decline conditions or `lower`'s own alignment
    resolution reads a policy this file's three cases do not already publish an entry for, so
    only which permitted entry is selected moves.
  - `docs/conformance-deferrals.toml`: `3.3.7` moves from `[[deferred]]` to `[[owned]]`, its
    `why` naming the three cases, the three genuinely-exercised readings, and the honest scope
    limit — §F entire (§F.1 through §F.4) stays uncased because it stays unimplemented,
    paragraph 2's own fourth-sentence two-threshold overhang ceiling stays a declared slot
    doubly moot behind the `phonetic` decline, `lower_jukugo`'s own absent `Separation` for a
    jukugo compound's surplus is stated in both `place` cases' own rationale, and the
    straddled-compound decline stays unit-test-only observable because `Compose::place`'s own
    adapter always derives `items` as a case's whole declared base stream, so this round did
    not spend effort hunting for a fixture that cannot exist. The `3.3.2` and `F` entries'
    own "`3.3.7`'s own entry above" pointers are corrected to "below," now that `3.3.7` sits
    in `[[owned]]`, past the `[[deferred]]` table both entries live in.
  - Every stale "task #90 has not yet run" claim this round falsifies is repaired in place,
    prose only, no field or type changed: `crates/jlreq-inline/src/lib.rs`'s own `# Status`,
    `crates/jlreq-inline/src/place.rs`'s own "What is not here" section, and `docs/decisions/
    jukugo-group-layout-distribution.md`'s own closing section (which had promised task #90
    would publish exactly the `flush`-forcing case this round's second `place` case's own
    third `permitted` entry now does) are all rewritten to state that the phase has run, name
    the cases it published, and state the same honest residue the ledger's own new `why`
    states.

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
  Japanese, and §3.1.6's fourth Note, whose English leaves a cross-reference as the literal
  placeholder the Japanese resolves to §B. Which Note it is was an unmeasured ordinal until
  the detector counted them.
- **§D.2 note 5 is not a contradiction, and this project said it was.** The note gives the
  middle-dot conditional space the third priority in Table 3 where notes 1 to 3 give it the
  fourth, and §3.8.3 lists the line-end reduction and the mid-line one as separate steps: note
  5 is the first and notes 1 to 3 are the second. What is defective is one locale of one
  sentence — note 5's English half drops the 行末に配置する its Japanese half states — so the
  row is `d2-note-5-line-end-qualifier-omitted-in-english` and not
  `d2-note-5-priority-contradiction`. `generation.md` had pre-committed the rule to
  `Standing::Adjudicated` and `conformance.md` had it as a worked example of a case carrying
  both readings; a case written to either would have published an alternative JLReq does not
  permit. ADR 0009, `api-spine.md`, `generation.md` and `conformance.md` are corrected.
- Classification narrows on one more axis, and the axis is Appendix A's own. Where a key is
  listed under several classes and the caller has declared the frame, a Remarks cell that
  states that frame is describing this occurrence and a cell that states none is describing a
  different one — which is what makes `proportionally-spaced` mean anything for the 469 keys
  §A.27 shares with a lower-numbered class, and which Appendix A prints in its Character
  column too, for the 92 keys where exactly one listing is qualified: （ against `(`, ％
  against `%`. Without it a declared frame was read only against §3.1.2's five classes, so a
  proportional `U+0028` answered cl-01. The rule reproduces §3.1.3's and §3.2.6's three stated
  answers for a European numeral — full-width cl-19, half-width cl-24, proportional cl-27 —
  without being told them, and §3.2.6's Note is now read for the cl-24 arm it states in so
  many words rather than for the cl-27 arm alone.
- A narrowing may no longer answer a question nobody asked. Removing §3.1.2's five classes on
  a proportional advance had left `U+3014` alone in cl-28 and told the caller that JLReq had
  decided their bracket surrounds a warichu (割注); a removal whose survivors are all
  membership in a construct that no Remarks cell states the declared frame for is refused, and
  `AxisSet::CONSTRUCT` reports the axis nobody supplied.
- `docs/decisions/ambiguous-context.md` publishes the tie-break the implementation had always
  applied: the lowest-numbered surviving class **the supplied facts can reach**, passing over
  membership in a construct the caller never declared. Nine of the thirty classes are such
  memberships and four are numbered below cl-27, so the unqualified wording answered "inside a
  unit symbol" for every proportional Latin letter in a Japanese document. Two conformance
  cases were written against the wording before the correction, which is the measurement: a
  published reading an implementer cannot reproduce from the document is the defect.
- `docs/design/conformance.md` is written in the tense the code is in. There is no `judge`
  binary, no `answers.schema.json`, no `answers/` and no `src/bin`; every sentence describing
  them now says so, at the top of the document rather than four hundred lines down, and ADR
  0006's ecosystem claim is stated as not yet met. The three ADR 0018 input refusals are
  likewise not published as cases, because the format has no way to say that an input is
  expected to be refused — a requirement on the format first, held meanwhile by
  `jlreq-class`'s own tests over `Text::new`.
- ADR 0018's two `input` properties are checked by `jlreq-conform`'s own test rather than by
  `conform --check`. `Text::new` *is* that reader, and a second reader inside a gate that does
  not carry Appendix A would be a second answer to a question that already has one; the two
  had already parted, which surfaced when the first case the gate accepted and the constructor
  refused reached the runner.
- `spec/derived/defects.tsv` is derived rather than captured. `generation.md` had put it on
  the captured side on the reasoning that most of its rows are defects of the matrices;
  measured, not one of the twelve is — every one is a property of the HTML snapshot.
- `Report` carries `unselectable`, the count of permitted entries no declared policy of a run
  could select. A published reading nothing can select is evaluated by nothing, and the number
  is what stops that being a silence on a green run.
- **The M0 policy-space entry above is stale, and this is the correction rather than a silent
  edit of it.** "The twenty-one places" is twenty-two, and "Stage 2 ... is still to come:
  `Question::ALL` remains empty" is no longer true — see this milestone's Added entries for
  what stage 2 generated and what `jlreq-class` and `jlreq-spacing` now read from it.
- `crates/jlreq-conform/tests/suite.rs`'s `UNSELECTABLE` fell from 170 to 0. Every permitted
  reading a published case names was, at M0, a reading naming a question the policy space did
  not have yet; now that `jlreq_spec::QUESTIONS` holds all twenty-two, every one of those
  readings is a `Choice` this workspace can evaluate, and the count that used to state how
  much of the suite nothing could measure now states that nothing is in that position.
- The five conformance-case declarations that named a Table 6 citation before anything read
  it — `E.2.json`'s `E.2/em-dash-then-horizontal-ellipsis/two-kinds-open-a-third-stage-
  quarter-em` (`"rule": "E.2#4"`, cl-08 x cl-08) and `E.2/western-character-then-postfixed-
  abbreviation/the-general-rule-opens-a-third-stage-quarter-em` (`"rule": "E.2#10"`, cl-27 x
  cl-13), and `E.json`'s `E/dividing-punctuation-then-western/the-boundary-carries-an-
  independent-reduction-and-expansion`'s three `permitted` entries (`"rule": "E"`, cl-04 x
  cl-27) — were audited against `crates/jlreq-spacing/src/generated/table6.rs`'s own rows at
  those exact coordinates now that `check_expansion` reads them. All five agree with the
  generated cell's own `rule` field: `(8, 8)` cites `RuleId::E_2_NOTE_4`, `(27, 13)` cites
  `RuleId::E_2_NOTE_10`, and `(4, 27)` cites the bare `RuleId::OPPORTUNITIES_FOR_INTER_
  CHARACTER_SPACE_EXPANSION_DURING_LINE_ADJUSTMENT` (rendered `"E"`, the same generic
  citation an unnoted Table 1 cell renders `"B"` and an unnoted Table 2 cell renders `"C"`).
  No case needed correction and no generator defect was found.
- `docs/conformance-deferrals.toml`'s own `E.2#8`, `E.2#9` and `E.2#11` entries are rewritten.
  Their stated blocker — a coordinate answering `Expansion::None` being "indistinguishable
  from a bare absence" — no longer holds: `Boundary::expansion_rule()` now reads
  `Some(RuleId::E_2_NOTE_8)` at cl-24 x cl-13, `Some(RuleId::E_2_NOTE_9)` at cl-24 x cl-27,
  and `Some(RuleId::E_2_NOTE_11)` at cl-27 x cl-27, in every case regardless of what
  `expansion()` itself answers there. All three stay `[[deferred]]` at M1, because publishing
  a citation is not the same act as authoring the case that measures it (ADR 0006's own phase
  split) — no case is added and none is moved to `[[owned]]` by this entry. `E.2#11`'s own
  rewritten entry additionally records that whether its own alternative reading is worth a
  case at all is still an open question for that later phase, not answered here: §3.8.4 step
  (d)'s own Note calls the alternative 処理系定義 (implementation-defined) under JIS, so a case
  asserting `kind: "none"` there might measure this workspace's own bookkeeping rather than
  anything JLReq itself requires.
- `docs/conformance-deferrals.toml`'s own `E.2#8` and `E.2#9` entries move to `[[owned]]`,
  naming the cases added above and the percent-sign scope limit as a stated fact rather than
  an open question. `E.2#11`'s own entry stays `[[deferred]]`, and its "why" is rewritten
  rather than left pointing at a future round: the decision is taken this round, and no case
  is authored. The two rejected-alternative coordinates read alike at first — both `E.2#8`
  and `E.2#9` state a captured `limit: None` default with a note offering an unselectable
  alternative — but `E.2#11`'s own alternative is JLReq's fourth, residual expansion stage
  with no stated ceiling, and §3.8.4 step (d)'s own Note (the only other sentence anywhere in
  the document discussing a fourth-order opportunity at cl-27-against-cl-27) attributes that
  residual stage to a JIS X 4051 provision JIS itself calls 処理系定義 — a genuinely different
  kind of silence from E.2#8's and E.2#9's own concrete, merely-unselectable ceilings, and
  the entry's own "why" now says so instead of naming the judgment as still open.
- `E.2/quantity-symbol-then-postfixed-abbreviation/a-declared-role-withdraws-the-opportunity`'s
  expectation gains `rule: "E.2#10"` beside `kind: "none"`: `note_governed_expansion`'s own
  doc and its literal `RuleId::E_2_NOTE_10` for this coordinate, corroborated independently
  by Table 6's own `(27, 13)` cell carrying the identical citation, are what the note's own
  denial cites even while withdrawing the opportunity it would otherwise state — a
  strengthening the specification itself warrants, not a correction made to match code. No
  other field of this case, and no other of `E.2.json`'s pre-existing four cases, changes.
- `conformance-cases-agree-with-the-cells` (ADR 0006) now runs: `xtask::attest` reports 16 of
  18 registered invariants running, up from 15. A boundary case may declare which captured
  cells it exercises through `cells`, a new case-level, optional, list-valued field of
  `{table, before, after}` objects — `crates/jlreq-conform/cases.schema.json`'s own
  `matrix_cell`, validated by `conform`'s own `check_cells` and added to `CASE_OPTIONAL`.
  Deliberately not the `address` grammar's `@` suffix: §D.1 is the legend of three matrices
  at once ("Legend of Tables 3, 4 and 5"), so `D.1@cl-02,line-end` never named one captured
  cell, and `spec/derived/rules.tsv` does not inventory the natural per-table alternative
  either — it has `D.1` but never `B.1`, `C.1` or `E.1`. The checker (`Evidence`, threaded
  through `Check::Whole` and `Check::Partial` uniformly rather than bolted onto `Capture` or
  carried by a nineteenth `Check` variant) asserts existence — every declared coordinate is
  one the agreed transcription has, at every table alike — and, for Table 1 alone, that a
  case's default-policy (`policy: {}`) boundary answer agrees in units with the captured
  cell. 21 of the suite's 72 boundary cases now declare a coordinate this way — every
  `B.json` and `B.2.json` case, `D.1.json`'s one case, every `D.2.json` case, and `E.json`'s
  and `E.2.json`'s cases, 43 coordinates in all, derived from each case's own quote and
  rationale rather than read back off the transcription (ADR 0006) — and the run reports
  zero disagreements. The remaining 51 boundary cases (`C.json`'s and `C.2.json`'s own Table
  2 coordinates, which carry no amount to compare, and every `A.*` and `3.x` boundary case,
  whose own coordinate a checker here would have to derive by classifying `text` — a second
  implementation of Appendix A, which ADR 0019 forbids) are the invariant's own named
  remainder rather than a silence. `conform --check`'s own census is unchanged apart from a
  new line reporting the count declared; declared coverage, the rule and address counts, and
  every other number stay 56 files, 466 cases, 69 rule addresses, 373/72/10/2/9 by kind,
  69 owned / 37 deferred / 0 uncovered.
- Two stale claims this invariant's own absence had left standing are repaired. `xtask::
  attest`'s module doc no longer states that `B.1@cl-02,line-end` is a working matrix-cell
  address — `B.1` is not an inventoried rule any more than `D.1` is, and the corrected
  example, `B@cl-05,cl-05`, is the one `docs/design/address-corpus.tsv` actually validates.
  `docs/design/conformance.md` no longer attributes the absence of table cells from
  `spec/derived/rules.tsv` to `spec/captured/` being empty — the matrices have been
  transcribed since the round that landed them; the real reason `covers` still has no user
  is that `derive` has never been extended to walk a matrix into rule addresses at all,
  independent of whether the transcription exists.
