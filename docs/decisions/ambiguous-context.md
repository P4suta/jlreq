# Reading: the class of an occurrence several classes name and nothing separates

- Applies to: `jlreq_class::resolve`
- Standing: `Unstated`
- JLReq: §3.9.2

## The silence

Appendix A names 473 of its 1133 keys under more than one class, reaching degree four.
Where the caller's frame (字幅), role and construct do not separate the survivors, §3.9.2
concedes the case rather than deciding it. Its own example is a Latin spelling
parenthesized inside Japanese — "エディター（editor）は……" — of which it says the Japanese
design of the brackets "is better".

"Is better" is a preference. It is not a rule, it names no class, and it is stated about
one example rather than about the general case. So the specification decides nothing here,
and `jlreq_class::classify` reports the surviving candidates instead of picking one.

## The reading

`resolve` answers with the **lowest-numbered surviving class the supplied facts can
reach** — the lowest-numbered survivor that is not membership in a construct the caller
never declared, and the lowest-numbered survivor of all only where every survivor is one.

Every answer it produces this way carries `Standing::Unstated`, so a caller can tell it from
a class the specification decided by reading the provenance rather than by knowing which
keys are ambiguous.

### Why the construct classes are reached last

Nine of the thirty classes are membership *in* a construct: the five that enumerate nothing
at all, and cl-24, cl-25, cl-28 and cl-29, which enumerate what may appear inside a grouped
numeral (連数字), a unit symbol or a warichu (割注) bracket. `classify` takes no construct
axis and cannot — a construct is a run over a stream rather than a property of one item
([ADR 0015](../adr/0015-the-crate-graph-and-the-inline-line-seam.md)) — so as far as this
layer can know, no occurrence it is given is inside one.

Those four are numbered *below* cl-27. A tie-break that took the lowest-numbered survivor
over all of them therefore answered "a character inside a unit symbol" — 25 before 27 — for
every proportional Latin letter in a Japanese document, which is a statement about the
caller's text that nothing in the caller's text supports. Passing over them is not a second
preference beside §3.9.2's; it is the same preference applied to the survivors the caller's
own facts reach. Where every survivor is a construct membership — `U+0031` declared on the
half em, which §A.24 and §A.25 both name — nothing is passed over and the lowest-numbered
survivor is the answer, because Appendix A named those and named nothing else.

`classify` continues to report the whole surviving set with `AxisSet::CONSTRUCT` naming the
axis that would settle it. The candidates are the specification's; only the tie-break is
ours.

This clause was added in M0-b. It was the rule the implementation had applied since M0-a and
the sentence above did not state it, and two conformance-case authors reading this file
wrote `A.25/digit-grouping-space/quarter-em-frame` and
`A.26/word-space/frame-and-role-unstated` against the unqualified wording. A published
reading an implementer cannot reproduce from the document is the defect; the correction is
here rather than in the code.

## Why

Appendix A numbers the Japanese classes before the Western ones. cl-01 through cl-26 are
the brackets, the punctuation, the kana, the ideographs and the ideographic space; cl-27 is
Western characters. Taking the lowest-numbered survivor is therefore §3.9.2's own preference
made mechanical: on its own example — `U+0028` named under cl-01, cl-25, cl-27 and cl-28 —
it answers cl-01, the Japanese design, which is the answer the section says is better.

On `U+0020`, which §A.24, §A.25 and §A.26 all name and which a caller may declare nothing
about, it answers cl-26: the two classes numbered below it are the grouped numeral and the
unit symbol, and a caller who declared neither has not put the space inside one.

The alternative readings were considered and are worse for reasons that are about this
specification rather than about taste. Answering with the *last* class would prefer the
Western design over the Japanese one, which is the answer §3.9.2 argues against. Answering
with the most-listed class is a property of the table's shape and of nothing the document
says. Refusing to answer at all is what `classify` already does, and `resolve` exists for
the caller who cannot use that.

The ordering is total and deterministic, so two implementations reading this file answer
alike, which is what makes the disagreement publishable if one of them disagrees.

## What would change it

A revision of §3.9.2 that states a rule rather than a preference, or that gives the
ambiguous case an answer of its own. A `Question::AMBIGUOUS_CONTEXT` in the generated policy
space with choices the specification permits: this reading is what applies when the caller
answers nothing, and the moment the policy space exists a caller may answer otherwise.

Evidence that publishers systematically prefer a different survivor would not change the
reading on its own — it would be recorded as a `disagreements` entry on the conformance
case, which is what that field is for.
