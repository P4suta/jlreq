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

There will be a `judge` binary, and there is not one yet. An implementation in any language
will emit a JSON answers file and run one command; no Rust, no FFI, no build integration.
That, and not the trait, is what will make this an ecosystem artifact — which is why the
state of it is stated here, at the top, rather than four hundred lines down. **At M0 the
crate has no binary, no `judge` and no answers-file schema**, so the only way to run a case
is to implement the `Compose` trait in Rust. Every sentence below that describes the binary
describes what the crate owes and not what it has; `crates/jlreq-conform`'s own crate
documentation says the same thing to anyone who arrives at the code first. ADR 0006's
ecosystem claim rests on this and is therefore not yet met, which is a debt worth stating
plainly rather than a sentence worth writing in the present tense.

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
  answers/                  example answers files for the `judge` path — not written yet
  src/
    lib.rs                  Compose, Case, Suite, Report, load, run; judge not written yet
    bin/jlreq-conform.rs    the `run` and `judge` commands — not written yet
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

Two properties of `items` are checked because
[ADR 0018](../adr/0018-an-item-is-one-occurrence.md) makes them properties of a well-formed
input rather than of an implementation. Every item must be exactly one Appendix A key —
except a Western ligature, which is several cl-27 keys on the proportional frame — and every
item whose key Appendix A names under cl-01, cl-02, cl-05, cl-06 or cl-07 must declare a
`frame`. A case violating either is malformed, not failing: kumihan would refuse to build the
`Text`, and a case whose input the library rejects tests nothing. This is why `frame` appears
on every item of the worked case below, including the two ideographs, where it is optional
and written out anyway so the file reads as one thing.

Both are checked by `jlreq-conform`'s own test rather than by `conform --check`, and the
difference is not bookkeeping. `Text::new` **is** ADR 0018's reader — it is the constructor
that refuses a split key, a multi-key item and an unstated frame, and it holds the Appendix
A table those refusals are stated against — so a second reader inside a gate that does not
carry the table would be a second answer to a question that already has one, and the two
would part. This document said "at gate time" before M0-b, and `conform --check` never did
it: the divergence surfaced when the first case the gate accepted and the constructor
refused reached the runner. The test is
`every_published_input_is_one_this_workspace_would_build`, it names the case and the
refusal, and it is a failure rather than a skip, because the invariant is the contract every
other case's `input` relies on.

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
what this project does and why. §3.1.3's Note reading "vertical" in English against 横組 in
Japanese is recorded this way. §D.2 note 5 against notes 1 through 3 stood beside it until
M0-b read the two ordinals against §3.8.3, which lists the line-end reduction and the
mid-line one as separate steps: what the note omits is the position, in one locale, and a
case carrying "both readings" of it would have published an alternative JLReq does not
permit. Nothing in the format lets a silence be laundered into a requirement, and nothing in
this document may launder a translation defect into one either.

## Disagreements

