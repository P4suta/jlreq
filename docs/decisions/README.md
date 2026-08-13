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
- **The reading** — what kumihan answers, precisely enough to be implemented from.
- **Why** — the argument, from the specification's own text wherever there is one.
- **What would change it** — what evidence or what revision would make this wrong.

The conformance suite must carry every reading here with *all* of its readings, not only
the one this project takes ([conformance.md](../design/conformance.md)).

| Reading | Question | Where it applies |
| --- | --- | --- |
| [ambiguous-context](ambiguous-context.md) | Which class, when Appendix A names several and nothing separates them | `jlreq_class::resolve` |
| [unlisted-code-point](unlisted-code-point.md) | Which class, when Appendix A lists the key nowhere | `jlreq_class::resolve` |
| [compatibility-ideographs](compatibility-ideographs.md) | Whether a CJK Compatibility Ideograph is cl-19, and whether it is normalized first | `jlreq_class::classify`, `jlreq_class::resolve` |
| [grouped-numeral-qualification](grouped-numeral-qualification.md) | Whether the width or the job §A.24's Remarks cell names is what reaches cl-24 | `jlreq_class::classify`, `jlreq_class::resolve` |
| [adjustment-preference](adjustment-preference.md) | Where the four non-ladder demerit components sit in `Preference`'s ordering | `jlreq_line::Preference`, `jlreq_line::Demerits` |
| [european-numeral-by-code-point](european-numeral-by-code-point.md) | Whether §C.2 note 11's "European numeral" is a declared role or a fact read from the occurrence's own key | `jlreq_spacing::evaluate::boundary` |
| [inseparable-character-kind](inseparable-character-kind.md) | What "of different kinds" means for two adjacent inseparable characters (cl-08) in §E.2 note 4 | `jlreq_spacing::evaluate::boundary` |
| [specified-character-tab-alignment](specified-character-tab-alignment.md) | Whether §3.6.2's specified-character tab kind names its occurrence by `char` or by a caller-declared `ItemIndex` | `jlreq_line::tab::TabKind::Character` |
| [sentence-medial-dividing-mark](sentence-medial-dividing-mark.md) | What §3.1.6's third Note leaves open about a sentence-medial cl-04 mark's quarter em: whose em it is, which coordinates it reaches, and what a line edge does to it | `jlreq_spacing::evaluate::boundary` |
| [jukugo-ruby-unset-group](jukugo-ruby-unset-group.md) | What an occurrence with no declared `GroupId` means for §C.2 note 8's base-and-ruby indivisibility | `jlreq_line::feasible::same_run_refusal` |
| [line-head-opening-bracket](line-head-opening-bracket.md) | What §3.1.5 pattern 2 and §B.2 note 17 leave open about the wrapped-line-head half em before an opening bracket: whose em it is, whether Appendix D reduces it, and which of the two paired addresses the synthesized space cites | `jlreq_spacing::evaluate::boundary` |
| [tolerance-exhaustion](tolerance-exhaustion.md) | What `compose` does when `Search::Optimal`'s own `tolerance` admits no complete arrangement at all | `jlreq_line::compose` |
| [widow-threshold](widow-threshold.md) | What §3.5.4 leaves open about widow adjustment: what counts as a character, whether a one-line paragraph can have a widow, the penalty's own shape, and what an unsatisfiable threshold means | `jlreq_line::compose` |
| [mono-ruby-separation-split](mono-ruby-separation-split.md) | How a mono-ruby run's §3.3.8 rule 1 overhang surplus splits between its two boundaries, whether the split depends on `RubyAlignment`, and how two runs' demands at one shared boundary combine | `jlreq_inline::lower`, `jlreq_inline::place` |
| [group-ruby-flush-single-character](group-ruby-flush-single-character.md) | What §3.3.6's `flush` method does for a group-ruby run of exactly one ruby character, whose leading and trailing clauses name the same character at once | `jlreq_inline::place` |
| [jukugo-group-layout-distribution](jukugo-group-layout-distribution.md) | Which of §3.3.6's own two methods §3.3.7¶2's own `group` answer means — `jis`, forced, or whichever `Question::GROUP_RUBY_DISTRIBUTION` names | `jlreq_inline::place` |

These are Markdown and not TOML. An earlier revision of `docs/design/api-spine.md` named
them `*.toml`, on the model of the other machine-read files in this repository; they are
prose, and the reason is visible in every one of them. A reading is an argument from the
specification's own words, with the alternatives and the reasons they are worse — the four
headings above are its shape, not a schema — and a TOML file either flattens that into one
quoted string or invents a record format for paragraphs. Nothing reads these mechanically:
the `conform` gate's overlay keys name a reading by the stem of its file, which is the same
whichever extension follows it, and `attest` names the recorded defects rather than the
readings. The spine now says Markdown, and says so here as well as there.
