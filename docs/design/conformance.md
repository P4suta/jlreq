# The conformance suite

[ADR 0006](../adr/0006-conformance-suite-as-artifact.md) makes `jlreq-conform` a
deliverable rather than a test directory, and says the artifact is worth more than the
implementation. This document is the format, the trait, and the gates.

Every decision below was made by asking what a browser engineer or a Typst maintainer
needs, not what is convenient for our tests. Three of them are worth naming up front,
because they are what makes the suite usable by someone who will never depend on us.

The trait returns `Option`, and `None` means *not attempted* rather than *failed*. Chrome
will implement the boundary question and will never expose anything resembling our
classification; Typst will implement composition and nothing else. Under a non-optional
trait both would score as catastrophic failures and the suite would be discarded as
hostile by exactly the people it exists to serve.

There is a `judge` binary. An implementation in any language emits a JSON answers file and
runs one command; no Rust, no FFI, no build integration. That, and not the trait, is what
makes this an ecosystem artifact.

A boundary expectation carries the conditional spaces, not their sum
([ADR 0014](../adr/0014-the-conditional-space-is-the-unit-of-spacing.md)). An
implementation that assigns the two quarter ems of a middle-dot pair to the wrong owners,
or drains them in the wrong priority order, produces the same total and would score as
agreeing under a scalar format.

A line expectation states three quantities whose definitions are pinned by
[ADR 0017](../adr/0017-normalized-line-geometry.md) and are the same words the library
uses: `placements` are the caller's own glyph-box origins on the line's own inline axis,
`extent` is the line in normalized geometry, and `trailing` is the realized conditional
space at the line end whether or not it lives inside the last item's supplied advance.
Nothing here is reconstructed by the runner from something else, because two runners would
reconstruct it differently. A fourth, `parts`, carries the sub-lines of any segment on the
line; it is absent from most cases and is the only place a non-inline offset appears.

## Format and layout

JSON, one file per JLReq section, plus a committed JSON Schema. JSON because it is the one
format every ecosystem parses with no dependency decision — a Node harness, a Python
script, a `serde_json` a Rust project already has, a C++ engine's vendored parser — and
because its integer grammar is exact, which is the whole reason
[ADR 0005](../adr/0005-integer-layout-units.md) chose integers. TOML would have been nicer
for us and would have pushed a parser choice onto everyone else; that is optimizing for the
maintainer over the audience.

`jlreq-conform` reads it with its own reader and takes no outside dependency. The reason is
the same one: the crate is the artifact a browser engineer runs, and it should not hand them
a proc-macro chain, a `bans.multiple-versions = "deny"` conflict, or an MSRV negotiation to
run a test suite. The subset is small and unusually safe to own, because
[ADR 0005](../adr/0005-integer-layout-units.md) already guarantees that every number in a
case is an integer inside 2^53 — the genuinely hard part of JSON is the part this format
does not contain. The schema stays committed, so nobody else has to use our reader.

JSON has no comments, and that is a forcing function in the right direction. ADR 0006
requires publishing our disagreements, and a published disagreement must be machine
readable to appear in a report. So `rationale`, `quote`, and `disagreements` are **fields**,
not comments, and they show up in output.

```text
crates/jlreq-conform/
  cases.schema.json         the committed schema
  cases/
    3.1.2.json              one file per JLReq section
    3.1.9.json
    3.3.8.json
    A.19.json
    B.2.json
    C.3.json
    ...
  answers/                  example answers files for the `judge` path
  src/
    lib.rs                  Compose, Case, Suite, Report, load, run, judge
    bin/jlreq-conform.rs    the `run` and `judge` commands
```

A case id is `<section>/<subject>/<variant>`, unique across the suite:
`3.1.9/closing-bracket-at-line-end/half-em-frame`. One `#[test]` is generated per **file**,
not per case: the nextest profile reports a test slow at 10 s and terminates it at 60 s
with `flaky-result = "fail"`, so a sweep over all 1133 Appendix A keys must be split by
section rather than run as one test.

