# ADR-0017: a line is stated in normalized geometry, and every trim is reported

- Status: accepted
- Date: 2026-08-06

## Context

[ADR 0008](0008-classification-is-a-function-of-an-occurrence.md) makes the frame (字幅)
the caller's statement of what its supplied advance covers, and identifies §3.1.2 as the
sharpest reason the caller must speak first: the character advance of commas (cl-07), full
stops (cl-06), opening brackets (cl-01), closing brackets (cl-02) and middle dots (cl-05)
is half-width, and Table 1's amount is what "makes them appear as if they were intrinsically
full-width". A modern font reports one em for the same glyph, with the blank half already
inside the advance.

So the same geometry is reached from two directions. On the half-em frame the conditional
space is *added* to the supplied advance; on the ideographic frame it is already inside it
and must be *trimmed*. Adding is representable — it is a positive amount at a boundary.
Trimming is not representable by anything the design had: a line reported placements and an
adjustment, and no output carried an effective advance differing from the one the caller
supplied.

The published case format makes this concrete and unavoidable, because every expected line
carries a trailing amount and an extent, and those two words have no definition yet. Two
implementers will define them differently on the first line they write, and because §3.1.2
covers the five commonest punctuation classes in Japanese, a disagreement is a systematic
half-em error across the whole corpus rather than an edge case.

## Decision

Composition works in one geometry, and it is the specification's. Every occurrence of those
five classes has its half-width intrinsic advance, and every conditional space is explicit
at a boundary. A caller-declared frame that already contains a conditional space is
normalized by subtracting that amount, and the subtraction is reported: `Line::trims` names
the item, the amount, the side it came off, and the rule that states it. kumihan therefore
never silently shortens an advance the caller supplied — it reports every unit it took and
cites the sentence, which is what [ADR 0002](0002-caller-supplied-metrics.md) requires of
anything that touches a caller's measurement.

A trim is not a negative conditional space. Appendix D gives a conditional space a reduction
priority, and that priority is a property of the space, unchanged by which side of an
advance boundary the space currently sits on. A trim states where a space already lives; it
does not assert that a second space exists.

Three output quantities are defined once, here, and are the same in the library and in the
case files.

*Placements* are the caller's own glyph-box origins. Add the advance you supplied to a
placement and you have your own box. A trimmed item's box may extend past the line's extent,
covering the blank half of a punctuation em; and an item whose trim came off its leading
side — an opening bracket set solid at the line head — receives an origin *before* the
line's start, which is correct and is stated rather than clamped.

*Extent* is the normalized geometry: from the line-head origin to the line end, including
the realized conditional space at the line end and excluding any character placed outside
the measure by §2.5.1's hanging punctuation. It is the quantity compared against the
measure.

*Trailing* is the realized conditional space at the line end, whether or not it lives inside
the last item's supplied advance.

Placements carry one axis and one origin, and never two. That needs saying because a segment
(§3.2.5, §3.4.2, §3.7.2, §3.7.3) contains items the line does not lay out as ordinary inline
text, and the obvious reading — that a segment's interior positions are also in the
placements slice, in the segment's own frame — would put two coordinate systems under one
type and reintroduce exactly the axis mix
[ADR 0011](0011-typed-axes-and-direction-as-a-datum.md) exists to make impossible. The
specification does not require it. Three of the four interiors run along the line's own inline
axis and their items' origins are ordinary placements, offset on the block axis by the
sub-line they landed in. The fourth, §3.2.5's tate-chu-yoko, sets its run "from left to right"
across a vertical line and centers the whole string on it — so every interior item shares the
segment's inline position, and what distinguishes them is where they sit *across* the line,
which is the block axis. Their inline origin is therefore the segment's, stated in the
placements slice like every other item's, and their spread across the line is a separate
block-axis slice on the part. One entry per item, one axis per type, and no interior
coordinate space anywhere.

The point of those definitions is the property they produce: the same text declared on
either frame composes to identical placements, identical trailing and identical extent. The
two readings of §3.1.2 are then a differential pair of conformance cases with byte-identical
expectations, and an implementation that adds Table 1's amount to an advance that already
contains it passes the first and fails the second.

## Consequences

A renderer needs nothing beyond the placements it is given and the advances it supplied. A
renderer that wants the normalized cell — for a selection highlight, an underline, or a
background fill — reconstructs it from the trims, which is the only consumer that needs
them.

The definitions are a one-way door, because they are baked into every expected value the
suite publishes. That is the reason they are settled at M0, before the first case is written
rather than after the hundredth.

A caller that declares the ideographic frame for text it did not set that way still gets a
loud answer rather than a quiet one: the trim is reported, and the frame contradicting the
supplied advance is already a diagnostic.

A caller that declares *no* frame on one of the five classes gets no answer at all, because
[ADR 0018](0018-an-item-is-one-occurrence.md) refuses to build the text. The add-versus-trim
decision above has two branches and the default frame is in neither; the choice was to define
a third geometry or to remove the state, and removing it is the only option that does not put
a guess at the commonest adjacency in Japanese. With that state gone the rule is total in one
sentence: the conditional space is inside the advance on the ideographic frame and added on
every other, so the two frames of §3.1.2 are the whole of it.

Every trim is also checkable against the tables rather than merely reported. A trim's rule
must resolve to §3.1.2 or to a Table 1 cell that states a conditional space of that amount
with that referent, and `conform --check` asserts it for every published case — so an
implementation cannot discharge an overlong line by subtracting an arbitrary quantity from a
caller's advance and labeling it a trim.
