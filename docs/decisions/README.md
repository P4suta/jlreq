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
| [jukugo-group-layout-distribution](jukugo-group-layout-distribution.md) | Which of §3.3.6's two methods §3.3.7¶2's `group` answer means | [`pipeline`](../../crates/jlreq/src/pipeline.rs) |

These are Markdown and not TOML. An earlier revision of `docs/design/api-spine.md` named
them `*.toml`, on the model of the other machine-read files in this repository; they are
prose, and the reason is visible in every one of them. A reading is an argument from the
specification's own words, with the alternatives and the reasons they are worse — the four
headings above are its shape, not a schema — and a TOML file either flattens that into one
quoted string or invents a record format for paragraphs. Nothing reads these mechanically:
the `conform` gate's overlay keys name a reading by the stem of its file, which is the same
whichever extension follows it, and `attest` names the recorded defects rather than the
readings. The spine now says Markdown, and says so here as well as there.