Case files are matched by `**/tests/fixtures/**` in `.gitattributes` for line endings only
if placed there, which they are not — they are a published deliverable, not a fixture — so
`cases/**` gets its own `REUSE.toml` annotation block and its own `.gitattributes` entry
marking it `text eol=lf`, and every file is written with LF.

## A worked case

This is the case that demonstrates the most consequential correctness point in the design.
§3.1.2 states that the character advance of a closing bracket (cl-02) is half-width, and
that Table 1's amount is what "makes them appear as if they were intrinsically full-width".
A modern font reports a full em for the same glyph. So the same geometry is reached from
two directions, the caller says which by declaring the frame (字幅), and both readings must
be recorded — which is exactly what ADR 0006 means by recording every permitted outcome.

The pair is a differential test, and that is what makes it worth being the worked example.
Because composition normalizes to the specification's geometry and reports the trim
(ADR 0017), the two frames produce **byte-identical** expectations: the same placements,
the same trailing amount, the same extent. The only difference anywhere in the two cases is
the input frames and the `trims` array, which is precisely the fact being asserted. An
implementation that adds Table 1's half em to an advance that already contains it passes
the first case and fails the second on `extent`; an implementation that shortens the
caller's advance without saying so passes both `extent` checks and fails on `trims`.

The paragraph composes to one line, which is therefore also the last line, so §3.8.1's
Note exempts it from justification and both line-end conventions are observable. Nothing in
the input says so: a case supplies only what
[api-spine.md](api-spine.md)'s `Paragraph` accepts, and whether a line is last is something
composition decides.

