<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: how many sides of an inserted space §3.7.3 opens, where its two renderings differ

- Applies to: the jidori round in
  [`pipeline`](../../crates/jlreq/src/pipeline.rs), and the same round in
  [`engines/ocaml/lib/pipeline.ml`](../../engines/ocaml/lib/pipeline.ml)
- Standing: `Adjudicated`. This is a `divergent` permission in the sense
  `xtask/src/policy.rs` defines — the two renderings of one sentence do not state the same
  rule — and not a silence. `spec/derived/questions.tsv` carries no row for it today, so no
  answer this engine produces is tagged with that standing yet; adding the row is the change
  named under **What would change it**.
- JLReq: §3.7.3
- Observed by: `just census constructs`, the jidori variants; no built-in case reaches it

## The silence

§3.7.3's own list of what a jidori does states, of an inserted Western word space (cl-26) or
ideographic space (cl-14), two different rules in its two renderings of the same sentence.

In English:

> Where Western word space (cl-26) or full-width ideographic space (cl-14) are inserted, add
> the same spacing to those space characters as is being added to the other characters.

In Japanese:

> 欧文間隔（cl-26），和字間隔（cl-14）など空白を挿入してある箇所は，その空白の前及び後ろの2箇所
> ではなく，空白の前（又は後ろ）だけとする．空白の前後2箇所で空けると空き過ぎになる．

The English sentence says the space characters take the same spacing as everything else,
which in a jidori means both of the boundaries around them. The Japanese sentence says the
opposite in so many words — *not* the two places before and after the space, but the front
(or the back) alone — and gives its reason: opening both would leave too much space.

This is not a place where JLReq is silent. It is a place where JLReq states one rule twice
and the two statements are not equivalent. `spec/derived/` carries both texts, as it carries
both renderings of every rule, and settles neither.

## The reading

**Both boundaries around an inserted space are opened: the English reading.** A cl-26 or
cl-14 occurrence inside a jidori is treated as an ordinary member of the run, and the
boundaries on either side of it take the same share as every other boundary the run opens.

The reference engine answers this, and the OCaml engine matches it — not by copying, but
because the same argument reaches the same rendering.

## Why

**The English sentence states a rule; the Japanese one states an exception without stating
its scope.** "Add the same spacing to those space characters as is being added to the other
characters" is complete: it names the sites and the amount, and it composes with the rest of
§3.7.3's list, which is a list of things that are *not* opened. The Japanese sentence names
one of two boundaries — 前（又は後ろ）, "the front (or the back)" — and does not say which,
or how an engine chooses. A rule that leaves the choice between two visibly different layouts
to the word 又は is not implementable as stated; a reader would have to invent the
tie-break, and inventing one is exactly what this project's own posture forbids where a
rendering that needs no invention exists.

**Both renderings agree on the amount, and only on the count of sites do they part.** That
narrows the divergence to a question with two answers, one of which is the general rule
§3.7.3 already applies to every other boundary. Choosing the reading that requires no special
case for cl-26 and cl-14 keeps the section's own list — which is a list of exceptions — the
whole of the exceptions.

**The Japanese sentence's reason is a typographic judgment rather than a rule.** 空き過ぎに
なる — "it comes out too spaced" — is an argument for the rule and not part of it, and it is
an argument that depends on how much the run is being opened by. At a small adjustment it
does not hold; at a large one it holds for every boundary in the run and not only for the
ones around a space. A rule whose stated reason does not track the case it governs is the
weaker of the two statements.

**This is a judgment between two texts, not a discovery about one.** It is published under
`Adjudicated` rather than `Unstated` for that reason: the specification did decide, twice,
and this project is choosing between its decisions rather than filling a gap. An adopter who
sets Japanese text and follows the Japanese rendering is not violating JLReq, and a
conformance case for a jidori containing a Western word space should carry both answers.

## What would change it

A row in `spec/derived/questions.tsv` with `permission = divergent`, carrying both sentences
as its two `statements`, is the concrete change: it turns this file's answer from a published
reading into a selectable one, gives the `jis_reading` preset somewhere to select the
Japanese rendering, and makes both answers reachable through an explicit policy overlay. That
row is written in `xtask/src/policy.rs`, and it is the change this file exists to ask for.

A revision of §3.7.3 that brings the two renderings into agreement settles it outright. Until
either happens, an engine that follows the Japanese rendering will differ from this one at
every jidori that holds an inserted space and is opened at all — a shape the `constructs`
census reaches and no built-in conformance case does.
