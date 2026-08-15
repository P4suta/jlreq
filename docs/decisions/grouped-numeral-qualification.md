# Reading: what reaches the grouped numeral class where §A.24 names a job as well as a width

- Applies to: `jlreq_class::classify`, `jlreq_class::resolve`
- Standing: `Unstated`
- JLReq: §3.9.2, §A.24

## The silence

§A.24 enumerates thirteen keys. Ten are the European numerals, and their Remarks cells state
a width and nothing else — `half-width` / 「字幅は半角」. The other three state a width **and**
a job: `U+002C` reads "quarter em width or half-width" against
「位取りのコンマ／字幅は四分角又は半角」, `U+0020` reads "quarter em width" against
「位取りの空白／字幅は四分角」, and `U+002E` names the decimal point.

§3.9.2 describes the class itself as "Sequences of European numerals which are not
full-width and are handled as Japanese text, the decimal point or the comma and space used
as a decimal place indicator in numbers."

For a numeral the two agree, because the numeral's cell states no job. For the other three
they cover different occurrences and the document never says which is the membership test.
An occurrence that has the width and does not do the job — a comma set on the quarter em
between two hiragana, which indicates no decimal place — is described by neither sentence
and excluded by neither. §3.9.2 does not say that a comma outside a numeral is not cl-24; it
says what the class is *for*. §A.24 does not say the width alone admits an occurrence; it
qualifies a listing.

The occurrence is not idle. §3.1.2 states the character advance of commas (cl-07) as
half-width, so a quarter em is not the frame that class is set on, and cl-27's cell names
the proportional frame, so it is not that either. Whichever way this question is answered,
the answer is a class the other reading refuses.

## The reading

The width is the membership test **where the document has already ruled out every listing
of the key outside a construct**. There `classify` answers **cl-24** for an occurrence whose
declared frame one of §A.24's Remarks cells names, whether or not the caller declared the
job that cell also names: §3.1.2 states the character advance of full stops (cl-06) and
commas (cl-07) as half-width, so a quarter em is not the frame either class is set on, and
§A.27's cell names the proportional frame, so it is not that either. §A.24's qualified cell
is then the only listing left that describes what the caller measured, and it is the answer.

What the width does **not** do is displace a listing the document leaves standing. A
qualified cell states the width the character has *inside* the construct its own class is
membership of; it is not a test that admits every occurrence of that width to it, and
`classify` is given no construct axis to check the other half against
([ADR 0015](../adr/0015-the-crate-graph-and-the-inline-line-seam.md)). So where a listing
outside a construct survives, it stands beside the qualified one and the answer is the
tie-break of [ambiguous-context](ambiguous-context.md), which passes over the classes that
are membership in a construct.

The answer carries `Standing::Unstated` wherever this reading is what produced it, so a
caller can tell it from a class the specification decided. §3.2.6's Note is not this
reading and is not marked as one: it states outright that a half-width European numeral
mixed with Japanese text is cl-24, which is the document answering rather than this project
reading, and that answer is `Standing::Normative`.

### The three keys, and where each falls

`U+002E` and `U+002C` on the quarter em are the reading's own subject and answer **cl-24**,
because §3.1.2 has ruled out cl-06 and cl-07 and §A.27's cell has ruled out cl-27.

`U+0020` on the quarter em answers **cl-26**. §A.26 lists it with an *empty* Remarks cell,
which states no width and therefore refuses none, and §D gives the Western word space a
quarter em of its own: it is reduced "to leave a minimum of a quarter em spacing between
words", so a quarter em is a width §D itself produces for cl-26 rather than one that rules
it out. Nothing has ruled out the listing outside a construct, so §A.24's and §A.25's cells
qualify their own listings and displace nothing, all three stand, and the tie-break reaches
cl-26.

This clause was added in M0-b, and adding it was a correction rather than a refinement. The
sentence above it read "the width is the membership test" without the qualification, and
applied to `U+0020` it contradicted [ambiguous-context](ambiguous-context.md)'s own worked
example of the same key — that file states, of `U+0020` under §A.24, §A.25 and §A.26, that
the tie-break "answers cl-26: the two classes numbered below it are the grouped numeral and
the unit symbol, and a caller who declared neither has not put the space inside one". Two
decision documents disagreed about one key and the implementation followed the wrong one,
by eliminating cl-26 before the tie-break could reach it. The reading here is the one both
files now state.

## Why

Three reasons, all about this specification rather than about taste.

The Remarks column is the only thing Appendix A gives a reader to tell two listings of one
key apart, and the width is what it states for 834 of its rows. Reading a cell's width as
descriptive and its job as the condition makes the column mean two different things in one
cell, and makes it mean nothing at all for `U+002C`, whose three listings are separated by
the width and by nothing else.

§3.9.2's sentence is a description of a class, not a membership predicate. It opens
"Sequences of European numerals …", and a sequence is not something one occurrence is; the
class is enumerated by §A.24, which §3.9.2 hands the membership to in its own opening
sentence. Reading the description as the test would also empty the class of its ten numerals
whenever a caller declares no job, which §3.2.6's Note contradicts directly — a half-width
European numeral mixed with Japanese text is cl-24 with no job declared anywhere.

And the alternative answers with a class the specification refuses on the frame. Under the
job reading a quarter-em comma outside a numeral is not cl-24, and §3.1.2 has already said
it is not the frame cl-07 is set on, so the occurrence falls to the residual tie-break of
[ambiguous-context](ambiguous-context.md) and is answered cl-07 anyway — a class whose stated
advance the caller's own input contradicts. The width reading answers with the one listing
that describes what the caller measured.

None of the three reaches `U+0020`, which is why the reading above is qualified rather than
general. The first turns on `U+002C`'s three listings being separated by the width and by
nothing else; §A.26's listing of `U+0020` is separated from §A.24's by nothing at all, since
it states no width to be separated by. The third turns on §3.1.2 having already ruled the
other class out; §3.1.2 says nothing about cl-26, and §D says the opposite — a Western word
space is *reduced to* a quarter em, so that width is one it has. The reading answers with
the listing that describes what the caller measured, and where a listing outside a construct
still describes it, that is the listing.

The alternative is not thereby wrong, which is why it is a `Question` rather than a
correction: `classification.grouped_numeral_qualification` permits `by-width` and `by-role`,
`Policy::JLREQ` selects `by-width`, and
`A.7/comma-in-horizontal-composition/quarter-em-advance` publishes both with the sentence
each rests on. `A.25/space/quarter-em-frame-outside-a-unit-symbol` publishes the boundary of
the reading on the other side, and `A.24/space/first-row-of-the-table` publishes the same
key at the same width *with* the digit-grouping role declared, which is how this format
carries §3.9.2's "used as a decimal place indicator in numbers" for an implementation that
classifies one occurrence at a time.

## What would change it

A revision of §3.9.2 that states the class as a membership test rather than as a
description, or a Remarks cell that separates the width from the job. Evidence that
publishers set a digit-grouping comma at a quarter em outside a numeral would not change the
reading on its own — it would be recorded as a `disagreements` entry on the conformance
case, which is what that field is for.