```json
"disagreements": [
  {
    "implementation": "LaTeX jlreq class",
    "version": "2024-11-01",
    "behavior": "Sets the line end solid after cl-02 regardless of the configured convention.",
    "our_reading": "B.2 note 2 makes a half em the preferred spacing and solid the alternative, so the convention is a caller choice rather than a fixed answer.",
    "evidence": "docs/decisions/line-end-punctuation.md"
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
/// Eight methods, each taking data and returning data. `None` means "this implementation
/// does not attempt this layer" and is reported as skipped, never as a failure: an engine
/// that exposes only line composition scores honestly on line composition. `align`, `tab`,
/// `feasible`, `lower` and `place` are required rather than defaulted to `None`, matching the
/// other three: every question here is declined per-input by the method itself returning
/// `None`, never by the method being absent, so a default impl would be a second, silent way
/// to decline that no other method here has.
pub trait Compose {
    fn name(&self) -> &str;

    /// The policy this implementation claims to follow, if any: a total map from question
    /// path to choice name, against which each case's `permitted` overlays are matched by
    /// the selection rule above. An implementation declaring none is checked against every
    /// permitted outcome rather than one.
    fn declared_policy(&self) -> Option<CasePolicy> { None }

    /// The class number, 1 through 30, of one item. JLReq: §3.9.2, §A
    fn classify(&self, input: &CaseInput, item: usize) -> Option<CaseClass>;

    /// The spacing, breakability and placement at one boundary — interior when `edge` is
    /// `None`, at the line edge it names otherwise. JLReq: §B, §C
    fn boundary(&self, input: &CaseInput, before: usize, edge: Option<Edge>) -> Option<CaseBoundary>;

    /// The composed lines. JLReq: §3.8, §D, §E
    fn compose(&self, input: &CaseInput) -> Option<CaseOutput>;

    /// The single line produced by aligning a run shorter than a caller-stated target.
    /// Reuses `CaseOutput`/`CaseLine`: one `Line` is a one-element `lines` array, and every
    /// existing field already means the right thing for it. JLReq: §3.5.3, §3.7.3
    fn align(&self, input: &CaseInput) -> Option<CaseOutput>;

    /// The runs placed for one caller-declared tab line, one `CaseLine` per placed run in
    /// `tab_starts` order. Reuses `CaseOutput`/`CaseLine` the same way `align` does; a `tab`
    /// case supplies neither `candidates` nor `measure`, only `tab_starts` and `tab_stops`.
    /// JLReq: §3.6.1, §3.6.2, §3.6.3
    fn tab(&self, input: &CaseInput) -> Option<CaseOutput>;

    /// Which of one caller-declared break candidate kinsoku leaves standing, and which rule
    /// refused it when it does not — `candidate` is the ordinal into `input.candidates`, not
    /// a byte offset. Not `boundary`'s question restated: a `boundary` answer is Tables 1
    /// and 2 at one adjacency, while a candidate's own survival additionally reads the
    /// same-run refusals of §C.2 notes 6 through 8 and 13, which need a construct overlay
    /// no table cell can express (`constructs` is load-bearing for this kind and for `lower`
    /// and `place`; every other kind either declines outright or per item wherever a case
    /// declares one). JLReq: §C.2#6, §C.2#7, §C.2#8, §C.2#13
    fn feasible(&self, input: &CaseInput, candidate: usize) -> Option<CaseFeasible>;

    /// What `jlreq_inline::lower` resolved for one declared ruby construct: its run identity
    /// against its neighbors, the forced boundary spacing §3.3.8 rule 1 computes, and, for
    /// mono-ruby, the resolved `RubyAlignment` and whether it is §3.3.5's own discouraged
    /// combination. `construct` is the ordinal into `input.constructs.ruby`. Not a further
    /// `feasible` or `boundary` question: it reaches `jlreq_inline::lower` directly, and
    /// every fact it answers is the inline-construct layer's own, resolved before a boundary
    /// or a line ever enters the picture. JLReq: §3.3.5, §3.3.8
    fn lower(&self, input: &CaseInput, construct: usize) -> Option<CaseLower>;

    /// What `jlreq_inline::place` computed for the case's own whole declared `Constructs`:
    /// every annotation it placed and every mono-ruby run it declined to place —
    /// §3.3.5(c)'s own katatsuki-with-overflow choice, unresolved for want of a policy
    /// `Question`. No ordinal parameter, unlike `boundary`, `feasible` and `lower`: `place`
    /// answers the whole call rather than one occurrence of it — `jlreq_inline::place::
    /// Attachments` has no per-construct selector for a case to name — so this method takes
    /// only `input`, `align`'s, `tab`'s and `compose`'s own shape rather than the three
    /// per-occurrence methods'. JLReq: §3.3.5
    fn place(&self, input: &CaseInput) -> Option<CasePlace>;
}