```json
{
  "$schema": "../cases.schema.json",
  "section": "3.1.9",
  "heading_en": "Positioning of Closing Brackets, Full Stops, Commas and Middle Dots at Line End",
  "heading_ja": "行末に配置する終わり括弧類，句点類，読点類及び中点類の配置方法",
  "cases": [
    {
      "id": "3.1.9/closing-bracket-at-line-end/half-em-frame",
      "rules": ["3.1.2", "3.1.9", "B.2#2"],
      "standing": "alternative",
      "quote": "In principle, closing brackets (cl-02), commas (cl-07) or full stops (cl-06) at the line end have half em spacing after them. This half em spacing can be removed for line adjustment. However, the possibilities are only half em spacing or solid. Other spacing, such as quarter em spacing should not be used.",
      "rationale": "The caller declares the half em frame, so the conditional space after the bracket is added to the supplied advance. JLReq keeps it at the line end and JIS X 4051 sets solid; both are permitted and the intermediate quarter em is forbidden by the quoted sentence, which states the prohibition twice.",
      "input": {
        "kind": "compose",
        "text": "あい」",
        "direction": "horizontal",
        "scales": [{ "inline_em": 1000, "block_em": 1000 }],
        "items": [
          { "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 },
          { "start": 3, "advance": 1000, "frame": "full-em", "scale": 0 },
          { "start": 6, "advance":  500, "frame": "half-em", "scale": 0 }
        ],
        "candidates": [{ "at": 0 }, { "at": 3 }, { "at": 6 }, { "at": 9 }],
        "measure": 3000
      },
      "permitted": [
        {
          "policy": {},
          "source": "JLReq preferred (B.2#2)",
          "expect": {
            "lines": [
              {
                "placements": [0, 1000, 2000],
                "trims": [],
                "trailing": { "em": [1, 2], "units": 360, "resolved": 500 },
                "extent": 3000
              }
            ],
            "violations": []
          }
        },
        {
          "policy": { "spacing.line_end_punctuation": "solid" },
          "source": "JIS X 4051 (3.1.9, Figure 77)",
          "expect": {
            "lines": [
              {
                "placements": [0, 1000, 2000],
                "trims": [],
                "trailing": { "em": [0, 1], "units": 0, "resolved": 0 },
                "extent": 2500
              }
            ],
            "violations": []
          }
        }
      ],
      "forbidden": [
        {
          "expect": { "lines": [{ "trailing": { "em": [1, 4] } }] },
          "why": "3.1.9: 'the possibilities are only half em spacing or solid. Other spacing, such as quarter em spacing should not be used.'"
        }
      ],
      "disagreements": []
    },
    {
      "id": "3.1.9/closing-bracket-at-line-end/full-em-frame",
      "rules": ["3.1.2", "3.1.9", "B.2#2"],
      "standing": "normative",
      "quote": "The character advance of commas (cl-07), full stops (cl-06), opening brackets (cl-01), closing brackets (cl-02) and middle dots (cl-05) is half-width (half em). But when those punctuation marks are placed side-by-side with ideographic (cl-19), hiragana (cl-15), or katakana (cl-16) characters, in principle, a given amount of spacing will be inserted before or after the symbols, which makes them appear as if they were intrinsically full-width (one em).",
      "rationale": "The identical text, with the bracket declared on the ideographic frame — the advance a modern OpenType font reports. The conditional space is already inside that advance, so composition trims it out and reports the trim, and the line is stated in the same normalized geometry as the half-em case. Every expected value below is therefore byte-identical to the case above and only `trims` differs, which is the whole assertion: an implementation that adds the Table 1 amount to a full-em advance overshoots by half an em at the commonest adjacency in Japanese text, and one that shortens the advance silently produces the right extent with no evidence.",
      "input": {
        "kind": "compose",
        "text": "あい」",
        "direction": "horizontal",
        "scales": [{ "inline_em": 1000, "block_em": 1000 }],
        "items": [
          { "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 },
          { "start": 3, "advance": 1000, "frame": "full-em", "scale": 0 },
          { "start": 6, "advance": 1000, "frame": "full-em", "scale": 0 }
        ],
        "candidates": [{ "at": 0 }, { "at": 3 }, { "at": 6 }, { "at": 9 }],
        "measure": 3000
      },
      "permitted": [
        {
          "policy": {},
          "source": "JLReq preferred (B.2#2)",
          "expect": {
            "lines": [
              {
                "placements": [0, 1000, 2000],
                "trims": [
                  { "item": 2, "em": [1, 2], "units": 360, "resolved": 500, "referent": "preceding", "rule": "3.1.2" }
                ],
                "trailing": { "em": [1, 2], "units": 360, "resolved": 500 },
                "extent": 3000
              }
            ],
            "violations": []
          }
        },
        {
          "policy": { "spacing.line_end_punctuation": "solid" },
          "source": "JIS X 4051 (3.1.9, Figure 77)",
          "expect": {
            "lines": [
              {
                "placements": [0, 1000, 2000],
                "trims": [
                  { "item": 2, "em": [1, 2], "units": 360, "resolved": 500, "referent": "preceding", "rule": "3.1.2" }
                ],
                "trailing": { "em": [0, 1], "units": 0, "resolved": 0 },
                "extent": 2500
              }
            ],
            "violations": []
          }
        }
      ],
      "forbidden": [
        {
          "expect": { "lines": [{ "extent": 3500 }] },
          "why": "Adding the Table 1 half em to an advance that already contains it. 3.1.2 states the bracket's own advance is half-width and that the amount is what makes it appear full-width, so it cannot be both."
        },
        {
          "expect": { "lines": [{ "extent": 2500, "trims": [] }] },
          "why": "Trimming the caller's advance and not saying so. ADR-0002 makes the supplied advance the caller's; a unit taken out of one is reported with the sentence that took it."
        }
      ],
      "disagreements": []
    }
  ]
}
```

## Streams and constructs in the input

The `input` object carries the base stream shown above, and from M0 it also carries the two
things a construct case needs. Both are in the schema from the first case file, not added at
M4, because the shape of `input` is the part of this artifact that cannot be revised without
rewriting every case already published.

`annotations` is a list of further streams, each with the same `text`, `scales` and `items`
shape as the base one, because annotation text is a second stream and not a range of the
first ([ADR 0016](../adr/0016-annotation-text-is-a-second-stream.md)). `constructs` names
them: a ruby entry gives a base range in the base stream, an annotation index, and the run
pairing; an emphasis entry gives a base range, a symbol and the symbol's advance, and no
stream at all, because §3.3.9 fixes the size and the side and repeats one mark.

