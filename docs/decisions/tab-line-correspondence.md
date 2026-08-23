<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: what §3.6.3 corresponds a line's tab signs with, and what a sign with no stop does

- Applies to: the tab round in
  [`pipeline`](../../crates/jlreq/src/pipeline.rs) and
  [`paragraph`](../../crates/jlreq/src/paragraph.rs), and the same round in
  [`engines/ocaml/lib/pipeline.ml`](../../engines/ocaml/lib/pipeline.ml)
- Standing: `Unstated`
- JLReq: §3.6.1, §3.6.2, §3.6.3, §3.1 (silence), ADR-0018
- Observed by: `just census tabs` (24,334 requests), and by probing the reference engine
  with stops the eighty-nine built-in cases never state

## The silence

§3.6.3 states the correspondence between the signs of a line and its stops:

> Set the text from the line head to the position before the tab sign in the first tab
> position, set the text from the first tab sign to the next tab sign in the second tab
> position, and so on.

and, four sentences later, the case where the correspondence runs out:

> […] no tab position corresponding to the target string, the string should be set from the
> tab position of the next line.

That sentence says where the *string* goes. It does not say that the line ends before the
sign, rather than the sign taking some default width where it stands — those are two
different mechanisms with the same effect on the string and different effects on everything
else. Nor does it say what happens when there is no earlier boundary to send the string back
from: a sign at the line head has no "next line" to be moved to that is not the line it is
already on.

Five more questions sit around the same sentence and none of them is answered anywhere:

- Whether §3.6.3's cut answers to Table 2, or to §3.1's rules about where a line may end at
  all.
- What a sign does when it stands inside a construct — an object the line holds at one
  position, whatever it holds inside.
- Which stop a sign takes when the request lists its stops out of positional order. §3.6.3
  says "in order" and does not say whose order.
- Whether a stop may sit at the measure, or past it, and whether that is checked at all when
  the source holds no tab sign for it.
- What §3.6.1's count means. Its sentence —

  > […] it is necessary to set the same numbers of tab positions and tab types as the number
  > of tab signs

  counts signs *in a line*, and which line a sign lands on is what composition decides, so
  the count is stated about something that does not exist until after the rule has been
  applied.

## The reading

**A tab sign whose stops the line has gone past ends the line, and that cut answers to no
character class.** The line ends there, at boundaries Table 2 would never let a line end at:
a line whose last character is an opening bracket (cl-01) is the answer when the sign follows
one. The cut is §3.6's and not §3.1's.

**A tab sign standing at the line head with no stop left keeps its line and takes one em.**
It is the one place §3.6.3's fourth sentence has nothing to say, because there is no earlier
boundary to send the sign to. The width taken is one em of the paragraph's own size — a
number §3.6 never mentions.

**A sign standing inside a construct keeps its line too, for the same reason.** A sign inside
an emphasis run, a superscript, a reference mark, a jidori, a formula or a base character
group runs its stops out and takes one em where it stands. A construct that begins or ends
exactly *at* the sign leaves the cut available, because the sign is then beside the construct
rather than in it.

**Stops are taken in the order they stand along the line, not the order the request lists
them.** A request may list them descending; each sign takes the nearest stop ahead of the
cursor either way.

**A stop must lie strictly inside the measure, and that is checked whether or not the source
holds a tab sign.** A stop at the measure exactly is refused.

**§3.6.1's count of stops is enforced between mandatory breaks, and a surplus is allowed.**
The engine refuses a stretch between two mandatory breaks that holds more signs than there
are stops, and accepts one that holds fewer. "The same number" is read as a floor and not as
an equality.

## Why

**The string cannot be set from the next line's stop without the line ending.** §3.6.3's
fourth sentence puts the target string on the next line. Everything before the sign is
already set on this one, so the only composition that produces the stated result is a line
boundary at the sign. The alternative — the sign takes a default width and the string
continues on the same line — does not put the string at the next line's tab position at all,
so it is not a reading of the sentence but a different rule.

**A cut §3.1 could veto would make §3.6.3 unreachable.** Which boundary the cut falls at is
decided by arithmetic — where the stops are, and how wide the text before them turned out —
and that arithmetic has no way to prefer a boundary Table 2 approves of. If §3.1's
prohibitions applied, a paragraph whose stop arithmetic lands the cut after an opening
bracket would have no answer at all: §3.6.3 requires the cut and §3.1 forbids it. Reading the
cut as §3.6's own is what leaves both sections satisfiable. The same argument decides the
construct case from the other end: §3.6.3's cut is a *line boundary*, not a break opportunity
that a rule about characters could permit or forbid, so the only thing that can withhold it
is there being no boundary at that point — and inside one object on the line, there is not.

**One em is the only width §3.6 leaves available.** The line-head sign is the one case the
fourth sentence cannot reach, so the sign has to take *some* width, and §3.6 names none.
Every other unstated width in this engine is read from the paragraph's own size, which is the
one quantity the request always carries, and a full em is what a character with no advance of
its own takes everywhere else in JLReq. This is the weakest of the readings here and the one
most clearly labeled an invention.

**A line knows where its stops are and not how they were typed.** §3.6.3's "in order" is
stated about the correspondence between signs and stops as the line is walked, and the only
order a line has is position. Reading it as list order would make the same paragraph compose
two different ways depending on how a caller happened to serialize a set, which is a
distinction the protocol does not otherwise draw anywhere (ADR-0018).

**A stop at the measure is not a position in the line.** §3.6 says a stop is a position in
the line and says nothing about the ends of one. A stop at the measure exactly names the point
the line ends at, where no text can be set; a stop past it names a point outside. Refusing
both, and refusing them whether or not a sign happens to reach them, keeps the validation a
statement about the request rather than about the composition — a request that is ill-formed
is ill-formed before anything is laid out.

**A mandatory break is the only division into lines validation can see.** §3.6.1's count is
stated about a line, and which line a sign lands on is exactly what §3.6.3 decides — so
enforcing an equality would mean validating the input against a result that does not exist
yet. The caller's own mandatory breaks are the one division into lines that is known before
composition. Reading the count as a floor rather than an equality follows from the same fact
in the other direction: a surplus stop is a stop no sign reached, which is a harmless
statement about a line that came out shorter than the caller expected, while a deficit is a
sign with nothing to correspond to, which is the contradiction §3.6.1 is there to catch.

## What would change it

A revision of §3.6.3 that says the line ends at the sign, or that names a width for a sign
that cannot be moved, settles the first two readings. A statement anywhere that §3.6.3's cut
is or is not subject to §3.1 settles the third, and is the one a browser or a LaTeX
comparison is most likely to produce evidence about. An `order` note in §3.6.3, a sentence in
§3.6.1 about the ends of a line, and a restatement of §3.6.1's count against something other
than a line settle the rest.

Two coordinates in this same subject are **not** readings this project publishes but places
the two reference engines answer differently, and they are recorded as issues rather than
here, because the rule is to return to JLReq and to `spec/` rather than to settle a
disagreement by copying: a tab sign that is the first character of a tate-chu-yoko run, and a
tab sign inside a warichu or a furawake. The `tabs` census deliberately covers neither shape
— a census is a gate, and a gate that is red is not one — and covers a sign beside a construct
and inside an emphasis run, a superscript, a jidori and a tate-chu-yoko run instead.
`engines/ocaml/test/test_pipeline.ml` pins the OCaml engine's answer for both.
