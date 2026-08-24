# Published readings

JLReq says nothing in some places and two incompatible things in a few. A library that
quietly filled either would publish invention as requirement
([ADR 0009](../adr/0009-generated-data-and-attested-transcription.md),
[ADR 0013](../adr/0013-rules-are-addressed-by-specification-address.md)), so each one is
recorded here as what it is, and every answer that passed through one carries
`Standing::Unstated` or `Standing::Adjudicated` rather than `Standing::Normative`.

A reading is not a decision about how to implement something. It is an answer to a question
the specification leaves open, published so that an adopter can disagree with it in writing
and a conformance case can record both readings rather than only ours.

Each file states four things, in this order and with these headings:

- **The silence** — what the specification says, quoted, and what it does not say.
- **The reading** — what jlreq answers, precisely enough to be implemented from.
- **Why** — the argument, from the specification's own text wherever there is one.
- **What would change it** — what evidence or what revision would make this wrong.

The conformance suite must carry every reading here with *all* of its readings, not only
the one this project takes ([conformance.md](../design/conformance.md)). The prose in an
individual reading preserves names from the implementation it adjudicated; this index links
to the current private owner after the crate unification.

The first table below is the set of readings the Rust engine's own development produced. The
second is a set the *second* engine found: writing an independent implementation
([engines/ocaml/](../../engines/ocaml/README.md)) turned up rules that two engines have to
agree on to pass the same case and that JLReq states in no sentence and `docs/` stated in no
file. Those were listed in the OCaml engine's README while the port was in progress and are
promoted here, bundled by subject rather than one file per rule, because a policy an engine
had to be told about is a reading whether it was found by argument or by measurement. Each
one names how it was observed — which census, and which of the eighty-nine built-in cases
reaches it, where any does — so that a reader can reproduce the observation rather than take
the claim.

A *third* engine ([engines/racket/](../../engines/racket/README.md)) has since been brought
to zero differences against both of the others on all ten censuses, 111,090 requests, and
that convergence turned up nine more policies of the same kind. Two of them are subjects
nothing here covered and have files of their own in the table below; the other seven belong
to subjects already published and were written into those files, one of them into a reading
in the *first* table. A reading found this way is not a weaker reading. Three
implementations — one reading the English transcription, one the Japanese, one the English
while holding the Japanese cell for cell in its own tests — answering every request
identically have agreed about something JLReq does not state, and what they agreed about
belongs here rather than in an engine's README.

| Reading | Question | Where it applies |
| --- | --- | --- |
| [ambiguous-context](ambiguous-context.md) | Which class, when Appendix A names several and nothing separates them | [`spec` / `normalize`](../../crates/jlreq/src/normalize.rs) |
| [unlisted-code-point](unlisted-code-point.md) | Which class, when Appendix A lists the key nowhere | [`spec` / `normalize`](../../crates/jlreq/src/normalize.rs) |
| [compatibility-ideographs](compatibility-ideographs.md) | Whether a CJK Compatibility Ideograph is cl-19, and whether it is normalized first | [`spec`](../../crates/jlreq/src/spec.rs) |
| [grouped-numeral-qualification](grouped-numeral-qualification.md) | Whether the width or the job §A.24's Remarks cell names is what reaches cl-24 | [`spec`](../../crates/jlreq/src/spec.rs) |
| [adjustment-preference](adjustment-preference.md) | Where the non-ladder demerit components sit in paragraph optimization | [`pipeline`](../../crates/jlreq/src/pipeline.rs) |
| [european-numeral-by-code-point](european-numeral-by-code-point.md) | Whether §C.2 note 11's "European numeral" is a declared role or a fact read from the occurrence's own key | [`pipeline`](../../crates/jlreq/src/pipeline.rs) |
| [inseparable-character-kind](inseparable-character-kind.md) | What "of different kinds" means for two adjacent inseparable characters (cl-08) in §E.2 note 4 | [`pipeline`](../../crates/jlreq/src/pipeline.rs) |
| [specified-character-tab-alignment](specified-character-tab-alignment.md) | Whether §3.6.2's specified-character tab kind names its occurrence by `char` or by a caller-declared item | [`paragraph` / `pipeline`](../../crates/jlreq/src/paragraph.rs) |
| [sentence-medial-dividing-mark](sentence-medial-dividing-mark.md) | What §3.1.6's third Note leaves open about a sentence-medial cl-04 mark's quarter em: whose em it is, which coordinates it reaches, and what a line edge does to it | [`pipeline`](../../crates/jlreq/src/pipeline.rs) |
| [jukugo-ruby-unset-group](jukugo-ruby-unset-group.md) | What an occurrence with no declared group means for §C.2 note 8's base-and-ruby indivisibility | [`pipeline`](../../crates/jlreq/src/pipeline.rs) |
| [line-head-opening-bracket](line-head-opening-bracket.md) | What §3.1.5 pattern 2 and §B.2 note 17 leave open about the wrapped-line-head half em before an opening bracket: whose em it is and whether Appendix D reduces it | [`pipeline`](../../crates/jlreq/src/pipeline.rs) |
| [tolerance-exhaustion](tolerance-exhaustion.md) | What composition does when no complete arrangement meets its quality threshold | [`pipeline`](../../crates/jlreq/src/pipeline.rs) |
| [widow-threshold](widow-threshold.md) | What §3.5.4 leaves open about widow adjustment and an unsatisfiable threshold | [`paragraph` / `pipeline`](../../crates/jlreq/src/paragraph.rs) |
| [mono-ruby-separation-split](mono-ruby-separation-split.md) | How mono-ruby overhang surplus splits and how demands at a shared boundary combine | [`pipeline`](../../crates/jlreq/src/pipeline.rs) |
| [group-ruby-flush-single-character](group-ruby-flush-single-character.md) | What §3.3.6's `flush` method does for a group-ruby run of exactly one ruby character | [`pipeline`](../../crates/jlreq/src/pipeline.rs) |
| [jukugo-group-layout-distribution](jukugo-group-layout-distribution.md) | Which of §3.3.6's two methods §3.3.7¶2's `group` answer means, and whether `ruby.group_distribution` selects anything inside a jukugo compound | [`pipeline`](../../crates/jlreq/src/pipeline.rs) |