The annotation stream is where the **ruby size** is stated, and it is the only place it is
stated. There is no `size` key on a ruby entry and no `ruby.size` policy question, because
§3.3.3 does not close the set — for headings at twelve points or more it says the ruby "is
generally smaller than half the size of the base characters" with no ratio at all — and
because the caller shaped the reading at some size and measured it there
([ADR 0019](../adr/0019-one-fact-one-carrier.md)). §3.3.3's two named sizes are exercised by
cases that declare the corresponding `scales`, which is also how its anisotropy is pinned: a
one-third-ruby case declares `inline_em` a third of the base and `block_em` a half, and a
correct implementation produces both of §3.3.3's physically-stated sentences from that one
declaration.

```json
"annotations": [
  { "text": "かん", "scales": [{ "inline_em": 500, "block_em": 500 }],
    "items": [ { "start": 0, "advance": 500, "scale": 0 },
               { "start": 3, "advance": 500, "scale": 0 } ] }
],
"constructs": {
  "ruby": [ { "base": [0, 1], "annotation": 0, "style": "mono",
              "runs": [ { "base": [0, 1], "annotation": [0, 2] } ] } ],
  "emphasis": [ { "base": [2, 4], "symbol": "FE45", "advance": 500 } ]
}
```

A construct is named in a report by its kind and its position in the array above — the ruby
you wrote third — never by an identity kumihan allocated, because a caller never saw those
([ADR 0015](../adr/0015-the-crate-graph-and-the-inline-line-seam.md)).

An `item` in an expectation is always an ordinal into the stream the surrounding object
names, and the runner rejects a case that indexes one stream with another's ordinal. That
is now genuinely the same invariant the library holds, which it was not in the previous
revision: `ItemIndex` and `AnnotationIndex` are different types, so a swapped `base` and
`annotation` is a compile error inside kumihan and a `conform --check` failure in a case
file ([ADR 0016](../adr/0016-annotation-text-is-a-second-stream.md)). Before, the runner
would have been the only place the rule existed.

Two properties of `items` are checked at gate time because
[ADR 0018](../adr/0018-an-item-is-one-occurrence.md) makes them properties of a well-formed
input rather than of an implementation. Every item must be exactly one Appendix A key —
except a Western ligature, which is several cl-27 keys on the proportional frame — and every
item whose key Appendix A names under cl-01, cl-02, cl-05, cl-06 or cl-07 must declare a
`frame`. A case violating either is malformed, not failing: kumihan would refuse to build the
`Text`, and a case whose input the library rejects tests nothing. This is why `frame` appears
on every item of the worked case below, including the two ideographs, where it is optional
and written out anyway so the file reads as one thing.

Three further details of the format are load bearing.

An amount is written as a **fraction and a unit count**, and `conform --check` asserts they
agree under the current denominator. The published artifact is therefore
denominator-independent: a future change to the 1/720 unit of
[ADR 0007](../adr/0007-two-scalars-and-the-fixed-point-unit.md) is a mechanical
re-derivation of our code rather than a rewrite of the suite. `resolved` is the same
quantity in the case's own caller units, so an implementation with a different internal
unit can compare against something.

Every integer stays inside 2^53, so a JSON parser using IEEE-754 doubles is exact. That is
checked, not assumed.

`policy: {}` is the empty overlay on the JLReq preset, and by the selection rule below it is
a case's fallback entry rather than its "JLReq-only" entry. Policy keys are the stable dotted
paths generated alongside `Question`, so renaming a question breaks the case files at gate
time rather than silently.

A `trims` entry names a rule, and `conform --check` requires that rule to be §3.1.2 or a
Table 1 cell stating a conditional space of that amount with that referent. Without the
check, an implementation could discharge an overlong line by subtracting an arbitrary
quantity from a caller's advance and calling it a trim, which is the one way
[ADR 0002](../adr/0002-caller-supplied-metrics.md) can be evaded while still reporting
everything ([ADR 0017](../adr/0017-normalized-line-geometry.md)).

## Multiple permitted answers

