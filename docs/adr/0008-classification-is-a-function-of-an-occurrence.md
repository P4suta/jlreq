# ADR-0008: a character class is a function of an occurrence, not of a code point

- Status: accepted
- Date: 2026-08-05

## Context

`jlreq-class` currently claims that "the class of a code point is a property of the
writing system, not of a document or a font." Measured against Appendix A, that is false,
and the measurement is not close. Appendix A enumerates 1133 distinct keys across 25
tables; 473 of them are named by more than one class, reaching degree four. Five classes —
cl-20 through cl-23 and cl-30 — enumerate nothing at all, because their section text reads
in full "Any character may participate in …". No total function from `char` to a class
exists to be written.

The disambiguators are visible in the Remarks column and stated mechanically in the prose.
§3.2.4 puts full-width and fixed-space Western characters in cl-19, §3.2.6 puts
proportional ones in cl-27 and half-width European numerals mixed into Japanese in cl-24.
That axis is a property of how the character was set, and 834 Remarks cells carry it, the
sole signal for 312 of the 317 cl-19/cl-27 pairs whose glyphs are identical.

Twenty-five entries key on an ordered pair of code points, and cl-27 lists `<02E5, 02E9>`
and `<02E9, 02E5>` as distinct members, so the lookup key is an ordered sequence.

§3.1.2 adds a second, sharper reason the caller must speak first. It states that the
character advance of commas (cl-07), full stops (cl-06), opening brackets (cl-01), closing
brackets (cl-02), and middle dots (cl-05) is half-width, and that Table 1's amount is what
"makes them appear as if they were intrinsically full-width." A modern font reports a full
em for the same glyph. A library that adds the Table 1 amount to a full-em advance
overshoots by half an em at the commonest adjacency in Japanese text, silently.

## Decision

Classification takes an occurrence the caller describes, not a code point. The unit is a
cluster identified by a byte offset into the caller's text, matched longest-first over
Appendix A's ordered sequences, after folding the Wide and Narrow compatibility
decompositions and nothing else — full compatibility folding would fold U+2160, a genuine
cl-19 member, onto `I`.

The caller supplies three facts it already holds. The frame (字幅) is what the supplied
advance covers, in JLReq's own vocabulary: ideographic, half em, third em, quarter em, or
proportional. It carries both the Appendix A Remarks axis and the §3.1.2 advance model at
once, because they are the same distinction: a closing bracket declared on the half-em
frame has the conditional space added to it, and one declared on the ideographic frame has
it trimmed out. The role is the syntactic job the document gives the occurrence, needed by
six code points and no others. The construct names which ruby, tate-chu-yoko (縦中横), or
warichu run this occurrence belongs to, carrying a run identity because §B.2 notes 9
through 11 and §C.2 notes 6 through 8 all turn on whether two neighbors are in the same
run.

The frame defaults to unstated rather than to a guess. An unstated frame on a multi-class
key is answered with the surviving candidates and the axis that would separate them, not
with a class, because a confident wrong class is the failure this library exists to
prevent.

What an occurrence *is* — one item, one Appendix A key, and the two mismatches that are
otherwise silent — is settled by [ADR 0018](0018-an-item-is-one-occurrence.md), which also
requires the frame on the five classes of §3.1.2, where an unstated frame has no answer to
report because it names a geometry rather than a class.

Every answer carries its provenance: which Appendix A section listed it, which axis
disambiguated it, which policy reclassified it, or that JLReq is silent and this project's
published reading applied.

## Consequences

The crate doc comment is corrected. What is a property of the writing system is the table,
not the answer; the occurrence is a property of the document, and the party that chose a
proportional glyph over a full-width one is the party that decided the class.

Integrations must supply a frame. That is real friction and it is unavoidable: 42 percent
of enumerated keys are ambiguous without one, and inferring it from the advance would be
measuring, which [ADR 0002](0002-caller-supplied-metrics.md) forbids and which is
undecidable anyway for a proportional glyph exactly one em wide.
