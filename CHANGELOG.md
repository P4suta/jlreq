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