`permitted` is a list of **(policy, expectation) pairs**, not a list of bare expectations.
ADR 0006 says to record every permitted outcome; recording which knob produces each one
makes the suite a specification of the policy surface as well.

### Which entry applies

This is the evaluation semantics of the artifact, so it is stated exactly rather than left
to a reader. A `policy` object is an **overlay**, not an identity: a partial map from
question path to choice name, read against `Policy::JLREQ`.

- An entry **applies** to a declared policy when every question it names has that value in
  the declared policy. `{}` names none, so it applies to every policy.
- The entry that is **selected** is the applying one that names the most questions.

`conform --check` makes that selection unique by a static check on the case rather than by a
runtime tie-break: the key sets of a case's `permitted` entries must be **totally ordered by
inclusion**. Two entries naming disjoint questions are malformed and rejected at gate time,
because a policy setting both would select neither. Under that constraint the
most-specific-applying entry always exists and is always unique.

The earlier wording — "given a policy, exactly one entry applies" — is what this replaces,
and it was self-contradictory for the worked case below. Read as a predicate, `{}` and
`{"spacing.line_end_punctuation": "solid"}` both apply to an implementation declaring
`solid`, so two applied. Read as an identity, `Policy::BOOK` — which differs from
`Policy::JLREQ` in the reduction table and in hanging punctuation, neither relevant to this
case — matched neither, so none applied. Overlay plus most-specific gives the solid entry in
the first reading and the `{}` entry in the second, which is what both answers should be.

That gives the runner two modes, and the second is what turns our test suite into the
ecosystem's:

- An implementation that **declares a policy** is checked against the entry selected for
  that policy. Strict.
- An implementation that **declares nothing** passes if it matches *any* permitted entry.
  This is what makes the suite runnable against a browser or InDesign, which cannot be
  configured to our knobs, and the report says which reading the implementation is closest
  to rather than only that it differs from ours.

`forbidden` records outcomes the specification excludes even though they lie between two
permitted ones — §3.1.9's quarter em is the canonical instance, and it is the value a
naively continuous justifier emits.

A case whose `standing` is `unstated` or `adjudicated` records that JLReq does not decide.
Both readings appear under `permitted` with `source` naming each, and the `rationale` says
what this project does and why. §D.2 note 5 against notes 1 through 3, and §3.1.3's Note
reading "vertical" in English against 横組 in Japanese, are both recorded this way. Nothing
in the format lets a silence be laundered into a requirement.

## Disagreements

```json
"disagreements": [
  {
    "implementation": "LaTeX jlreq class",
    "version": "2024-11-01",
    "behavior": "Sets the line end solid after cl-02 regardless of the configured convention.",
    "our_reading": "B.2 note 2 makes a half em the preferred spacing and solid the alternative, so the convention is a caller choice rather than a fixed answer.",
    "evidence": "docs/decisions/line-end-punctuation.toml"
  }
]
```

Publishing these is uncomfortable and correct. A case saying "we read 3.1.9 this way, this
implementation does otherwise, here is why" is more useful to the ecosystem than a green
suite that hides the question. The field is data rather than prose so it appears in a
report, and the report quotes the specification sentence, so a reader can adjudicate
without reading our source.

## The trait