/// One case's `input` object, deserialized. The eight trait methods share it because a
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
    /// Which of `Search`'s two variants a `compose` case is measured under. `None` reads
    /// as `Search::FirstFit`, so every case published before this field existed keeps
    /// answering exactly what it always answered.
    pub search: Option<CaseSearch>,
    pub direction: String,
    pub first_line_indent: Option<i64>,
    /// Narrows every line's own measure, first line included — distinct from
    /// `first_line_indent`, which applies once.
    pub head_indent: Option<i64>,
    /// Narrows every line's own composition target from the line end side.
    pub end_indent: Option<i64>,
    /// §3.5.4's own widow threshold, in items rather than a length. `None` reads as `0`,
    /// `Paragraph::new`'s own default and a no-op by construction.
    pub widow_threshold: Option<i64>,
    /// Which of `Alignment`'s four methods an `align` case asks for. Required of `align`,
    /// ignored elsewhere.
    pub alignment: Option<String>,
    /// For a `tab` case: `starts[k]` is the item ordinal where the run after the `k`-th
    /// tab sign begins. Required of `tab`, ignored elsewhere.
    pub tab_starts: Vec<usize>,
    /// For a `tab` case: the caller's own declared pool of tab positions and their
    /// alignment kinds, in declaration order. Required of `tab`, ignored elsewhere.
    pub tab_stops: Vec<CaseTabStop>,
}

/// `tolerance` is required alongside `kind: "optimal"` and absent for `"first-fit"`, which
/// reads no tolerance at all — `Search::Optimal`'s own field.
pub struct CaseSearch { pub kind: String, pub tolerance: Option<i64> }

