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

These are Markdown and not TOML. An earlier revision of `docs/design/api-spine.md` named
them `*.toml`, on the model of the other machine-read files in this repository; they are
prose, and the reason is visible in every one of them. A reading is an argument from the
specification's own words, with the alternatives and the reasons they are worse — the four
headings above are its shape, not a schema — and a TOML file either flattens that into one
quoted string or invents a record format for paragraphs. Nothing reads these mechanically:
the `conform` gate's overlay keys name a reading by the stem of its file, which is the same
whichever extension follows it, and `attest` names the recorded defects rather than the
readings. The spine now says Markdown, and says so here as well as there.