```rust
/// What an implementation supplies to be measured.
///
/// Three methods, each taking data and returning data. `None` means "this implementation
/// does not attempt this layer" and is reported as skipped, never as a failure: an engine
/// that exposes only line composition scores honestly on line composition.
pub trait Compose {
    fn name(&self) -> &str;

    /// The policy this implementation claims to follow, if any: a total map from question
    /// path to choice name, against which each case's `permitted` overlays are matched by
    /// the selection rule above. An implementation declaring none is checked against every
    /// permitted outcome rather than one.
    fn declared_policy(&self) -> Option<CasePolicy> { None }

    /// The class number, 1 through 30, of one item. JLReq: §3.9.2, §A
    fn classify(&self, input: &CaseInput, item: usize) -> Option<CaseClass>;

    /// The spacing, breakability and placement at one boundary. JLReq: §B, §C
    fn boundary(&self, input: &CaseInput, before: usize) -> Option<CaseBoundary>;

    /// The composed lines. JLReq: §3.8, §D, §E
    fn compose(&self, input: &CaseInput) -> Option<CaseOutput>;
}

/// One case's `input` object, deserialized. The three trait methods share it because a
/// case is one input and several questions about it, which is what lets an implementation
/// answer only the layer it has.
pub struct CaseInput {
    pub kind: String,
    /// The base running-text stream.
    pub text: String,
    pub scales: Vec<CaseScale>,
    pub items: Vec<CaseItem>,
    /// Further streams, each the same shape. Indexed by `annotation` in `constructs`.
    pub annotations: Vec<CaseStream>,
    pub constructs: CaseConstructs,
    pub candidates: Vec<CaseCandidate>,
    pub measure: i64,
    pub direction: String,
    pub first_line_indent: Option<i64>,
}

pub struct CaseScale { pub inline_em: i64, pub block_em: i64 }

/// `frame` is required whenever the item's Appendix A key is named under cl-01, cl-02,
/// cl-05, cl-06 or cl-07, and `conform --check` enforces it: there the frame decides a
/// geometry and an unstated one has no answer to report (ADR-0018).
pub struct CaseItem { pub start: usize, pub advance: i64, pub scale: u8,
                      pub frame: Option<String>, pub role: Option<String> }

pub struct CaseStream { pub text: String, pub scales: Vec<CaseScale>,
                        pub items: Vec<CaseItem> }

/// A classification answer, with the reason if the implementation has one.
pub struct CaseClass { pub class: u8, pub rules: Vec<String> }

/// A boundary answer. The conditional spaces, never their sum (ADR-0014).
pub struct CaseBoundary {
    pub spaces: Vec<CaseSpace>,
    pub breakable: bool,
    pub permitted: bool,
    pub ruby_overhang: Option<CaseOverhang>,
    pub rules: Vec<String>,
}

pub struct CaseSpace {
    pub units: i32,
    /// "preceding" or "trailing" — Appendix B's `be` and `af`.
    pub referent: String,
    /// "rigid", "range", or "discrete" — §3.1.9's two-valued case is not a range.
    pub reduction: String,
    pub floor_units: i32,
    /// "reduction" or "expansion". Appendix D's six steps and Appendix E's four are two
    /// orderings of two different things and §3.8.2 orders the ladders themselves, so a
    /// bare `stage` would mean two things in one field (ADR-0014).
    pub ladder: String,
    pub stage: u8,
}

pub struct CaseOutput { pub lines: Vec<CaseLine>, pub violations: Vec<CaseViolation> }

/// The three quantities are ADR-0017's, and none is reconstructed by the runner.
pub struct CaseLine {
    /// Caller glyph-box origins, **always on the line's own inline axis and relative to
    /// the line's origin**, one per item of the line, with no exceptions. May be negative,
    /// and may run past `extent`.
    ///
    /// A segment's interior items are here like any other. Three of the four interiors run
    /// along this axis; §3.2.5's tate-chu-yoko does not, so its items all carry the
    /// segment's own inline origin and their spread across the line is `CasePart::across`.
    /// A flat array with two coordinate spaces in it would let two implementations
    /// disagree about the convention while both "passing" every case without a segment,
    /// and then neither could tell a wrong answer from a different convention (ADR-0011,
    /// ADR-0017).
    pub placements: Vec<i64>,
    /// The realized conditional space at the line end, inside a supplied advance or not.
    pub trailing: i64,
    /// The line in normalized geometry, excluding anything hung outside the measure.
    pub extent: i64,
    /// Every unit taken out of a supplied advance, with the sentence that took it.
    pub trims: Vec<CaseTrim>,
    /// The sub-lines of every segment touching this line. Empty for a case with no
    /// segment, which is most of them.
    pub parts: Vec<CasePart>,
    pub hanging: Option<i64>,
}

pub struct CaseTrim { pub item: usize, pub units: i32, pub resolved: i64,
                      pub referent: String, pub rule: String }

/// One sub-line of one segment. `inline` and `block` are its origin relative to the line's;
/// `across` is one block offset per interior item and is non-empty only for §3.2.5.
pub struct CasePart { pub segment: usize, pub index: u8, pub items: [usize; 2],
                      pub inline: i64, pub block: i64, pub extent: i64,
                      pub across: Vec<i64> }

pub struct Report {
    pub attempted: usize,
    pub agreed: usize,
    pub disagreed: Vec<Disagreement>,
    pub skipped: usize,
    /// Every rule the run actually exercised. Drives the exercised-coverage gate.
    pub rules_exercised: BTreeSet<String>,
}

pub struct Disagreement {
    pub case: String,
    pub rules: Vec<String>,
    /// The specification sentence, quoted, so the report is readable without our source.
    pub statement: String,
    /// Every outcome the specification permits, so the report says which reading the
    /// implementation is closest to.
    pub permitted: Vec<String>,
    pub got: String,
}

pub fn load(dir: &Path) -> Result<Suite, LoadError>;
pub fn run<C: Compose>(suite: &Suite, implementation: &C) -> Report;
/// Score an answers file produced by any implementation in any language.
pub fn judge(suite: &Suite, answers: &Path) -> Result<Report, LoadError>;
```