/// One declared tab stop: a position and a kind (`start`, `end`, `centered` or
/// `character`), with `at` present only for `kind: "character"` — the same flattening
/// `CaseSpace` already gives a reducible space's own `floor`/`stage`.
pub struct CaseTabStop {
    pub position: i64,
    pub kind: String,
    pub at: Option<usize>,
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
///
/// The runner compares `class` and never `rules`. An implementation is required to answer
/// the question, not to reproduce our chain of specification addresses: Chrome answers a
/// boundary and publishes no address for it, and a suite that failed it on provenance
/// would be the hostile artifact ADR 0006 exists to avoid. The `rules` an implementation
/// does report are unioned into `Report::rules_exercised`, where they drive the
/// exercised-coverage gate rather than the pass.
///
/// `CaseExpansion::rule` below is not a reversal of that decision: it compares one
/// provenance field conditionally rather than every classification answer's whole chain,
/// passes over (never fails) an expectation the answer meets with no citation at all, and
/// exists because three deferrals named the prior *inability* to state that citation as
/// their own blocker, which no deferral names for classification's own provenance.
///
/// Nor is `CaseBoundary::rules` below, compared under the identical logic (task #44, round
/// 16): the whole array is read as a *subset*, never an equality and never an order — every
/// address a case declares must appear somewhere among the answered rules — and a declared
/// address met by an empty answered list is passed over rather than failed, the same third
/// state `CaseExpansion::rule`'s own conditional comparison already gives one provenance
/// field. Two comparisons in this module now read a provenance field; zero read
/// classification's own.
pub struct CaseClass { pub class: u8, pub rules: Vec<String> }

/// A feasible-break answer for one of the caller's own candidates: whether kinsoku left it
/// standing (`Feasible::breaks()`) or refused it (`Feasible::rejected()`), and the rules
/// that decided it. Not `CaseBoundary`: spaces, placement, ruby overhang and expansion say
/// nothing about a candidate's own survival. `rules` is compared the identical subset way
/// `CaseBoundary::rules` is — presence among the answer's own citations, never equality and
/// never order — for the identical reason: `Feasible::compute`'s own citation for one
/// candidate is the same fixed-shape provenance chain ADR-0006 already keeps this suite
/// from demanding a foreign implementation reproduce exactly.
pub struct CaseFeasible { pub breakable: bool, pub rules: Vec<String> }

/// A `jlreq_inline::lower` answer for one declared ruby construct: per-item run identity
/// (opaque, scoped to this one answer — two items share a run when both resolve `Some` and
/// equal), the forced boundary spacing `Contribution::separations` reports across every
/// construct the answer resolved, the `RubyAlignment` resolved for the identified
/// construct (real only for `RubyStyle::MonoRuby`), and whether it is §3.3.5's own
/// discouraged combination. Not `CaseBoundary` or `CaseFeasible`: none of spacing-at-a-
/// boundary, placement or a candidate's own survival is this answer's subject.
pub struct CaseLower {
    pub runs: Vec<Option<u32>>,
    pub separations: Vec<(usize, i64)>,
    pub alignment: Option<String>,
    pub alignment_discouraged: bool,
    pub rules: Vec<String>,
}

/// A `jlreq_inline::place` answer for the case's own whole declared `Constructs`: every
/// annotation it placed and every mono-ruby run it declined to place. Not `CaseLower`: this
/// is the whole call's own answer rather than one construct's, so it carries no `construct`
/// ordinal, and no `rules` either — `Attachments` publishes none, because §3.3.5 is one rule
/// address and `lower` already records it the moment it resolves an alignment (ADR-0019).
pub struct CasePlace {
    pub attachments: Vec<CaseAttachment>,
    pub declined: Vec<usize>,
}

/// One placed annotation character, narrowed to the two facts the cased examples turn on —
/// `size`, `side`, `run` and `construct` are real `Attachment` accessors this shape omits.
pub struct CaseAttachment {
    pub inline: i64,
    pub item: Option<usize>,
}

/// A boundary answer. The conditional spaces, never their sum (ADR-0014).
pub struct CaseBoundary {
    pub spaces: Vec<CaseSpace>,
    pub breakable: bool,
    pub permitted: bool,
    pub ruby_overhang: Option<CaseOverhang>,
    /// The boundary's own Table 6 opportunity, independent of `spaces`: a fact about the
    /// class pair, not about either neighbor's own contribution (ADR-0014, amended by
    /// ADR-0021 to say so explicitly — Table 6 has no `be`/`af` column at all).
    pub expansion: CaseExpansion,
    pub rules: Vec<String>,
}

pub struct CaseSpace {
    pub units: i32,
    /// "preceding" or "trailing" — Appendix B's `be` and `af`.
    pub referent: String,
    /// "rigid", "range", or "discrete" — §3.1.9's two-valued case is not a range.
    pub reduction: String,
    pub floor_units: i32,
    /// Which ladder the stage below belongs to. Appendix D's six steps and Appendix E's
    /// four are two orderings of two different things and §3.8.2 orders the ladders
    /// themselves, so a bare `stage` would mean two things in one field (ADR-0014) — the
    /// reason this disambiguator exists at all, though a `CaseSpace` can now only ever
    /// answer `"reduction"` here: ADR-0021 moved Appendix E's own stage off the
    /// conditional space entirely, onto `CaseExpansion::stage` below.
    pub ladder: String,
    pub stage: u8,
}

/// One boundary's own expansion opportunity (ADR-0014, amended by ADR-0021, and again by
/// ADR-0021's own 2026-08-09 amendment for `rule` below).
pub struct CaseExpansion {
    /// "none", "range", or "residual" — §3.8.4 step (d)'s own unbounded fourth stage.
    pub kind: String,
    /// The ceiling, in kumihan's own unit. `None` outside `kind: "range"`.
    pub ceiling_units: Option<i32>,
    /// The priority stage (2 or 3 — Appendix E's own first stage, the Western word space,
    /// is outside Table 6 and never appears here). `None` outside `kind: "range"`.
    pub stage: Option<u8>,
    /// Which rule states this coordinate's Table 6 row, by address —
    /// `jlreq_spacing::Boundary::expansion_rule`'s own answer. `None` both for an
    /// implementation that publishes no specification address and for a coordinate Table 6
    /// carries no row for at all: the runner cannot and does not tell the two apart, only
    /// an implementation that knows which one it meant can. Present here even when `kind`
    /// is `"none"` — a row can state that the opportunity does not exist, and that denial
    /// is still a citable fact the row's own address states, not the same absence as a
    /// coordinate carrying no row.
    pub rule: Option<String>,
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
    /// §3.1.12 ⑤'s repair as `Search::Optimal` applied it to this line. `None` asserts
    /// that `Line::pull_up` answers `None` — a positive claim, not "unchecked" — which
    /// is safe retroactively because `Search::FirstFit` never answers anything else.
    pub pull_up: Option<CasePullUp>,
}

pub struct CaseTrim { pub item: usize, pub units: i32, pub resolved: i64,
                      pub referent: String, pub rule: String }

/// `PullUp::pulls` is the item the nearer, un-taken candidate would have ended this line
/// at — the boundary where the next line would otherwise have started.
pub struct CasePullUp { pub amount: i64, pub pulls: usize, pub rule: Option<String> }

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
    /// How many permitted entries no declared policy of this run could select, because the
    /// implementation's policy does not have the question the entry names. A published
    /// reading nothing can select is evaluated by nothing, so a case may carry three
    /// entries and assert only what its `{}` entry says; the number is what stops that
    /// being a silence on a green run. Zero for an implementation declaring no policy,
    /// which is measured against every entry.
    pub unselectable: usize,
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
/// One file's cases, which is the unit one generated test covers.
pub fn run_file<C: Compose>(file: &CaseFile, implementation: &C) -> Report;
/// Score an answers file produced by any implementation in any language. Not written yet.
pub fn judge(suite: &Suite, answers: &Path) -> Result<Report, LoadError>;
```

Every type above is `#[non_exhaustive]`, which [ADR
0012](../adr/0012-outcome-and-detail-compatibility.md) requires of the whole published
surface, so every answer type an implementation *constructs* — `CaseClass`, `CaseBoundary`,
`CaseFeasible`, `CaseLower`, `CaseSpace`, `CaseExpansion`, `CaseOutput`, `CaseLine`,
`CaseTrim`, `CasePart` — carries a `new` naming the fields that exist. A field added to one of them then leaves an
implementation that already compiles compiling, which is the whole of that decision applied
to an artifact other people build against.

