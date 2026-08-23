# ADR-0016: annotation text is a second item stream

- Status: accepted
- Date: 2026-08-06

## Context

[ADR 0002](0002-caller-supplied-metrics.md) makes the caller's text and its measured
advances the single carrier of what jlreq is told. The obvious model is one array of
items per paragraph, and every construct naming a range of it. That model is right for
three of the constructs and wrong for two, and the difference is not stylistic.

Tate-chu-yoko (縦中横) and warichu (割注) characters are running text. They appear in the
document in reading order, the caller's UAX #14 implementation ran over them and its
candidate positions inside them are meaningful, they belong in the byte range a line
covers, and a search engine reading the document reads them in place. A range of the
paragraph's own items is exactly what they are.

Ruby (ルビ) text is none of those things. It has its own size, its own advances, its own
classes, and it never participates in a base-text adjacency: no cell of Table 1 is indexed
by "a base character beside the ruby attached to it". If ruby items lived in the base
stream then `members` and `boundary` would be asked questions JLReq does not contemplate,
the line's byte range would cover text that is not on the line, the caller's break
candidates would have been computed over a string with annotations interleaved into it,
and the inline cursor would count every ruby advance twice.

Emphasis dots (圏点) are a third case again, and §3.3.9 settles it in two sentences: the
symbol is one character chosen once for the run, and "the character size of emphasis dots
is the half size of the base characters". One symbol repeated at a stated size is not a
character string at all.

Reference marks (合印) are both cases at once. §4.2.3 gives two styles: the mark set in the
line just after the target word, and the mark set in the line gap beside it.

## Decision

A `Text` is one stream of items over one string in reading order. An annotation is a second
`Text`.

A construct that carries characters of its own owns the `Text` that holds them and names
the range of the annotated stream it attaches to. A construct over running text names a
range of the stream it sits in and owns nothing. Ruby and the interlinear style of
reference mark are the first kind; tate-chu-yoko, warichu, the ornamented character complex
(cl-21), furiwake (振分け), and the in-line style of reference mark are the second.

An ordinal into an annotation stream is a **different type** from an ordinal into running
text. The earlier version of this decision made that a documented invariant on the grounds
that every function taking an ordinal takes its stream beside it, and two places in the
design falsified it: a ruby run pairs a base range with an annotation range, both of the same
type, kept apart by field order alone; and the ruby constructor receives the annotation but
never the annotated text, so it structurally could not have validated the base range against
anything. An invariant that the library does not hold is not one the conformance runner can
be said to mirror.

So `ItemIndex` indexes running text and `AnnotationIndex` indexes an annotation, a swap is a
compile error rather than a review finding, and the ruby constructor takes both streams and
validates both ranges. `ByteOffset` stays one type, and the asymmetry is deliberate: a byte
offset is only ever dereferenced through the stream that owns the item carrying it, and the
two places a bare byte offset appears in the surface — a break candidate and a line's byte
range — are running text by definition, because annotation streams are not broken into lines.

Annotation streams do **not** nest, and that is a correction of the earlier reading rather
than a simplification of it. §3.4.2 says the symbols in a warichu interior are handled as in
the main text and a warichu interior can carry ruby, which is exactly why nesting is absent:
a warichu interior *is* running text and its items are the paragraph's own, so ruby inside
one attaches to a range of the paragraph. Every construct that owns a stream attaches to
running text, and no construct that owns a stream sits inside another's — JLReq defines no
ruby on ruby, and no reading of a reading. Depth is therefore exactly one, which is what
makes two ordinal types sufficient where an unbounded nesting would have needed a stream
identifier threaded through every signature.

Emphasis dots carry no stream. They are one member, repeated once per base item, at half
the base size, and both facts are §3.3.9's rather than the caller's. That is why they have
no character class and no row in any of Tables 1 through 6: there is nothing there to
classify. The hole is published as a case rather than filled.

The conformance case format carries annotation streams from M0, alongside the base stream
and with the same shape.

## Consequences

A line's item range and byte range are the base stream's, and they stay honest: the text a
line covers is the text a reader reads on that line.

The caller declares two arrays where it might have expected one. That is not extra work: a
shaper shapes ruby as a separate run already, because it is set at a different size, so the
second array is a thing the caller is holding when it calls.

Placement of annotations is reported separately from placement of the base, which is what a
renderer wants, since it draws them with a different size and on a different axis.

This is decided at M0 because it is not absorbable later.
[ADR 0012](0012-outcome-and-detail-compatibility.md)'s compatibility regime covers a new
variant and a new field; it does not cover a change to what an index means, and the
`input` object of every published case would have to be rewritten
([ADR 0006](0006-conformance-suite-as-artifact.md)). That reasoning is why the second ordinal
type is introduced now: adding it at M4 would have moved every published case's ruby base and
run pairing at once, which is the one edit this regime cannot make quietly.