```sh
jlreq-conform run   --cases cases/                    # this workspace's implementation
jlreq-conform run   --cases cases/ --section 3.1.9
jlreq-conform judge --cases cases/ --answers mine.json
```

The answers file is one object per case id carrying the same shapes, so an implementation
in any language emits JSON and runs one command. Thirty lines in any language, no build
integration, and it works for a browser driven by a headless harness.

## "Every rule has a case", mechanically

[CONTRIBUTING.md](../../CONTRIBUTING.md) requires that a rule without a case is incomplete.
[ADR 0013](../adr/0013-rules-are-addressed-by-specification-address.md) makes that
arithmetic, because the tables, the doc comments, and the case files all use one address
space. Two gates, because one is not enough.

**Declared coverage**, static, in `cargo run -p xtask -- conform --check`:
`RuleId::ALL` minus the union of every case's `rules` field must be empty. It also checks
that every `rules` entry resolves to a known rule, every case id is unique, every case
validates against the schema, every `permitted` entry's overlay is valid and the entries'
key sets are totally ordered by inclusion, every integer is inside 2^53, every fraction
agrees with its unit count, every `trims` rule is §3.1.2 or a Table 1 cell stating that
amount with that referent, every item is one Appendix A key, and every item whose key
Appendix A names under one of §3.1.2's five classes declares a frame.

**Exercised coverage**, dynamic, as a test in `jlreq-conform`: run every case against this
workspace while accumulating the rules the evaluator reports as fired, and assert the
accumulated set equals `RuleId::ALL`. This catches a case that *names* a rule and never
reaches it — the failure a static gate cannot see, and the reason the declared gate alone
is satisfiable by adding a string to a list.

For the dynamic half to be honest, rules must report from every layer that has them, not
only from the boundary evaluator. Classification reclassifies (§C.2 notes 1 through 3),
line adjustment drains stages (§3.8.3, §3.8.4), and the inline constructs carry the whole
of §3.3. So every layer reports its own and the facade unions them: `Classified` and
`Answer` carry provenance, `jlreq_spacing::rules_fired` reports a boundary's,
`Composition::rules_fired` reports the ladder's, and `Contribution::rules_fired` reports
the constructs'. Nothing is threaded through the no-alloc crates — a mutable trace
parameter would have to cross `jlreq-class` and `jlreq-spacing`, which allocate nothing and
whose answers already carry the rules that produced them.

Table cells are rules, addressed `B.1@cl-05,cl-05`, because most cells implement no note
and a gate over sections alone would be discharged by one case per appendix. `RuleId::ALL`
is consequently several thousand, so a case may be a **family**: one case entry may carry a
`covers` field naming a row, a column, or a class-pair set, and the gate credits every rule
in it. Without families the coverage requirement would demand several thousand hand-written
cases and would be abandoned; with them it stays honest, because a family still has to
*exercise* each cell it claims under the dynamic half.

