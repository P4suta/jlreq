<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: which keys the `warichu-bracket` role reaches cl-28 and cl-29 with

- Applies to: the classification round in [`spec`](../../crates/jlreq/src/spec.rs), and the
  same round in [`engines/ocaml/lib/spec.ml`](../../engines/ocaml/lib/spec.ml) and
  [`engines/racket/classes.rkt`](../../engines/racket/classes.rkt)
- Standing: `Unstated`
- JLReq: §3.9.2, §3.4.2, §A.01, §A.02, §A.28, §A.29
- Observed by: `just census constructs` (18,515 requests), the warichu variants

## The silence

Appendix A gives a warichu's brackets two classes of their own. §A.28 is

> Opening brackets for inline cutting note

and §A.29 the closing ones, and between them they enumerate six keys: `0028`, `3014` and
`005B` under cl-28, and `0029`, `3015` and `005D` under cl-29. Every one of those six is
*also* listed under §A.01 or §A.02, the ordinary opening and closing brackets, so the two
listings are a genuine ambiguity of the kind §3.9.2 concedes — and the protocol's own
`warichu-bracket` cluster role is the caller-supplied fact that separates them.

What no sentence says is what that role does to a key §A.28 and §A.29 do *not* list, and
§3.4.2 is the section that leaves the question open. It says a warichu

> usually has two lines, and is surrounded by LEFT PARENTHESIS "(" and RIGHT PARENTHESIS
> ")" characters that are double the size of the characters in the inline cutting note
> itself

— *usually*, which is the specification's own acknowledgement that a note is set in other
brackets too, and its own Note names a further style that uses no brackets at all. A
caller who sets a note in `300C` 「 and declares the role has therefore stated something
true about the text. Whether the role is a *declaration* — this occurrence is a warichu
bracket, so it is cl-28 whatever Appendix A enumerates — or a *disambiguation* — this
occurrence is the cl-28 reading of a key both sections list — is not stated anywhere, and
the two answers differ at every key outside the six.

The difference is not academic. cl-28 and cl-29 carry their own rows and columns in all
six matrices, so a bracket that reaches them takes different spacing, different
breakability and a different ladder cell from the same character read as cl-01 or cl-02.

## The reading

**The `warichu-bracket` role narrows a key Appendix A already lists under cl-28 or cl-29,
and promotes nothing.** The role is applied to an occurrence that is an opening or a
closing bracket, and the class it selects is kept only where Appendix A's own listings for
that key contain it. A note bracketed with `0028` is bracketed with a cl-28; a note
bracketed with `300C`, or with any other of §A.01's sixteen keys that §A.28 does not
enumerate, is a note bracketed with an ordinary opening bracket, and the role changes
nothing about it.

The role is not refused and no diagnostic is raised: a caller who declares it on 「 has
said something the engine agrees with about the *text* and nothing that Appendix A lets it
say about the *class*.

## Why

**A role is a fact about the occurrence, and Appendix A is the whole of what a class
listing is.** Every other role the protocol carries works this way — `unit-symbol` reaches
cl-25 at the keys §A.25 lists, `grouped-numeral` reaches cl-24 at the keys §A.24 lists —
and §3.9.2's own procedure is to read the key out of Appendix A first and let the caller's
facts choose among the survivors. A role that could *add* a class the table does not list
would be the one place in this engine where a caller writes a new row into Appendix A, and
`docs/adr/0009-generated-data-and-attested-transcription.md` is the reason that cannot be
right: the listings are transcribed, and a transcription the caller may extend is not one.

**The alternative reading has to explain the enumeration away.** §A.28 and §A.29 could have
been written as "the brackets of §A.01 and §A.02 when they delimit an inline cutting note",
and they were not: they list three keys each, which is a statement about which brackets
JIS X 4051 and JLReq take a warichu to be set in. Reading the role as a promotion makes
those two enumerations decorative — every key of §A.01 would reach cl-28 on request — and
leaves the six named keys with no work to do.

**The ambiguity the role does resolve is real and needs it.** `0028` is listed under cl-01,
cl-25, cl-27 and cl-28, and nothing in the occurrence itself separates them;
[ambiguous-context](ambiguous-context.md) is what answers when the caller declares
nothing, and its answer for `0028` is cl-01. The role exists so that a caller who *has* set
a warichu is not held to that tie-break. That is a job with a scope, and the scope is the
keys Appendix A lists.

**The census is what makes the difference observable.** No built-in case brackets a warichu
with anything but a listed key: all three set the note in `0028` and `0029`, which §A.28 and
§A.29 enumerate, so both readings answer them alike. The `constructs` census wraps its note
in `3008` 〈 and `3009` 〉 instead — the least ambiguous keys Appendix A lists for cl-01 and
cl-02, and keys §A.28 and §A.29 do *not* enumerate — with the role declared on both, and
sets that note against every class the matrices carry on either side of it.

That is where the two readings part. Table 1 states a half em after an ideographic character
(cl-19) before an opening bracket (cl-01) and *nothing* before a warichu opening bracket
(cl-28); Table 6 makes the first `residual` and gives the second a `0-1/4 stage 3` cell. A
promotion reading and a narrowing reading therefore compose the same paragraph two visibly
different ways, on a line that is set solid and on one that has room to spare alike.

## What would change it

A revision of §A.28 or §A.29 that states its list as open — "and other opening brackets
used for this purpose" — settles it in the other direction, and would be the natural place
for W3C to record what publishers actually set warichu in. A sentence in §3.4.2 that turned
its "usually" into a list of the brackets a warichu may take would do the same.

The concrete evidence would be a conformance case for a note bracketed with `300C`
carrying both classes as `disagreements`, which is what the suite is for: both readings are
publishable today and only one is selected.
