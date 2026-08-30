<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: which coordinates §3.8.4's ladder asks its ceiling at, and which its fourth step re-levels

- Applies to: the expansion ladder in
  [`pipeline`](../../crates/jlreq-core/src/pipeline.rs), and the same ladder in
  [`engines/ocaml/lib/pipeline.ml`](../../engines/ocaml/lib/pipeline.ml)
- Standing: `Unstated`
- JLReq: §3.8.4, §E.1, §E.2, ADR-0021
- Observed by: `just census expansion` (3,174 requests), both silences below

## The silence

§3.8.4 states the expansion ladder as four steps, and two of them leave a question the
section does not return to.

**Step (b) names classes, not coordinates.** The step names three Japanese classes —
hiragana (cl-15), katakana (cl-16), ideographic characters (cl-19) — and three Latin ones
— grouped numerals (cl-24), unit symbols (cl-25), Western characters (cl-27). Three
against three is nine ordered coordinates in each direction, eighteen in all. §3.8.4's own
Note, which is the sentence the `rigid` answer of
`adjustment.japanese_latin_expansion_ceiling` rests on, names `漢字等（cl-19）など` in
Japanese and expands that to all three Japanese classes in English. Neither the step nor
the Note says whether the ceiling the style answers is asked at all eighteen coordinates or
at some subset of them, and `spec/derived/questions.tsv` — the file both engines read for
exactly this kind of qualification — carries no scope column at all. A question with no
scope column is a question stated as if it applied everywhere.

**Step (d) names four steps and one purpose.** §E.1's own sentence for the fourth step is
that it adds space

> to equalize the spacing of 1st, 2nd, 3rd and 4th steps.

Read at face value that is all four stages, step (a)'s Western word spaces included. The
sentence does not say whether a site that only step (a) could reach — a word space beside
cl-26 that Table 6's own cl-26 row does not independently make residual — is one of the
sites the fourth step re-levels, or whether "equalize" means only that the amounts the
earlier steps already placed are brought to a common level.

## The reading

**The Japanese–Latin ceiling is asked at `(cl-19, cl-27)` and `(cl-27, cl-19)` and nowhere
else.** `adjustment.japanese_latin_expansion_ceiling` — `half-em`, `third-em` or `rigid` —
is consulted at those two coordinates. At the other sixteen stage-two coordinates Table 6's
own half em stands, whatever the style answers. The style question is therefore not a
global cap on Japanese-against-Latin expansion; it is a cap on one pair of rows and
columns.

**Step (d) re-levels the second and third stages' boundaries and the residual cells, and a
Western word space only where Table 6's own cl-26 row makes that boundary residual too.** A
first-stage site that is not independently residual is excluded from the fourth step's
re-leveling. Every other site the earlier steps opened is included, with no ceiling
applied at this stage.

## Why

**A ceiling that applied at eighteen coordinates would be measured at coordinates Table 6
gives no stage-two amount to.** §3.8.4's stages are read off Table 6, and the eighteen
coordinates step (b) names are not eighteen identical cells: the matrix states its own
amount and its own stage for each, and the ceiling is a cap on an amount the matrix already
placed. Asking a `third-em` cap at a coordinate whose cell is not the Japanese–Latin half
em is asking it of a number that came from somewhere else. The two coordinates the ceiling
is asked at are the two the Note's own example — an ideographic character against a Western
character — actually names in both renderings; the other four Japanese-side and Latin-side
classes appear only in the step's own enumeration, which is a statement of *which
boundaries expand*, not of which of them the style question governs.

**The census is what makes the difference observable at all.** Every `expansion` line the
census emits carries three interior boundaries, because a line with one boundary cannot
tell a stage or a ceiling apart from "hand the whole shortfall to the only site that will
take it" — which is the shape all six of the built-in suite's expansion cases have. The
scope of the ceiling is invisible in the eighty-nine cases and visible in 3,174 synthetic
ones. So is step (d)'s exclusion: it needs a line carrying both a word space and a
stage-two or stage-three boundary, with a shortfall large enough that the fourth step runs
at all.

**A word space is already at its own maximum when step (a) finishes.** §3.8.4's first step
opens a Western word space up to a half em, which is the whole of what that site is
permitted. Handing it more in the fourth step under the heading of "equalize" would take
it past the amount step (a) itself capped it at, and the fourth step states no ceiling to
re-impose one. Excluding it keeps the cap the first step stated; including it would make
the first step's cap advisory, which §3.8.4 nowhere says it is. Where Table 6's own cl-26
row makes the boundary residual, the site is in the fourth step for that reason and not as
a word space.

## What would change it

A revision of §3.8.4 that states the ceiling's scope — either "at every coordinate named
in step (b)" or a narrower list — settles the first reading directly, and a `scope` column
in `spec/derived/questions.tsv` is the mechanical form that would carry it to both engines
at once. A revision of §E.1's fourth-step sentence that names the sites rather than the
steps settles the second.

Two neighboring questions were checked against the same census and found **not** to be
policies of this kind, and are recorded here so that a later reader does not re-derive
them:

- §E.1 states that the `1/4–1/2` cells "shall not be expanded" when Table 5 is adopted as
  the reduction method. Neither engine implements that exclusion, and
  `spec/derived/questions.tsv`'s own `excludes` column carries no such pair. Adding the
  pair to that column is the concrete change that would make the exclusion real for both
  engines at once, and it is a defect in the derivation rather than a reading.
- `adjustment.expansion_order` selects nothing today. Its `implementation` answer rests on
  §3.8.4 step (d)'s Note, whose only coordinate is cl-27 against cl-27, which
  `docs/conformance-deferrals.toml` classifies `[[non-observable]]`. Both engines answer
  the same layout for both answers, so there is no reading to publish until a coordinate
  exists that tells them apart.
