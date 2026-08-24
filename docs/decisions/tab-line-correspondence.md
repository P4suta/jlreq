<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: what §3.6.3 corresponds a line's tab signs with, and what a sign with no stop does

- Applies to: the tab round in
  [`pipeline`](../../crates/jlreq/src/pipeline.rs) and
  [`paragraph`](../../crates/jlreq/src/paragraph.rs), and the same round in
  [`engines/ocaml/lib/pipeline.ml`](../../engines/ocaml/lib/pipeline.ml) and
  [`engines/racket/compose.rkt`](../../engines/racket/compose.rkt)
- Standing: `Unstated`
- JLReq: §3.6.1, §3.6.2, §3.6.3, §3.1 (silence), ADR-0018
- Observed by: `just census tabs` (30,153 requests), and by probing the reference engine
  with stops the eighty-nine built-in cases never state; the reading about which constructs
  hold a coordinate a stop can name by the third engine's convergence on the same census.
  The two coordinates this file once excluded — a sign that opens a tate-chu-yoko run
  ([#12](https://github.com/P4suta/jlreq/issues/12)) and a sign inside a warichu or a
  furawake ([#13](https://github.com/P4suta/jlreq/issues/13)) — are settled below and
  implemented by all three engines, the third of them from
  [#19](https://github.com/P4suta/jlreq/issues/19); the census covers both

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
  position, whatever it holds inside — and, before that, whether a coordinate inside one is
  a position a stop can name at all. §3.6 states a stop as a position in the line, and a
  construct that sets its text somewhere a line does not has no position in the line to
  offer except its own.
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
group runs its stops out and takes one em where it stands. A construct that *ends* exactly
at the sign leaves the cut available, because the sign is then beside the construct rather
than in it.

**Whether such a sign takes a stop at all depends on how the construct sets its text.** A
jidori sets its characters along the line, one position each, so a coordinate inside one is
a position a stop can name, and a sign standing there takes the next stop ahead of it like
any other character of the line. A tate-chu-yoko run is the other shape: it runs *across*
the line and holds one position however many characters it holds, so a coordinate inside one
is not a position a stop could name, and a sign there takes the advance it was shaped with.
§3.6.3's cut is unavailable inside either, because neither is a place a line boundary can
fall.

**A warichu's and a furawake's sublines are the same shape as a tate-chu-yoko run, and are
read the same way.** They run *beside* the line rather than across it, and either way the
structure holds one position on the line however many characters it holds. A sign inside one
takes no stop and sets the advance it was shaped with, and the stop of the *next* sign is
measured from the line's own walk, where the whole structure is one step.

**A structure that stacks its text off the line holds a sign that stands at its first
character.** A run's or a subline's first character is set in the structure like every other
one, so the sign there is in the structure and not beside it: it takes no stop, and §3.6.3's
cut is never chosen at it. This is the one place where a construct that *begins* at the sign
differs from one that ends at it — a construct whose text is set along the line, an emphasis
run or a jidori, leaves the sign a sign of the line at both of its ends.

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

**A stop is a position in the line, so a construct with no positions in the line has no
coordinate a stop can name.** §3.6.3 corresponds the *signs of a line* with the *stops of
that line*, and both halves of that sentence are about the line's own inline axis. A
tate-chu-yoko run runs across that axis and occupies one position however many characters it
holds, so measuring a stop to a coordinate inside one would measure to a point the line does
not have — and would make the width the line is *measured* at and the width it is *set* at
two different numbers. A jidori is the opposite case and is why the rule has to be stated
about the geometry rather than about a list of constructs: it is a construct in every other
respect, but its characters stand one after another along the line at positions of their
own, and every one of them is a coordinate a stop can name. Reading the rule off the
construct list rather than off the geometry would give a jidori a sign that ignores its
stops for no reason a reader could see.

**The same argument reaches the sublines and the first character, and the width the line is
measured at is what settles both.** A warichu's and a furawake's sublines run beside the line
and have the geometry a tate-chu-yoko run has, so nothing distinguishes them from it here. A
sign that is the first character of such a structure is either in it or beside it and cannot
be both, and the two answers are not symmetric: reading it as being *in* the structure gives
one composition, while reading it as a sign of the line requires the engine to end the line
before it and then set it inside the structure on the next line — two answers to "is this a
sign of the line" in one composition. What decides between them is the same fact the
paragraph above turns on: a stop measured to a coordinate the line does not have makes the
width a line is *measured* at and the width it is *set* at two different numbers, and a line
measured wider than it is set is a line reduced when it did not need to be. Reading the sign
as a member of the structure is the only one of the two under which those are one number.

Both coordinates were filed as [#12](https://github.com/P4suta/jlreq/issues/12) and
[#13](https://github.com/P4suta/jlreq/issues/13) rather than settled by copying an engine's
answer, and both are settled here from §3.6.3 and from the geometry §3.2.5, §3.4.2 and
§3.7.2 give those structures. The reference engines agreeing with the result is not the
argument for it.

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
than a line settle the rest. A sentence saying whether a stop may name a coordinate inside a
construct — and, if so, which constructs have such coordinates — settles the one about
geometry, and would be the same sentence that settles the two coordinates the paragraph below
names. A sentence saying that a structure's first character is a character of the line would
settle the last reading against it.

**What the census covers.** The `tabs` census covers a sign beside a construct and inside an
emphasis run, a superscript, a jidori and a tate-chu-yoko run; a sign that *opens* a
tate-chu-yoko run; a sign inside a warichu and inside a furawake; a sign that opens either of
those; and a sign of the line standing *after* such a structure, whose own stop has to be
measured from a walk in which the whole block is one step. Every one of them at both a stop
the line has gone past and one it has not, and all three reference engines answer all of them
alike at every class pair (30,153 requests, no difference). The last four shapes went in when
the Racket engine reached the reading published here
([#19](https://github.com/P4suta/jlreq/issues/19)); until then they were deliberately left
out, because a census is a gate and a gate that is red is not one.
`engines/ocaml/test/test_pipeline.ml`,
[`crates/jlreq/tests/public_api.rs`](../../crates/jlreq/tests/public_api.rs) and
`engines/racket/tests/test-compose.rkt` pin the individual shapes as well.
