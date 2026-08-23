<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: what §3.1.6's third Note leaves open about a sentence-medial dividing mark

- Applies to: `jlreq_spacing::evaluate::boundary` (§3.1.6's private helper synthesizing the
  Note's own quarter em)
- Standing: `Unstated` for the six silences below; the underlying solid/quarter-em choice
  itself is `Alternative` — it is `Question::SENTENCE_MEDIAL_DIVIDING_MARK`, marked
  `stated` in `spec/derived/questions.tsv` — and unlike this file's sibling readings, that
  choice already has two publishable readings once a case exists to carry them: the `policy`
  overlay `A.4.json`'s own `A.4/question-exclamation-mark/sentence-medial-role` case sets
  (`{"spacing.sentence_medial_dividing_mark": "quarter-em"}`) is not a preset selector, it is
  an arbitrary overlay any case may set, so both of this question's answers are reachable
  from the published format today even though every preset answers `solid`.
- JLReq: §3.1.6, §B.1, ADR-0014

## The silence

§3.1.6's third Note, quoted from `spec/snapshot/index.html`'s own anchor
`positioning-of-dividing-punctuation-marks-question-mark-and-exclamation-mark-and-hyphens`
(the published text's own unbalanced parenthesis, "mark see Figure 74)", carried verbatim):

> There are some cases where dividing punctuation marks (cl-04) are used in the middle of a
> sentence, not at the end. In those cases, either add no spacing or a quarter em spacing
> before and after the dividing punctuation mark see Figure 74).

That sentence states a class (cl-04), a role distinction (medial versus the main body's
sentence-final case), and two named answers. It does not state:

1. **Whose em the quarter em is a fraction of.** "Before and after the dividing punctuation
   mark" names two positions, not two owners, and Appendix B's own referent vocabulary
   (`be`/`af`) is a statement about owners, not positions.
2. **Whether the Note's own withdrawal reaches a coordinate Table 1 already answers.** Ten
   Table 1 cells already carry a cl-04 term (listed in "The reading" below); the Note's
   words "add no spacing" do not say whether they mean "at every coordinate this mark
   touches" or "at the coordinate this Note is otherwise silent about."
3. **If the Note's withdrawal does reach an already-answered coordinate, whether the
   quarter-em answer replaces that term, adds to it, or takes whichever is larger.**
4. **Whether the main body's sentence-final exception for a following closing bracket
   (cl-02) carries over to the medial case.** The main body states it for one case only; the
   Note is silent for the other.
5. **What happens at a line head or a line end.** The Note's own two positions presuppose a
   character on each side; it says nothing about a mark that has no such neighbor because a
   line boundary sits there instead.
6. **What reduction stage and what citation a synthesized space carries**, since Table 1
   states none to read at the coordinates this Note actually governs.

## The reading

**1. The referent is the mark's own, on both sides — `Referent::Trailing` at the boundary
before the mark, `Referent::Preceding` at the boundary after it.** ADR-0014's own words are
the reason: "a space between two characters has exactly two owners, which is also why
Appendix B's referent vocabulary is exactly `be` and `af`" — the referent names *which
neighbor's frame* the fraction is measured against, not a textual "before" or "after". The
Note's own subject is the mark's positioning, not either neighbor's, so the owner is the mark
at both boundaries around it: at the boundary immediately before the mark, the mark is the
*trailing* item of that boundary, so its own contribution is `Referent::Trailing`; at the
boundary immediately after the mark, the mark is the *preceding* item, so its own
contribution is `Referent::Preceding`. This is the inverse of what the English words "before"
and "after" suggest read naively, and it is exactly the convention `spaces_of`'s own existing
`own_role` mapping already uses for every other note in this module (`term.trailing`
selecting `after_role`, its absence selecting `before_role`) — a caller who wanted "the
neighbor's own contribution" at these two boundaries already has that vocabulary available
and unclaimed by this reading, because `Referent::Preceding` at the boundary before the mark
and `Referent::Trailing` at the boundary after it name the neighbor, not the mark, and the
Note is not making a statement about the neighbor.

**2 and 3 together. The override reaches a boundary only where Table 1's own cell carries
zero terms (`cell.terms.is_empty()`), and where it does not reach, replace/add/take-the-max
does not arise.** The ten coordinates where Table 1 already states a cl-04 term are:

| coordinate | owner (by `trailing` flag) | term |
| --- | --- | --- |
| (4,1) cl-04 → opening bracket | the bracket (`af`) | half em |
| (4,5) cl-04 → middle dot | the middle dot (`af`) | quarter em |
| (4,21) cl-04 → ornamented char | **the mark itself** (`be`) | quarter em |
| (4,24) cl-04 → grouped numeral | **the mark itself** (`be`) | quarter em |
| (4,25) cl-04 → unit symbol | **the mark itself** (`be`) | quarter em |
| (4,27) cl-04 → Western char | **the mark itself** (`be`) | quarter em |
| (2,4) closing bracket → cl-04 | the bracket (`be`) | half em |
| (5,4) middle dot → cl-04 | the middle dot (`be`) | quarter em |
| (6,4) full stop → cl-04 | the full stop (`be`) | half em |
| (7,4) comma → cl-04 | the comma (`be`) | half em |

Six of the ten are a *neighbor's* own stated requirement (a bracket's own padding, a middle
dot's or full stop's or comma's own half-or-quarter em) — a different fact than the one this
Note states, answered by Table 1's base legend or by another note entirely, and true whether
the adjoining cl-04 mark is sentence-final or sentence-medial. Nothing in §3.1.6's Note
speaks to what a bracket or a middle dot needs; it speaks to what the *mark* needs.

The other four — (4,21), (4,24), (4,25), (4,27) — are `Referent::Preceding` at a
`before == 4` coordinate, which is the mark's own contribution by exactly this reading's own
rule 1 above: Table 1 already gives the mark a quarter em in the mark's own referent at
every coordinate where Table 1 states anything for the mark's own voice at all. That is not a
coincidence this reading has to explain away; it is the strongest textual evidence *for* the
`cell.terms.is_empty()` scope, not against it. Table 1's own legend, not this Note, is
already the mark's own answer wherever Table 1 speaks in the mark's own referent — the Note
is filling the *silence* Table 1 leaves (ordinary running text: ideographic space, hiragana,
katakana, ideograph, and the empty-cell coordinates named in point 4 below), not amending an
answer Table 1 already gives in the identical voice.

Consequently the override in `jlreq_spacing::evaluate` fires exactly where
`cell.terms.is_empty()` holds for that coordinate, checked directly rather than by
enumerating the empty coordinates by hand, so a future capture update cannot silently drift
out of step with this reading. Because it never reaches a coordinate with an existing term,
"replace, add, or take the maximum" (question 3) never has an occasion to arise under this
reading — there is nothing to replace, add to, or compare against.

**4. No closing-bracket exception is carried over to the medial case; the override applies
uniformly, `(4, 2)` included.** `(4, 2)` — a sentence-medial mark immediately followed by a
closing bracket — is itself one of the empty-term coordinates (`spec/captured/table1.en.tsv`'s
own cl-04-against-cl-02 cell is `blank`), so by rule 2's scope it is reachable, and this
reading declines to invent a bracket exception for it. The main body's own cl-02 exception
("add one em spacing after them... except they are followed by a closing bracket") answers a
different, unimplemented mechanism: the first Note's own reading is that the "one em" after a
sentence-*final* mark is realized by inserting a literal cl-14 ideographic-space character
into the stream, not by an inter-character spacing rule at all — `jlreq-spacing`'s own module
doc states plainly that this crate answers Table 1's coordinates, and the character-insertion
question is a caller's / composition-level decision this crate does not make. The main body's
own exception is therefore about whether to display that inserted character before a closing
bracket, a question with no operative meaning for the medial case, which inserts no character
in the first place. Importing a bracket exception into a case that has nothing analogous to
except would be inventing a rule the Note's own text does not state; "silence is not
permission to reuse the main body's rule" is this reading's own restatement of that refusal.

**5. A line-edge coordinate is declined explicitly.** `(4, 0)` (a sentence-medial mark ending
a line) is a real, non-prohibited, empty-term coordinate — unlike `(0, 4)` (a sentence-medial
mark starting a line), which Table 1 already prohibits outright (§3.1.7's own line-head
restriction on cl-04), so that coordinate never reaches this override at all regardless of
this reading. `(4, 0)` has no such structural exclusion, so the override checks for
`raw::LINE_EDGE` on the far side explicitly and answers nothing there, the same way
`line_end_punctuation_override` declines a coordinate outside its own governance rather than
falling through to an answer nobody derived. The Note's own two words, "before and after",
presuppose an actual neighboring character on both sides; a line boundary is not one, and the
Note does not say what a caller should do when one side of the mark is the edge of the
composed measure rather than another character.

**6. The synthesized space carries `Reduction::Rigid` and cites
`RuleId::POSITIONING_OF_DIVIDING_PUNCTUATION_MARKS_QUESTION_MARK_AND_EXCLAMATION_MARK_AND_HYPHENS`,**
not `RuleId::SPACING_BETWEEN_CHARACTERS`. `Reduction::Rigid` is the same total-absence value
`spaces_of`'s own reduction lookup already falls back to when a coordinate has no captured
reduction-table cell to read a stage from (`None => (Reduction::Rigid, cell.rule)`), which is
the honest answer here too: Appendix D states no reduction schedule for a coordinate Table 1
itself states nothing for, so there is no stage for a synthesized space to reduce at. Citing
this rule rather than the generic Table 1 legend citation is deliberate and load-bearing: a
later case, or a later reader of `rules_fired`, can distinguish "this space came from Table 1's
own cell" from "this space came from §3.1.6's own Note" by its citation alone, which is the
entire reason `docs/conformance-deferrals.toml`'s own rewritten §3.1.6 entry can say precisely
which coordinates are now observable and under which policy answer.

## Why

**The referent convention (1) already exists for exactly this purpose.** ADR-0014 states the
referent vocabulary is closed at two because a space has exactly two owners; reading "before
and after the dividing punctuation mark" as "the mark's own em on both sides" is the only
reading that keeps that vocabulary meaning what it means everywhere else in this crate,
rather than inventing a third notion — "textual position" — the vocabulary was never built to
express.

**The `cell.terms.is_empty()` scope (2, 3) is not the only coherent reading, and this file
says so rather than hiding the alternative.** A reader could instead scope the override to
"no term in the mark's own referent" — a referent-level silence rather than a cell-level one
— under which the override would additionally reach `(4, 1)` and `(4, 5)`, the two
coordinates whose existing term is owned by the *neighbor*, and would need to answer question
3's replace/add/max choice there. This file's own reading rejects that alternative because
ADR-0014 makes the *cell* — not the referent — the transcription's own unit answered by one
citation and one prohibition flag together, and a Table 1 cell that states any amount at a
coordinate, in any referent, is not a coordinate the Note's "add no spacing" can be read as
silent about: the cell already has an answer, stated by a different sentence of the
specification, for what happens at that adjacency. See "What would change it" for what
would flip this choice.

**The no-bracket-exception reading (4) follows from what the sentence-final "one em" actually
is.** Because this crate does not implement the character-insertion mechanism the main body's
own exception qualifies, there is no operative rule at `(4, 2)` for the medial case to inherit
in the first place — only a textual coincidence of class numbers between an implemented
question and an unimplemented one.

**The line-edge decline (5) matches this crate's own established idiom for a coordinate a
note does not reach**, `line_end_punctuation_override`'s own model, rather than treating an
absent statement as silent permission to apply the general rule anyway.

## What would change it

A revision of §3.1.6's third Note that names an owner directly — "a quarter em of the
preceding character's own em" the way §B.2 note 5 does for the full-stop-and-middle-dot sum —
would settle question 1 without inference from ADR-0014's general convention.

Evidence that JLReq's own worked example (Figure 74, not machine-readable from
`spec/snapshot/index.html`'s captured text and not transcribed into `spec/captured/`) shows
the quarter em applied *in addition to* an existing bracket or middle-dot term — rather than
Table 1's own six neighbor-owned coordinates being left untouched — would be the direct
evidence this reading's own "Why" section says it lacks, and would move `(4, 1)` and `(4, 5)`
from declined to answered, forcing question 3's own replace/add/max choice for both.

A future round wiring `jlreq-spacing`'s own reading of the first Note (the sentence-final "one
em" as an inserted cl-14 character) would give the `(4, 2)` bracket question in the
sentence-*final* case an actual mechanism for the first time; if that round's own reading
concludes the medial case's Table 1 silence at `(4, 2)` is the same silence the sentence-final
case's own exception fills, this file's own reading in point 4 would need to be revisited
against that mechanism directly, rather than declined as it is today for lack of one.

Evidence that publishers place a sentence-medial mark at a line edge and treat it
differently from an ordinary line-internal placement — carrying the quarter em across the
line boundary in some visible way, for instance — would be recorded as a `disagreements`
entry on a conformance case for this Note, once this file's own declined reading for
question 5 has a second, policy-selectable reading to publish as the alternative.
