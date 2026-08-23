<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: what a request that states no `alignment` asks for

- Applies to: the line-adjustment round in
  [`pipeline`](../../crates/jlreq/src/pipeline.rs), and the same round in
  [`engines/ocaml/lib/pipeline.ml`](../../engines/ocaml/lib/pipeline.ml)
- Standing: `Unstated`
- JLReq: §3.8.1, §3.5.3
- Observed by: `just census tabs` and `just census widow`, in the `alignment-omitted`
  variants; among the eighty-nine built-in cases, only
  `3.5.4/widow-keeps-two-clusters-on-last-line` reaches it

## The silence

`crates/jlreq-conformance/protocol.schema.json` gives `alignment` four values and no default.
A request that omits the member is well-formed, and neither the schema nor
`docs/design/conformance.md` says what the omission means.

JLReq does not fill the gap either, because JLReq has no notion of a request. It states
§3.8.1's own default posture for a paragraph:

> Within a paragraph, lines are created by separating character sequences at places where
> line breaking is not prohibited. Except for the end of the last line of a paragraph, it is
> necessary to set the head and end of each line at predicable, aligned positions.

and it states, separately, §3.5.3's four answers for what a *short* line does. Whether an
unstated `alignment` means "§3.8.1 and nothing further" or "one of §3.5.3's four answers,
chosen as the default" is the question, and the two are not the same answer for any line that
comes up short.

## The reading

**A request that states no alignment is justified, and that is not the same answer as
`start`.** Every line but a short last one is adjusted to the measure. `start` is one of
§3.5.3's four answers — the one a caller who wants a flush short line asks for — and asking
for it is a different request from not asking.

The difference is observable wherever a **non-last** line comes up short and Table 6 offers
it a site. Of the eighty-nine built-in cases only
`3.5.4/widow-keeps-two-clusters-on-last-line` reaches that shape: the line a widow minimum
shortened is opened back out to the full measure under an unstated alignment, and left short
under `start`.

## Why

**§3.8.1 is what a paragraph does before anyone states a preference.** The section is not one
of four options; it is the description of line adjustment as such, and its "it is necessary"
is JLReq's strong form. A caller who has not expressed a preference has asked for a
paragraph, and a paragraph in JLReq's own terms has its lines set to the measure. Reading the
omission as `start` would make the protocol's default the *one* of §3.5.3's four answers that
suppresses §3.8.1 for every short line, which is a strong editorial choice to hide in an
absent member.

**A default that equals one of the stated values makes the value unaskable.** If omitting
`alignment` meant `start`, then a caller who wrote `"alignment": "start"` and one who wrote
nothing would be indistinguishable to the engine, and the schema's own four-value enumeration
would carry three meanings and a synonym. Keeping the omission distinct is what lets a
conformance case state the `start` answer *as an answer* — which is what §3.5.3 needs, since
its four answers are alternatives the specification names and a suite has to be able to carry
all four.

**Short last lines are not the coordinate this decides.** §3.8.1 exempts the end of the last
line explicitly, so an unstated alignment and `start` give the same last line and differ
only where a non-last line came up short. That is a narrow shape, which is why it took the
`tabs` and `widow` censuses to find: a line the §3.6.3 cut left short, or a line a widow
minimum shortened, is a non-last line below the measure, and nothing else in the eighty-nine
cases produces one.

## What would change it

A `default` on `alignment` in `protocol.schema.json`, or a sentence in
`docs/design/conformance.md` naming what the omission means, settles this reading and moves
it out of `docs/decisions/` entirely — it is a reading of the *format* rather than of JLReq,
and the format is this project's own document. That is the change worth making, and this file
exists to record the answer both engines already agree on until it is made.

Making the member required would settle it in the other direction, at the cost of forcing
every caller to take a position on §3.5.3 in order to compose an ordinary paragraph.
