# ADR-0018: an item is one occurrence, and a text is well formed against Appendix A

- Status: accepted
- Date: 2026-08-06

## Context

[ADR 0008](0008-classification-is-a-function-of-an-occurrence.md) decided that a class is a
function of an occurrence the caller describes. It did not say what an occurrence *is*, and
the two halves of the design answer differently.

`Text` validates that its items' byte offsets are strictly increasing, land on character
boundaries, and stay in range. `classify` looks up a key in Appendix A. Nothing connects
them, so two mismatches are representable and both are silent.

An item may cover more than one Appendix A key. §3.2.1's own example of Western text inside
Japanese is the word `editor`, which is six cl-27 members; a shaper that forms the `ffi`
ligature hands the caller one glyph covering three. An item carries one advance, one frame,
one role and one scale, so it cannot describe a multi-key cluster whose keys disagree about
any of them, and `classify` would answer with whichever key it happened to reach.

A key may also be split across two items. Appendix A keys twenty-five entries on an ordered
pair, and both kinds of pair are things a shaper legitimately emits as two glyphs.
`<02E5, 02E9>` is a cl-27 falling tone contour whose first code point is *also* listed alone;
splitting it yields two plausible cl-27 answers instead of one correct one. `<31F7, 309A>` is
a cl-11 small kana with a combining semi-voiced mark whose second code point is listed
nowhere; splitting it yields cl-11 followed by `Classified::Unlisted` — a published reading of
a silence, applied to a code point the specification does list.

A third gap sits in the same constructor and has the same shape. §3.1.2 states that the
character advance of commas (cl-07), full stops (cl-06), opening brackets (cl-01), closing
brackets (cl-02) and middle dots (cl-05) is half-width, and
[ADR 0017](0017-normalized-line-geometry.md) makes the declared frame (字幅) decide whether
the conditional space is added to the supplied advance or trimmed out of it. `Frame`'s
default is `Unstated`, and ADR 0017 defines neither branch for it. `」`, `。` and `、` are
each in exactly one class, so the existing diagnostic — which fires on an unstated frame over
a *multi-class* key — never sees them. The commonest punctuation in Japanese therefore
reaches composition with no frame, no diagnostic, and no defined geometry, which is the
half-em systematic error ADR 0017 exists to prevent, arriving through a default value instead
of a wrong declaration.

All three are properties of the caller's input, discovered at the same moment, in the same
walk over the items.

## Decision

An item is one occurrence of one Appendix A key, and a `Text` that does not satisfy that has
no representation.

`Text::new` performs the check, which means `Text` moves from `jlreq-unit` to `jlreq-class`.
That is the substance of the decision rather than a detail of it: a type whose validity is a
statement about Appendix A cannot live in a crate that cannot read Appendix A, and putting it
there is what produced these three holes. `Item`, `ItemIndex` and `ByteOffset` stay in
`jlreq-unit`, because they are vocabulary rather than claims; every crate that names a `Text`
already depends on `jlreq-class`, so nothing else moves and no crate is added.

Three refusals, each with its own error and its own reason.

A key that begins inside one item and ends in another is refused. The caller merges the two
glyphs into one item whose advance is their sum, which is what the pair already is: a member
is one occurrence, and no cell of any of the six matrices is ever indexed inside one, so
nothing is lost by stating it as one advance.

An item covering more than one key is refused **unless** it declares `Frame::Proportional`
and every key in it is listed in cl-27. That exception is not a concession; it is the only
shape a shaper produces. §3.2.6 puts proportional Western characters in cl-27, so a
proportional multi-code-point cluster is a Western ligature and nothing else, and Table 1
sets cl-27 against cl-27 solid while §C.2 note 12 requires a caller-supplied hyphen before a
Western word may be divided at all. There is consequently no amount and no break inside such
a cluster for the merge to have destroyed. Every other class is one key per item.

An item is refused when its frame is unstated and any class Appendix A names it under is one
of §3.1.2's five. The frame is not optional there, because there is no answer to report
instead: an unstated *class* has candidates and a separating axis to return, and an unstated
*geometry* has neither. This does not weaken the `Unstated` answer machinery, which exists
for the cl-19 against cl-27 axis and is untouched.

With the frame required where it decides geometry, the geometry rule becomes total and is
stated once: a conditional space at a boundary lies **inside** the supplied advance when the
item declares `Frame::FullEm`, and is **added** to it on every other declared frame. A modern
font reporting one em for `」` is the `FullEm` case, and a half-em, third-em, quarter-em or
proportional advance contains no conditional space by construction.

Two smaller consequences of the same walk are settled here rather than left to an
implementer. A candidate break at byte offset zero or at the end of the text names the
paragraph's own edges rather than an interior break; both are accepted, because every UAX #14
implementation an adopter already runs emits the second, and neither creates a line. And an
annotation stream is validated identically, by the same routine — annotation text is
classified too — so a `Text` and an annotation differ in the ordinal type that indexes them
([ADR 0016](0016-annotation-text-is-a-second-stream.md)) and in nothing else.

## Consequences

Classification becomes total over items, which is what makes `classify(text, index, policy)`
have an answer at all. So does the boundary evaluator: an adjacency between two items is an
adjacency between two keys, which is what Appendices B through E are indexed by.

The caller does more segmentation work, and it is work the caller is better placed to do
than we are: it holds the shaping output, and the only case where it must merge glyphs is a
pair Appendix A already prints as one key. The three refusals name the item and the reason,
so the work is directed rather than guessed at.

This is decided at M0 for the same reason ADR 0016 was. The `items` array is the part of a
published conformance case that
[ADR 0012](0012-outcome-and-detail-compatibility.md)'s compatibility regime cannot absorb:
a new field is detail and a new variant is detail, but re-segmenting every case's input is
neither.