`judge` and the binary are what the crate owes the ecosystem and are the part of this
document the code has not caught up with, as the top of this document says: `run` and
`run_file` are written, `judge` is not, there is no `src/bin`, no `[[bin]]` and no
`answers/`, and `crates/jlreq-conform`'s own crate documentation says so rather than
leaving a reader to find out. The commands below are therefore the design and not the
interface; nothing in this repository accepts them today.

```sh
jlreq-conform run   --cases cases/                    # this workspace's implementation
jlreq-conform run   --cases cases/ --section 3.1.9
jlreq-conform judge --cases cases/ --answers mine.json
```

The answers file will be one object per case id carrying the same shapes, so an
implementation in any language emits JSON and runs one command. Thirty lines in any
language, no build integration, and it works for a browser driven by a headless harness.
That sentence is the whole of the format's specification today, which is not enough to
write one against: the answers file arrives with `answers.schema.json` beside
`cases.schema.json` and with an example under `answers/`, and until all three exist this
document is describing an artifact rather than publishing one.

## "Every rule has a case", mechanically

[CONTRIBUTING.md](../../CONTRIBUTING.md) requires that a rule without a case is incomplete.
[ADR 0013](../adr/0013-rules-are-addressed-by-specification-address.md) makes that
arithmetic, because the tables, the doc comments, and the case files all use one address
space. Two gates, because one is not enough.

