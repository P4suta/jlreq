<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: what an engine does with a break the caller states inside an indivisible construct

- Applies to: the feasibility round in
  [`pipeline`](../../crates/jlreq-core/src/pipeline.rs), and the same round in
  [`engines/ocaml/lib/pipeline.ml`](../../engines/ocaml/lib/pipeline.ml)
- Standing: `Unstated`
- JLReq: §C.2#13, §C.2#6, §C.2#8, §3.3.5, §3.3.6, §3.3.9, §3.7.1, §3.7.3, §3.7.4, §3.4.2,
  §3.7.2
- Observed by: `just census tate-chu-yoko`, `just census ruby`, `just census constructs`

## The silence

JLReq says in several places that a construct is indivisible. §C.2 note 13, for a
tate-chu-yoko run:

> There is no line break opportunity between two consecutive characters belonging to the
> same […]

and the same form of words for an ornamented character complex (§C.2 note 6), and §C.2
note 8 for the ruby text accompanying one base character:

> […] accompanying ruby text shall be indivisible, hence there is no line break opportunity
> between any two consecutive ruby characters in a run of ruby text accompanying a base
> character.

Every one of those sentences describes an *opportunity that does not exist*. None of them
says what an engine does when a caller states a break there anyway. Two answers are
coherent and JLReq chooses neither: never take the opportunity — treat the caller's
declaration as inapplicable and compose the paragraph as if it had not been made — or
refuse the request, on the ground that a caller who states a break inside an indivisible
run has described a paragraph that does not exist.

§3.7.4 is the same shape with the opposite polarity. It states a **permission**:

> A line can be broken between math symbols (cl-17) or math operators (cl-18) and adjacent
> grouped numerals (cl-24), Western characters (cl-27) or ornamented character complex
> (cl-21).

and a Note giving a priority between two such positions when more than one exists. Neither
says that a boundary inside a formula that is *not* beside a math symbol or a math operator
is forbidden. A permission to break in one place is not on its face a prohibition on
breaking anywhere else, and §3.7.4 nowhere adds the sentence that would make it one.

## The reading

**A break stated inside an indivisible construct is refused, not declined.** The request
comes back with `input.break-inside-construct` rather than with a layout, and that holds for
an `allowed` break and a `mandatory` one alike — and in horizontal composition too, where a
tate-chu-yoko construct changes nothing else at all.

The coordinate differs by construct, and the differences are the substance of the reading:

| Construct | A break strictly inside it | A break at an internal boundary |
| --- | --- | --- |
| tate-chu-yoko run (§3.2.5, §C.2#13) | refused | — the run has no internal structure |
| ruby base character group (§3.3.5, §3.3.6) | refused inside one **run** | **answered** at a run boundary, which §C.2 note 8 grants a jukugo compound outright |
| ornamented complex — `script`, `reference-mark` (§3.7.1, §C.2#6) | refused | — one complex outright |
| emphasis run (§3.3.9) | **answered** | §3.3.9 makes each base character a complex of its own |
| jidori (§3.7.3) | refused | — |
| formula (§3.7.4) | refused unless a math symbol or math operator is on one side | answered beside cl-17 or cl-18 |
| warichu (§3.4.2) | **answered** | the structure divides |
| furawake (§3.7.2) | **answered** | the structure divides |

**§3.7.4's two named classes are the whole of where a formula may break.** A break with a
math symbol (cl-17) or a math operator (cl-18) on either side of it is answered; every other
break inside a formula is refused, for a display formula and an inline one alike.

## Why

**Refusing is the answer that does not invent a paragraph.** A caller who states
`{"offset": n, "kind": "mandatory"}` inside a tate-chu-yoko run has asserted two things that
cannot both hold: that the run is a run, and that the line ends in the middle of it.
Composing anyway means silently discarding one of the caller's own declarations and
returning a layout for a paragraph the caller did not describe — and for a `mandatory`
break, silently discarding the stronger of the two. `docs/design/conformance.md`'s own
posture is that an engine reports what it cannot do rather than approximating it; the
refusal is that posture applied to an input contradiction, which is why it is `input.` and
not a diagnostic on a returned layout.

**A run and a construct are not the same coordinate, and the difference is what §C.2 note 8
turns on.** Note 8's indivisibility is stated of the ruby text accompanying *one base
character*. A jukugo compound is several such runs, and a break between two of them is a
break between two base characters, which the note does not reach and §3.3.7 positively
contemplates. Refusing at the construct rather than at the run would make a jukugo compound
unbreakable, which no sentence says. Refusing at the run and answering at the boundary is
the reading that keeps note 8's own subject.

**§3.3.9 and §3.7.1 differ because their complexes differ.** §B.2 note 9, §C.2 note 6 and
§E.2 note 5 are all stated about "two consecutive characters belonging to the same
ornamented character complex (cl-21)", and JLReq never says how many complexes an emphasis
run is. This project answers one per character
([ornamented-complex-geometry](ornamented-complex-geometry.md)), and the break behavior
follows from that answer rather than being a second decision: two emphasized characters are
two complexes, so the boundary between them is a boundary between complexes and note 6 does
not reach it.

**Reading §3.7.4's permission as exhaustive is what its Note presupposes.** The Note ranks
two break positions — before a math symbol first, before a math operator next — for the case
where more than one exists. A ranking among the permitted positions is only a ranking if the
permitted positions are the only ones; if any boundary inside a formula could take the
break, the Note would be ranking two members of an unbounded set and saying nothing about
the rest. The permission read as exhaustive is what gives the Note work to do.

## What would change it

A sentence in §C.2 or in the protocol's own contract saying that a break stated where none
exists is ignored rather than refused would settle the first reading, and would make the
refusal a defect in both engines at once — this is one of the readings most likely to be
overturned by a format decision rather than by a specification revision, because it is as
much about what a request means as about what JLReq says.

For §3.7.4, a revision that says "and nowhere else", or one that says the two named classes
are a preference rather than a rule, settles that reading in either direction. Evidence that
a publisher breaks a display formula at a boundary neither class touches — a long formula
broken after a closing bracket — would be recorded as a `disagreements` entry on a
conformance case rather than as a change here, once the case exists to carry both readings.

An engine that declined rather than refused would be visible immediately: the censuses that
reach these coordinates give every construct exactly the break shapes that are answerable,
because a refused request ends a census rather than measuring anything, and a `refused`
answer and a *composed* one are not two spellings of the same result.