## Two further gates the format makes free

**Direction parity**, from M1. Every case not marked `direction: "vertical"` or
`"horizontal"` is composed twice, once each way, and the inline results must be
bit-identical. [ADR 0011](../adr/0011-typed-axes-and-direction-as-a-datum.md) names exactly
three direction-conditional rules; a fourth shows up here as a failing case over the whole
corpus rather than as a code review that has to notice it. This is what turns ADR 0004 from
an aspiration checked at M5 into a property proved from M1.

**Cross-search agreement**, from M3. Every case runs under both `Search::FirstFit` and
`Search::Optimal`, which must agree on every single-line case and on the first break of
every paragraph whose first line has a unique feasible answer.

That gate is satisfiable rather than aspirational because of where two things were put.
Hanging punctuation is a stage of the shared `Ladder`, between reduction and expansion, not
a repair the greedy path applies after choosing a break — §2.5.1 says it "is only necessary
… when they would otherwise need to be wrapped to the line head" and that "if possible the
full stops or commas are placed at the line end", which is a fit decision. And §3.1.12 ⑤ is
not a mechanism at all: pulling up is taking the later of two feasible breaks and paying
reduction, pushing down is taking the earlier one and paying expansion, and preferring the
first is §3.8.2 — which `FirstFit` reaches greedily and `Optimal` reaches through
`Preference`. Had either lived in one search and not the other, this gate would fail across
the whole corpus and the fix would be a redesign after `Fit` and `Demerits` were frozen.

**Cross-platform identity** is already free: `just test-ci` runs on Linux, Windows and
macOS, and with integer arithmetic throughout a per-OS difference is a bug, never a
tolerance. That is worth stating in CONTRIBUTING.md so nobody ever "fixes" one with an
epsilon.

## What the suite must contain before a milestone is done

- Every rule in the inventory for that milestone, under both gates.
- Every `(question, choice)` pair in the policy space, so a permitted alternative cannot be
  added to the API and left untested.
- Every recorded defect and every `unstated` or `adjudicated` reading, with all of its
  readings — not only the one this project takes.
- Every cross-table invariant from [generation.md](generation.md), so the transcription's
  redundancy is published rather than private.
- The pairs that collapse two of JLReq's sentences into one rule: one-third ruby's two
  dimension statements (§3.3.3), the ruby side (§3.3.4), the emphasis-dot side (§3.3.9),
  the reference-mark alignment (§4.2.3), and the first-line and last-line escape rules
  (§4.5.1). A correct implementation must *produce* both of JLReq's sentences from the
  single rule, which an implementation with two code paths cannot do without duplicating.
- The frame pair of §3.1.2, whose two cases must have byte-identical expectations except
  for `trims` ([ADR 0017](../adr/0017-normalized-line-geometry.md)). `conform --check`
  asserts that equality directly rather than leaving it to a reader to notice, because a
  case pair whose whole value is an equality is worth checking as one.
- Every segment kind, since one concept serves four constructs and a bug in it is four
  bugs: tate-chu-yoko (§3.2.5), jidori (§3.7.3), warichu within one line (§3.4.2), warichu
  straddling two and three lines (§3.4.3), and furiwake (§3.7.2). §3.4.3's per-line lengths
  are published only as Figures 148 and 149, so they are captured rather than derived and
  the case records that provenance like any other captured value. The tate-chu-yoko case
  additionally pins the coordinate rule: its interior items share the segment's `placements`
  entry and differ only in `parts[].across`, so an implementation that laid the run out
  along the line's inline axis fails on `placements` rather than passing quietly.
- Both readings of a rule the specification permits, keyed as overlays whose key sets form a
  chain, so the selection rule above has a unique answer for every policy an implementation
  might declare.
- The three input refusals of [ADR 0018](../adr/0018-an-item-is-one-occurrence.md), as
  malformed-input cases rather than layout cases: a split `<02E5, 02E9>`, a full-width
  multi-key item, and an unstated frame on `。`. They belong in the suite because they are
  the contract every other case's `input` relies on, and because an implementation that
  accepts them will disagree with every implementation that does not.