**Declared coverage**, static, in `cargo run -p xtask -- conform --check`:
`RuleId::ALL` minus the union of every case's `rules` field must be empty. That subtraction
has two operands and either can be the one that does not exist yet. The inventory is
generated whole at M0-a; the suite is written milestone by milestone, and until the `cases`
directory exists at all the gate reports the subtraction as a check that did not run,
naming how many inventoried rules it would have closed over, rather than reporting a
coverage it never computed or a failure that is only the schedule. Creating the directory
turns it on, empty or not, so the deferral cannot outlive the first case. It also checks
that every `rules` entry resolves to a known rule, every case id is unique, every case
validates against the schema, every `permitted` entry's overlay is valid and the entries'
key sets are totally ordered by inclusion, every integer is inside 2^53, every fraction
agrees with its unit count, every `trims` rule is §3.1.2 or a Table 1 cell stating that
amount with that referent, every item is one Appendix A key, and every item whose key
Appendix A names under one of §3.1.2's five classes declares a frame.

The rule inventory is generated whole and the suite is written milestone by milestone, so
that subtraction has a remainder that is nothing but the schedule. It is not solved by
weakening the gate and not by writing hollow cases: an inventoried rule is in exactly one of
three states, and the third is written down in
[conformance-deferrals.toml](../conformance-deferrals.toml). A rule is **covered** when a
case names it or a family credits it; **deferred** when a `[[deferred]]` table names it, the
milestone whose cases close it, and why that is the milestone; and **uncovered** otherwise,
which fails. `conform --check` holds every entry to the inventory and to `ROADMAP.md`'s own
milestone headings, fails an entry whose rule a case already covers — the deferral is stale,
and deleting it is the reviewable act that says the rule now has a case — and prints the
deferred count per milestone on every run, green or not, so the debt is stated in numbers
rather than being a silence. `spec-links` subtracts the same file, because a cited rule
whose case a later milestone writes is the same debt seen from the citation side. Nothing
mechanical can know that the milestone named is the *right* one; that is what `why` and the
code-owner guard are for.

**Exercised coverage**, dynamic, as a test in `jlreq-conform`: run every case against this
workspace while accumulating the rules the evaluator reports as fired, and assert the
accumulated set equals `RuleId::ALL`. This catches a case that *names* a rule and never
reaches it — the failure a static gate cannot see, and the reason the declared gate alone
is satisfiable by adding a string to a list.

The accumulation is written — `Report::rules_exercised` is the union of what the answers
themselves carry — and the assertion is not, because at M0 it would compare a set two layers
have not begun to fill against the whole inventory and fail on the schedule rather than on a
defect. It arrives with the declared set it can honestly be compared against: from M1 the
subtraction is `RuleId::ALL` minus the deferrals, which is exactly the set the declared gate
already computes, and the two halves then constrain the same number from opposite sides.

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
will consequently be several thousand, so a case may be a **family**: one case entry may
carry a `covers` field naming a row, a column, or a class-pair set, and the gate credits
every rule in it. Without families the coverage requirement would demand several thousand
hand-written cases and would be abandoned; with them it stays honest, because a family still
has to *exercise* each cell it claims under the dynamic half.

The tense matters, because the arithmetic in this section is written against a number that
does not exist yet. `spec/derived/rules.tsv` inventories 106 rules — every section of §3 and
every appendix note — and no table cell, because `derive` has never been extended to walk
the six matrices into rule addresses; that is independent of whether the matrices
themselves are transcribed, which they now are
([generation.md](generation.md)). So `covers` has no user today and the coverage gate runs
over sections and notes alone; both facts are visible in the `conform` census, which prints
the inventory's size on every run.

