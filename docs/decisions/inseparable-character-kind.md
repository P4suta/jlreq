<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: what makes two adjacent inseparable characters (cl-08) "of different kinds" in §E.2 note 4

- Applies to: `jlreq_spacing::evaluate::boundary` (§E.2 note 4's `cl_08_same_kind`, read by
  `note_governed_expansion`)
- Standing: `Unstated`
- JLReq: §E.2#4, §C.2#5, §C.3, §A.8

## The silence

§E.2 note 4 states:

> A third order opportunity exists for inter-character spacing expansion, to take up to a
> maximum of a quarter em space, with respect to the corresponding character size, between two
> consecutive inseparable characters (cl-08) which are of different kinds.

cl-08 (`spec/derived/appendix-a.tsv`'s own six rows) lists exactly six members: EM DASH
(U+2014), HORIZONTAL ELLIPSIS (U+2026), TWO DOT LEADER (U+2025), VERTICAL KANA REPEAT MARK
UPPER HALF (U+3033), VERTICAL KANA REPEAT WITH VOICED SOUND MARK UPPER HALF (U+3034) and
VERTICAL KANA REPEAT MARK LOWER HALF (U+3035). The note conditions its own quarter-em
opportunity on the two occurrences being "of different kinds" (別の種類の文字) — a word,
種類 ("kind"), the note never defines anywhere in its own sentence, over a six-member set whose
members are visibly not identical: three ordinary punctuation marks with no evident
relationship to one another, and three code points Appendix A's own Remarks column ties
together as variants of one repeat mark. Nothing in §E.2 note 4 itself says whether "kind"
means "this exact code point and no other" (in which case only the diagonal of the six-by-six
grid below is "same kind") or something coarser that groups some of the six members together
before asking whether two occurrences fall inside the same group.

## The reading

**Four kinds among cl-08's six members: the em dash alone, the horizontal ellipsis alone, the
two dot leader alone, and the vertical kana repeat mark's three code points — U+3033, U+3034
and U+3035 — together as one kind.** Two occurrences are "of different kinds" exactly when they
fall in different groups of this partition, symmetrically: kind is a property of each
character, not a fact about which one came first (unlike §C.2 note 5's own five *ordered*
pairs, discussed below). `jlreq_spacing::evaluate::cl_08_same_kind(before, after)` answers
`true` when `before == after` (an identical character is certainly its own kind) or when both
members are drawn from `{U+3033, U+3034, U+3035}`, and `false` otherwise — including when
either side resolves no member at all, the same total absence
`crates/jlreq-spacing/src/evaluate.rs`'s own `inseparable_member_pair` states for the identical
reason.

The full grid, all thirty-six ordered pairs, against three candidate readings — R1 (a "kind" is
exactly one code point: same iff `before == after`), R2 (the reading above: same iff the two
members share one of the four groups), and R3 (a "kind" is defined by the complement of §C.2
note 5's own five named pairs: same iff the ordered pair is one of those five):

| before | after | R1 (member identity) | R2 (mark family, adopted) | R3 (complement of §C.2#5) |
| --- | --- | --- | --- | --- |
| — | — | same | same | same |
| — | … | different | different | different |
| — | ‥ | different | different | different |
| — | 〳 | different | different | different |
| — | 〴 | different | different | different |
| — | 〵 | different | different | different |
| … | — | different | different | different |
| … | … | same | same | same |
| … | ‥ | different | different | different |
| … | 〳 | different | different | different |
| … | 〴 | different | different | different |
| … | 〵 | different | different | different |
| ‥ | — | different | different | different |
| ‥ | … | different | different | different |
| ‥ | ‥ | same | same | same |
| ‥ | 〳 | different | different | different |
| ‥ | 〴 | different | different | different |
| ‥ | 〵 | different | different | different |
| 〳 | — | different | different | different |
| 〳 | … | different | different | different |
| 〳 | ‥ | different | different | different |
| 〳 | 〳 | same | same | **different** |
| 〳 | 〴 | different | **same** | different |
| 〳 | 〵 | different | **same** | same |
| 〴 | — | different | different | different |
| 〴 | … | different | different | different |
| 〴 | ‥ | different | different | different |
| 〴 | 〳 | different | **same** | different |
| 〴 | 〴 | same | same | **different** |
| 〴 | 〵 | different | **same** | same |
| 〵 | — | different | different | different |
| 〵 | … | different | different | different |
| 〵 | ‥ | different | different | different |
| 〵 | 〳 | different | **same** | different |
| 〵 | 〴 | different | **same** | different |
| 〵 | 〵 | same | same | **different** |

R1 and R2 agree on twenty-seven of the thirty-six pairs and diverge on exactly six — the
off-diagonal ordered pairs drawn from `{U+3033, U+3034, U+3035}` (bold above). R3 is refuted
outright, not merely rejected on balance: it calls `〳` against itself, `〴` against itself and
`〵` against itself "different kinds" (bold above), because none of the three is one of §C.2
note 5's own five named pairs — three cases of a character being "of a different kind" from
itself, which 別の種類の文字 cannot mean under any reading of "kind" as a property a character
either shares with another or does not.

The six bold "same" cells above are not all settled the same way, and this file separates the
three tiers rather than letting the closing claim below flatten them into one. **Two cells rest
on a JLReq sentence directly**: `〳` followed by `〵`, and `〴` followed by `〵` — the two
crossings §C.2 note 5 itself names, in that order, identified with §E.2 note 4's own "kind"
through the §C.3 addendum argued below. **Two more rest on the symmetry step alone**: the
reverse orderings, `〵` against `〳` and `〵` against `〴`. No sentence anywhere names either
reverse crossing; they follow only from reading "kind" as a property of one character rather
than of an ordered pair (the third "Why" argument below, which itself says a partition read this
way is "naturally read as" the consequence, not a sentence JLReq states) — a shorter, better
anchored inference than the transitive step below, but an inference and not a quotation either
way. **The last two rest on a second inference layered on top of the first**: `〳` against `〴`
(and its reverse). Two symmetry-established facts license this pair — `〳`~`〵` and `〴`~`〵`,
both from the tier above — and the partition shape `cl_08_same_kind` takes is the only shape
that stays total and answers every one of the thirty-six pairs without a fifth, order-specific
carve-out; but "kind" being transitive (if X and Z share a kind, and Y and Z share a kind, then
X and Y share a kind) is a second assumption this reading makes, not a sentence JLReq states
and not implied by the symmetry step alone. No text anywhere in the specification pairs U+3033
with U+3034 directly, and no real, well-formed text ever places them adjacent to one another (a
kunojiten mark is always written upper half then lower half, never two upper halves in a row) —
the pair is unobservable in practice, the one cell (and its reverse) that rests on transitivity
rather than on symmetry alone. A reader who rejects transitivity for 種類 need only change this
one relationship, `〳`~`〴` (two cells); a reader who rejects symmetry too need also change the
two reverse crossings above. Every other cell of this grid — the diagonal and every cross-family
cell — is decided without either inference: the diagonal because an identical character is
certainly its own kind, and every cross-family cell because no sentence anywhere links a mark
outside the kunojiten family to one inside it or to another mark outside it, so no reading of
"kind" this file considers groups them.

## Why

**The partition, at every cell but one (and its reverse), is not a guess.** Two arguments,
both from the specification's own text:

**§C.3's own addendum uses the identical word for exactly the pairs §C.2 note 5 forbids.** The
addendum's Very loose level (レベル1) relaxes several listed categories that Table 2 otherwise
forbids, and one of them is stated as "Inseparable characters (cl-08) of the same kind"
(同一の種類の分離禁止文字（cl-08）) — not "of the same code point", not "identical marks", but
"of the same kind", the exact word 種類 §E.2 note 4 itself uses. §C.2 note 5's own closed
enumeration is the only place Table 2 states an inseparability restriction over cl-08 at all,
so the addendum's "same kind" category can only be naming the pairs that restriction covers:
(—,—), (…,…), (‥,‥), (〳,〵) and (〴,〵) — an identification from the one restriction available
to be identified with, rather than an assumption. JLReq is therefore its own witness that a
kunojiten crossing — 〳 followed by 〵, or 〴 followed by 〵 — is a "same kind" pair in the
identical sense the word carries in §E.2 note 4, not merely a pair two different code points
happen to be inseparable at. This settles the two named crossings directly, through that one
identification rather than through a sentence that names them by their own code points.

**§A.8's own Remarks column states the two upper-half variants are partial forms of one
composite mark, not two independent marks.** The Remarks cell for both U+3033 and U+3034 reads
"used in vertical composition / U+3035 follows this" (縦組で使用 / この文字の後ろにU+3035が配置
される); U+3035's own cell reads only "used in vertical composition", naming no successor of
its own. Appendix A is stating, in its own descriptive column, that U+3033 and U+3034 are each
one half of a two-glyph mark whose other half is always U+3035 — the plain kunojiten (〳〵) and
the voiced kunojiten (〴〵) — rather than three independent marks that happen to look similar.
Reading "kind" coarser than code-point identity for this family, and not for the em dash, the
ellipsis or the two dot leader (each listed with no such cross-reference), is exactly the
distinction Appendix A itself draws.

**A "kind" is grammatically a property of the character, not of the pair.** Both the English
("which are of different kinds") and the Japanese (とは別の種類の文字, "a character of a
different kind") predicate "kind" of one occurrence, describing what it *is*, not what it forms
with its neighbor the way §C.2 note 5's own inseparability list does ("these two characters are
inseparable"). A property predicated of individual characters is naturally read as inducing a
partition — symmetric, and (barring a specific textual reason not to) transitive — rather than
an order-specific relation between exactly two named items the way §C.2 note 5's own five pairs
are. This is also why `cl_08_same_kind` is symmetric in its two arguments where
`inseparable_member_pair` is not: the two functions are answering different kinds of question,
not the same question read twice.

**The transitive step (`〳`~`〴`) follows from taking "kind" seriously as a partition rather
than from a sentence.** Once 〳~〵 and 〴~〵 are both established "same kind" facts (the two
arguments above), refusing 〳~〴 would require "kind" to be a relation that is not transitive —
coherent in principle, but nothing in JLReq's text suggests it, and the natural reading of a
classification word predicated of individual characters is an equivalence relation, whose
classes are closed under exactly this kind of inference. The alternative — treating "kind" as
directional or as a list of named pairs rather than a partition — was considered and rejected:
it would require this reading to either duplicate §C.2 note 5's own five-pair enumeration under
a different name (which `inseparable_member_pair`'s own doc already argues would answer a
different question) or invent a sixth, unstated pair-list of its own, neither of which the
specification's text motivates the way the partition does.

The alternative reading, R1, was considered and rejected as this project's own answer:
member identity alone answers §C.2 note 5's own inseparability question correctly (that note
truly is an ordered, closed enumeration, and `inseparable_member_pair` reads it that way on
purpose), but §E.2 note 4 is asking a different question in different words, and §C.3's own
"same kind" vocabulary is direct evidence that JLReq itself does not mean R1 by 種類 — if it
did, the addendum would have written "the same character" rather than reaching for a broader
word to describe the identical five pairs.

R3 — reading §E.2 note 4's "different kinds" as the exact logical complement of §C.2 note 5's
own five pairs — was considered and rejected outright, not merely disfavored: it is refuted by
the three diagonal cells shown in bold above, each an identical character called "of a
different kind" from itself. No coherent reading of 種類 produces that answer, so R3 cannot be
what either note means, independent of anything this file argues for R2 over R1.

## What would change it

A revision of §E.2 note 4, or a JIS X 4051 commentary on it, that names "kind" directly over
cl-08's six members — the way §B.2 note 3 names "preceding" and "trailing" directly for a sum
rather than leaving this reading to infer an owner from ADR-0014's general convention — would
settle every cell of the grid above from the text alone, including the one this reading leaves
to an inference from partition shape rather than to a sentence.

Evidence that JLReq intends U+3033 and U+3034 to be treated as different kinds from one another
specifically — a passage distinguishing "the voiced kunojiten's own kind" from "the unvoiced
kunojiten's own kind" as two things, rather than treating both uniformly as "vertical kana
repeat marks" the way §A.8's own class heading does — would be the direct evidence this
reading's own "Why" section says it lacks for the transitive step, and would need
`cl_08_same_kind` to special-case that one ordered pair (and its reverse) rather than falling
out of the four-group partition automatically.

The conformance suite now carries §E.2 note 4 at the two cells both readings agree on, and for
different reasons: `E.2/two-em-dashes/an-identical-inseparable-character-is-not-a-different-kind`
is the grid's own diagonal, where R1 and R2 both answer "same" because an identical character is
certainly its own kind; `E.2/em-dash-then-horizontal-ellipsis/two-kinds-open-a-third-stage-quarter-em`
is a cross-family, off-diagonal cell, where R1 and R2 both answer "different" because no reading
links an em dash to a horizontal ellipsis at all. Neither case touches the six ordered pairs
where R1 and R2 diverge, and none can: `xtask/src/conform.rs`'s own `check_standing` requires an
`unstated` or `adjudicated` case to publish at least two `permitted` readings, and no question in
`spec/derived/questions.tsv` is addressed to `E.2#4` (checked directly: no row's `address` column
reads it) to key a second reading on. `docs/decisions/README.md`'s own rule — that the
conformance suite carries every reading here with all of its readings — is consequently unmet
for R1 today, not by oversight but for want of a policy-selectable home for it. A
`spacing.inseparable_kind` question added to `xtask/src/policy.rs`, on the model of
`classification.ambiguous_context` and `adjustment.remainder` — both `Permission::Silent`, both
quoting their own address's sentence verbatim as each answer's `statement` — is the specific
change that would give R1 that home: once such a question exists, a case landing on one of the
six divergent pairs could carry `standing: adjudicated` with both readings under `permitted`,
exactly as this file's own grid already distinguishes them. Evidence that publishers actually
open inter-character spacing between the two halves of one kunojiten mark (splitting 〳〵 or 〴〵
across an expanded quarter em rather than treating them as one glyph pair) would then be
recorded as a `disagreements` entry on that case, once it exists — not before.
