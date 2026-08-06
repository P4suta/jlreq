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

`resolve` answers with the **lowest-numbered surviving class**.

Every answer it produces this way carries `Standing::Unstated`, so a caller can tell it from
a class the specification decided by reading the provenance rather than by knowing which
keys are ambiguous.

## Why

Appendix A numbers the Japanese classes before the Western ones. cl-01 through cl-26 are
the brackets, the punctuation, the kana, the ideographs and the ideographic space; cl-27 is
Western characters. Taking the lowest-numbered survivor is therefore §3.9.2's own preference
made mechanical: on its own example — `U+0028` named under cl-01, cl-25, cl-27 and cl-28 —
it answers cl-01, the Japanese design, which is the answer the section says is better.

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