A case may still name a matrix coordinate today, outside the coverage arithmetic entirely:
`cells`, a case-level, optional, list-valued field of `{table, before, after}` objects,
naming the table number and the two axis labels the way `spec/captured/` and
`xtask::attest` key a cell — not through the `address` grammar's `@` suffix, which a
multi-table legend such as §D.1 (Tables 3, 4 and 5 at once) cannot spell unambiguously.
`xtask attest`'s `conformance-cases-agree-with-the-cells` invariant (ADR 0006) is this
field's only reader: it asserts existence, at every declared table, and, for Table 1 alone,
that a case's default-policy (`policy: {}`) boundary answer agrees in units with the
captured cell. Neither `conform`'s declared-coverage gate nor `jlreq-conform`'s own case
reader ever looks at `cells` — a boundary case that omits it declares nothing about the
transcription either way, the same asymmetry `covers` has before the cell inventory exists.

## Two further gates the format makes free

**Direction parity**, from M1. Every case not marked `direction: "vertical"` or
`"horizontal"` is composed twice, once each way, and the inline results must be
bit-identical. [ADR 0011](../adr/0011-typed-axes-and-direction-as-a-datum.md) names exactly
three direction-conditional rules; a fourth shows up here as a failing case over the whole
corpus rather than as a code review that has to notice it. This is what turns ADR 0004 from
an aspiration checked at M5 into a property proved from M1.

**Cross-search agreement** is not a gate this suite runs, and the tense matters here the
same way it does above: this section once described it in the present tense, alongside
direction parity, before a case existed that could name `Search::Optimal` at all. No runner
in `jlreq-conform` composes a case under both `Search::FirstFit` and `Search::Optimal` and
compares the results — `ask` (`crates/jlreq-conform/src/run.rs`) calls `Compose::compose`
exactly once per case, under whichever search `input.search` names (`Search::FirstFit` when
it names none, `cases.schema.json`'s own field). What a later round added is the vocabulary
for a case to *name* a search at all, not a runner that runs both and checks agreement.
Building that gate is real, scoped, future work: it would have to run every existing case
under `Search::Optimal` in addition to whatever its own `input.search` already asks for, and
settle what "the first break of every paragraph whose first line has a unique feasible
answer" means as a checkable predicate over the corpus rather than a sentence — neither of
which is done by naming the field.

The design reasoning below is kept because it survives the gate's own absence and would be
what makes the gate satisfiable rather than aspirational, once built. Hanging punctuation is
a stage of the shared `Ladder`, between reduction and expansion, not a repair the greedy path
applies after choosing a break — §2.5.1 says it "is only necessary … when they would
otherwise need to be wrapped to the line head" and that "if possible the full stops or commas
are placed at the line end", which is a fit decision. And §3.1.12 ⑤ is not a mechanism at
all: pulling up is taking the later of two feasible breaks and paying reduction, pushing down
is taking the earlier one and paying expansion, and preferring the first is §3.8.2 — which
`FirstFit` reaches greedily and `Optimal` reaches through `Preference`. Had either lived in
one search and not the other, a cross-search gate built on this reasoning would fail across
the whole corpus and the fix would be a redesign after `Fit` and `Demerits` were frozen —
which is also why nothing here quietly widens the reasoning into a claim that the gate runs.

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
- The three input refusals of [ADR 0018](../adr/0018-an-item-is-one-occurrence.md): a split
  `<02E5, 02E9>`, a full-width multi-key item, and an unstated frame on `。`. They belong in
  the suite because they are the contract every other case's `input` relies on, and because
  an implementation that accepts them will disagree with every implementation that does not.
  **They cannot be published as cases until the format has a way to say that an input is
  expected to be refused**, and this document required them as ordinary cases while
  `tests/suite.rs`'s `every_published_input_is_one_this_workspace_would_build` asserted that
  every published input is one `Text::new` accepts — two requirements that cannot both be
  met, since a refusal case is precisely a case that constructor refuses. The requirement is
  therefore on the format first: an `expect.refused` naming the refusal, a `kind` for it, and
  an exemption from the two `input` checks that would otherwise reject the case at gate time
  — after which the three cases are written and this bullet is a case list again. Until then
  the three refusals are held by `jlreq-class`'s own tests over `Text::new`, which is the
  same invariant seen from the side that can state it.