Promoted from the second and third engines' observations:

| Reading | Question | How it was observed |
| --- | --- | --- |
| [expansion-ladder-scope](expansion-ladder-scope.md) | Which coordinates §3.8.4's Japanese–Latin ceiling is asked at, and which sites its fourth step re-levels | `just census expansion` |
| [tate-chu-yoko-spacing-sources](tate-chu-yoko-spacing-sources.md) | Whether §3.2.5's prose or Table 1's cl-30 row states the space beside a run, and whether Appendices D and E read a cell whose amount another section has spent — §3.2.5's cl-30 cells, and the cell after a tab sign | `just census tate-chu-yoko`, `tabs` |
| [construct-break-refusal](construct-break-refusal.md) | Whether a break the caller states inside an indivisible construct is refused or declined, and where §3.7.4 lets a formula break | `just census tate-chu-yoko`, `ruby`, `constructs` |
| [ruby-overhang-permission](ruby-overhang-permission.md) | Whether §3.3.8 rule 2's kana neighbor is a script or a class, whose em a Table 1 `hang` term was measured in, which side each allowance is available on, whose characters its full-width size is measured in, and what a middle dot allows | `just census ruby` |
| [ruby-distribution-and-rounding](ruby-distribution-and-rounding.md) | What §3.3.6 does for a run of one, what its outer units are, which way an odd unit falls, what §F.3's self-referring total evaluates to, and in which order it is spent | `just census ruby` |
| [ornamented-complex-geometry](ornamented-complex-geometry.md) | What an emphasis mark is centered on, how many complexes an emphasis run is, and where §3.7.1's annotation sits | `just census constructs` |
| [stacked-structure-geometry](stacked-structure-geometry.md) | Which positions a warichu may divide at, whether its balance sentence is a bound, whose advance a structure's trailing and leading space is part of, and where §3.4.3's balance ranks against the line it stands on | `just census constructs` |
| [warichu-bracket-listing](warichu-bracket-listing.md) | Whether the `warichu-bracket` role narrows a key §A.28 and §A.29 list or reaches cl-28 and cl-29 with any bracket | `just census constructs` |
| [tab-line-correspondence](tab-line-correspondence.md) | What a tab sign with no stop left does, whether §3.6.3's cut answers to §3.1, what §3.6.1's count is counted over, and which of a jidori and a tate-chu-yoko run holds a coordinate a stop can name | `just census tabs` |
| [jidori-room-and-solid-boundaries](jidori-room-and-solid-boundaries.md) | What §3.7.3 measures its room in, which boundaries it sets solid, and where a run with no boundary left stands | `just census tabs`, `constructs` |
| [unstated-alignment](unstated-alignment.md) | What a request that states no `alignment` asks for | `just census tabs`, `widow`; case `3.5.4/widow-keeps-two-clusters-on-last-line` |
| [inexpressible-advance-remarks](inexpressible-advance-remarks.md) | Whether an Appendix A Remarks cell naming only an unexpressible advance excludes its listing or qualifies nothing | `just census vertical` |
| [jidori-inserted-space-locale-split](jidori-inserted-space-locale-split.md) | How many sides of an inserted space §3.7.3 opens, where its two renderings state opposite rules | `just census constructs` |

Every reading in the second table applies to the layout round in
[`pipeline`](../../crates/jlreq/src/pipeline.rs) — except
[inexpressible-advance-remarks](inexpressible-advance-remarks.md) and
[warichu-bracket-listing](warichu-bracket-listing.md), which apply to
[`spec`](../../crates/jlreq/src/spec.rs) — and to the corresponding round of
[`engines/ocaml/lib/`](../../engines/ocaml/lib/pipeline.ml) and of
[`engines/racket/`](../../engines/racket/README.md). Each file names the first two by
module; the two the third engine's convergence added name all three.

Two coordinates the same work turned up are **not** in either table, because they are not
readings this project publishes: they are places the reference engines answer differently,
and the rule is to settle a disagreement by returning to JLReq and to `spec/` rather than by
copying one engine's answer into another. Both concern a tab sign standing inside a
structure that does not set its text along the line, and both are reported as issues on this
repository and named from [engines/ocaml/README.md](../../engines/ocaml/README.md). The
third engine reached the same answer as the second at both, from the specification rather
than from the second engine, and that changes nothing: a reading arrives here once one of
them is settled, and never by a majority among implementations.

These are Markdown and not TOML. An earlier revision of `docs/design/api-spine.md` named
them `*.toml`, on the model of the other machine-read files in this repository; they are
prose, and the reason is visible in every one of them. A reading is an argument from the
specification's own words, with the alternatives and the reasons they are worse — the four
headings above are its shape, not a schema — and a TOML file either flattens that into one
quoted string or invents a record format for paragraphs. Nothing reads these mechanically:
the `conform` gate's overlay keys name a reading by the stem of its file, which is the same
whichever extension follows it, and `attest` names the recorded defects rather than the
readings. The spine now says Markdown, and says so here as well as there.
