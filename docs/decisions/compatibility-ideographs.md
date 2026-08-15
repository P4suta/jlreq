<!--
SPDX-FileCopyrightText: 2026 kumihan contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: the CJK Compatibility Ideographs

- Applies to: `jlreq_class::classify`, `jlreq_class::resolve`, `jlreq_class::fold_compatibility`
- Standing: `Unstated`
- JLReq: §A.19, §A preamble

## The silence

§A.19 does not enumerate the ideographs. Its own text says so:

> In addition to CJK Ideographs, ideographic characters (cl-19) also includes some handful
> of other symbols. The following is the list of all non-ideographic characters assigned to
> this character class.

So the 465 rows of that table are the class's complement, and "CJK Ideographs" is left to
the character database. JLReq names no Unicode property and no code-point range, and it
says nothing at all about the CJK Compatibility Ideographs — the 1002 code points at
`U+FA0E`–`U+FA2D`, `U+FA30`–`U+FA6D`, `U+FA70`–`U+FAD9` and `U+2F800`–`U+2FA1D` that Unicode
encoded for round-trip compatibility with earlier national standards and that mostly
duplicate a unified ideograph.

Unicode splits them in two, and the split is not arbitrary. Twelve of them have no canonical
decomposition — `U+FA0E`, `U+FA0F`, `U+FA11`, `U+FA13`, `U+FA14`, `U+FA1F`, `U+FA21`,
`U+FA23`, `U+FA24`, `U+FA27`, `U+FA28`, `U+FA29` — and are therefore `Unified_Ideograph=Yes`
like any other kanji. Every other one canonically decomposes onto the unified ideograph it
duplicates, and is `Unified_Ideograph=No`.

## The reading

**`Unified_Ideograph=Yes` is the whole of the cl-19 predicate, and no canonical
normalization is applied.**

So the twelve non-decomposing compatibility ideographs classify as cl-19 with
`Standing::Normative`, exactly as `U+4E00` does. The decomposing ones — `U+FA10` 塚,
`U+FA12` 猪, `U+FA15`–`U+FA1E`, `U+FA20`, `U+FA22`, `U+FA25`, `U+FA26`, `U+FA2A`–`U+FA2D`
and all of `U+2F800`–`U+2FA1D` — are listed in no Appendix A table and claimed by no
predicate, so `classify` answers `Classified::Unlisted` and `resolve` applies
[unlisted-code-point](unlisted-code-point.md): cl-19 on every frame but the proportional
one, with `Standing::Unstated`.

The class is therefore right in ordinary Japanese text and the standing says the
specification did not decide it. That combination is the point of the reading, not an
accident of it.

## Why

Three things pull in the same direction.

**The folding is fixed by [ADR 0008](../adr/0008-classification-is-a-function-of-an-occurrence.md)
and folding these would break it.** Appendix A's preamble requires the Wide and Narrow
compatibility decompositions to be folded, because real text carries `U+FF08` where the
appendix keys `U+0028`. The ADR permits those two mappings and nothing else, for a measured
reason: full compatibility folding maps `U+2160` Ⅰ, a genuine cl-19 member listed in §A.19's
own table, onto the letter `I` and out of the class. A canonical decomposition of `U+FA10`
onto `U+585A` is a different mapping again, and admitting it would put a third folding into
a lookup whose two are already the subject of a written decision.

**Normalization is the caller's, by [ADR 0003](../adr/0003-layer-above-icu4x.md).** kumihan
sits above the Unicode toolchain rather than beside it: a caller who wants `U+FA10` treated
as `U+585A` runs NFC first, which is one line of `icu_normalizer` and is exactly what NFC is
for. Doing it here would be this library reimplementing a normalization it declines to own,
and doing it *silently* would make the class of an occurrence depend on a transformation the
caller never asked for.

**The set `Unified_Ideograph` admits is the right one for a Japanese context anyway.** The
twelve it includes are the compatibility ideographs Unicode itself keeps distinct, because
nothing in the standard says they are the same character as anything else; the ones it
excludes are, by their own canonical decomposition, duplicates of characters already in the
class. A document carrying a decomposing compatibility ideograph is carrying a legacy
round-trip artifact, and answering cl-19 for it — which `resolve` does — is what a
compositor would do.

The alternatives were considered and are worse for stated reasons. `Ideographic=Yes`
over-covers, pulling in Tangut, Nushu and Khitan, which no Japanese line composition rule
mentions. `Script=Han` over-covers differently: it includes `U+3005` 々, which JLReq puts in
cl-09 and which §C.2 note 1 then moves into cl-19 *by policy*, so a predicate that put it
there unconditionally would make a permitted alternative unavailable. `Unified_Ideograph`
excludes `U+3005` and `U+303B` exactly as JLReq needs, and it is the only one of the three
that does.

## What would change it

A revision of §A.19 that names a Unicode property, or that enumerates the compatibility
ideographs one way or the other. A conformance case establishing that publishers set a
decomposing compatibility ideograph differently from the unified ideograph it duplicates —
which would be evidence that the two are not interchangeable at all, and would bear on the
folding as much as on this reading.

Not a change: an adopter who wants NFC. They already have it, before the text reaches
`Text::new`, and the reason this reading is publishable is that taking it costs them one
call and leaves the specification's own silence visible in every `Standing` this crate
returns.
