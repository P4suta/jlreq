<!--
SPDX-FileCopyrightText: 2026 kumihan contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: what §3.1.5 pattern 2 and §B.2 note 17 leave open about the line-head half em

- Applies to: `jlreq_spacing::evaluate::boundary` (§3.1.5/§B.2#17's private helper
  synthesizing pattern 2's own half em, `line_head_opening_bracket_space`)
- Standing: `Unstated` for the three silences below; the underlying pattern choice itself is
  `Alternative` — it is `Question::LINE_HEAD_OPENING_BRACKET`, marked `stated` in
  `spec/derived/questions.tsv` — and every one of its three answers is reachable from the
  published format today through an explicit policy overlay, the same standing
  `docs/decisions/sentence-medial-dividing-mark.md` records for its own sibling question.
- JLReq: §3.1.5, §B.2#17, §B.1, ADR-0014

## The silence

§3.1.5's body states three patterns for starting a new line with an opening bracket (cl-01),
Figure 71, quoted from `spec/derived/rules.tsv`'s own row 29:

> When starting a new line with opening brackets (cl-01) there are some patterns as shown in
> Figure 71. Note that the amount of line indent after the line feed (the first line indent of
> a new paragraph) is assumed to be a one em space across all the patterns.

The three patterns, from `spec/snapshot/index.html`'s own body text at this section:

> ① 改行行頭の字下げは全角アキ，折返し行頭は行頭に空き量をとらない配置法である天付きとする
> （The first line indent after the line feed is set full-width (one em) and the next line
> after the first line break starts with no space）
>
> ② 改行行頭の字下げは全角半（全角の1.5倍）アキ，折返し行頭の字下げは二分アキとする
> （The first line indent after the line feed is set one and a half em and the next line
> indent after the first line break is set to a half em）
>
> ③ 改行行頭の字下げは二分アキ，折返し行頭は天付きとする
> （The first line indent after the line feed is set at a half em and the next line after the
> first line break is set tentsuki）

and the Note immediately under Figure 71:

> Because the inherent character width of a bracket is considered to be half-width, Figure 71
> ① can be explained as the result of applying the principle that any line should start with
> no spacing. On the other hand, the principle represented by Figure 71 ② is to assume that
> opening brackets should be always accompanied by the preceding half em spacing as if they
> were full-width and then apply the same principle as in Figure 71 ①. JIS X 4051 adopts the
> principle shown in ① (the patterns shown in ② is offered as options). […]
>
> 元々括弧類の字幅は半角であったのであるから，何も空き量を入れなければFigure 71の①の方法とな
> る．これに対し，②の方法は，行頭の括弧類の字幅について空き量を含めて全角とする処理方法であ
> る．JIS X 4051では，①の方法を採用している（ただし，オプションで②の方法も選択できる）．［…］

Both quotes are truncated at the point marked; `spec/snapshot/index.html`'s own Note continues
for a comparable length past it with pattern ③'s own publishing history — which named
publishers use which pattern, and that Iwanami Shoten once used ② in vertical composition and
few examples of it remain today — a discussion none of the three silences below depend on, so
it is marked rather than left to read as if the quote above were the Note's own entirety.

§B.2 note 17, `spec/derived/notes.tsv`'s own row for `B.2#17`:

> The preferred character spacing between the line head and opening opening brackets (cl-01)
> is zero. An alternative way is not to remove a conditional half em spacing accompanying the
> characters (see § 3.1.5 Positioning of Opening Brackets at Line Head including methods of
> positioning of opening brackets at the beginning of paragraphs).
>
> 行頭に配置する始め括弧類（cl-01）の前はベタ組である．ただし，行頭に配置する始め括弧類
> （cl-01）の前を二分アキとする方式もある（改行の行頭に配置する始め括弧類（cl-01）の配置法を
> 含め，§ 3.1.5 行頭の​始め括弧類の​配置方法 参照）．

Between them, these two passages state: three named patterns, each with two positions (the
paragraph's own first-line indent after a line feed, 改行行頭, and the indent an ordinary
in-paragraph wrap gets, 折返し行頭); which of the three positions is zero and which is a half
em at each position; and, in the appendix note, that the "preferred" zero and the "alternative"
half em are the identical two choices Figure 71 already lays out, cross-referenced by section
title. They do not state:

1. **Whose em the half em is a fraction of**, in Appendix B's own `be`/`af` vocabulary. Both
   passages describe the bracket as the thing the spacing "accompanies," in prose, never in
   the referent vocabulary the generated tables use.
2. **Whether Appendix D's reduction tables govern this half em at all**, and if the note's own
   word "conditional" is evidence either way.
3. **Which `RuleId`** — the appendix note Table 1's own captured cell already cites, or
   §3.1.5's own rule, cited by no generated cell anywhere — a synthesized space should carry,
   given that api-spine.md's own frozen doc for this Question already pairs both addresses on
   one line and neither document says which of the two names an amount and which names a
   citation.

## The reading

**1. The referent is the bracket's own, `Referent::Trailing`.** At the one coordinate this
synthesis can ever fire — `before == raw::LINE_EDGE`, `after == Class::OpeningBracket.number()`
— the bracket is the boundary's *only* possible neighbor: a line head has no preceding item at
all (`Adjacency::before_position` answers `Before::LineHead`, never a class), so
`Referent::Preceding` never has an owner to assign. That sole neighbor sits at this boundary's
`after` position, so ADR-0014's own "which neighbor's frame" convention and the plain textual
reading of "before"/"after" agree without needing an inversion — unlike
`docs/decisions/sentence-medial-dividing-mark.md`'s own point 1, whose mark sits at two
different boundaries in two different roles and needs the inversion argued out. §3.1.5's own
Note independently names the same owner: pattern ② is explained as brackets being "always
accompanied by the preceding half em spacing as if they were full-width," which states the
half em as a property of the bracket's own (assumed) width, not of anything else.

**2. Appendix D does not govern this half em; `Reduction::Rigid`, stated directly.** Checked
against the generated data rather than assumed: `crates/jlreq-spacing/src/generated/table3.rs`,
`table4.rs` and `table5.rs` each carry a row at `(0, 1)`, `limit: None` (which
`evaluate::cell_reduction` maps to `Reduction::Rigid` regardless of which table
`Question::REDUCTION_TABLE` selects), citing `RuleId::LEGEND_OF_TABLES_3_4_AND_5`. That
citation is not evidence of a stated reduction schedule for this coordinate — it is §D.1's own
generic legend statement, the one 833 of Table 3's, and 834 of Table 4's and Table 5's, 841 rows
each carry, because all three reduction tables are total over the entire 29-by-29 grid (every
class pair, plus the line-edge row and column) rather than sparse over the coordinates a D.2
note actually singles out. `evaluate::special_reduction`'s own three governed coordinates —
`(5, 5)`, `(6, 5)`, `(7, 5)` — are the only cells where a D.2 note assigns a term its own
reduction priority; `(0, 1)` is not among them, and Table 1's own cell at `(0, 1)` carries no
term at all (`terms: &[]`) for any reduction table to be describing the reduction *of* in the
first place. Reading this synthesis through the ordinary `ranged_cell` lookup the way the
per-term loop reads Tables 3 through 5 would therefore attach a real, specific-looking `RuleId`
to a fact nobody stated.

Whether the note's own word "conditional" — "a conditional half em spacing" — is itself a
signal that Appendix D reduces this amount does not settle the question either way, and this
file says so rather than picking a reading the words alone cannot support. §D.2 note 5 calls
the katakana middle dot's own quarter em, which Table 3 genuinely does reduce to nothing at its
third priority, "conditional" in the identical construction: "the preceding and trailing
conditional quarter em space accompanying middle dots." ADR-0014's own Context section quotes
"the conditional half em space accompanying the preceding comma" as Appendix B's ordinary,
blanket vocabulary for *any* table term — reducible (the comma's own half em is D.2#3's own
subject) or not. "Conditional" names the category — an Appendix B amount realized only under
stated conditions, as opposed to always — not a promise about Appendix D one way or the other,
so it cannot be the evidence either reading would need. What answers the reducibility question
is the captured reduction tables' own content, read above: a present-but-generic row, not a
stated schedule.

**3. The citation is §3.1.5's own rule, `RuleId::POSITIONING_OF_OPENING_BRACKETS_AT_LINE_HEAD`,
not §B.2#17's.** Checked directly, in both directions:

- `RuleId::B_2_NOTE_17` already has a live reader at this exact coordinate, and this round
  changes nothing about that fact. `crates/jlreq-spacing/src/generated/table1.rs`'s own `(0,
  1)` cell cites it by name (`prohibited: false, rule: B_2_NOTE_17, terms: &[]`), so
  `evaluate::boundary`'s own `placement` answer already carries it through
  `Provenance::of(cell.rule, Standing::Normative)`, and `evaluate::rules_fired`'s own
  `rules[1]` reads it — confirmed by a scratch probe run and discarded before this round's own
  gate battery (task #42's own fact 8) — for *every* answer `Question::LINE_HEAD_OPENING_BRACKET`
  can take, not only pattern 2.
- `RuleId::POSITIONING_OF_OPENING_BRACKETS_AT_LINE_HEAD` has no reader anywhere in this
  workspace before this round. Checked directly: `grep -rn
  POSITIONING_OF_OPENING_BRACKETS_AT_LINE_HEAD crates/ spec/ docs/` finds exactly two
  occurrences, the constant's own declaration
  (`crates/jlreq-spec/src/generated/inventory.rs`) and its own row
  (`spec/derived/rules.tsv`) — cited by no generated table cell and called from no evaluator
  function anywhere.

Citing `B_2_NOTE_17` on the synthesized space would give one of this coordinate's two paired
addresses a second reader while leaving the other permanently silent. Citing §3.1.5's own rule
instead gives both a reader at the one coordinate `docs/design/api-spine.md`'s own frozen doc
for `Question::LINE_HEAD_OPENING_BRACKET` already pairs them at — "Spacing before cl-01 at the
line head. JLReq: §B.2#17, §3.1.5" — completing the pairing the spine already promised rather
than adding a second voice to one half of it.

## Why

**The referent (1) needs no inversion because this coordinate has only one possible owner.**
ADR-0014's own convention exists to say *which* neighbor's em an amount is a fraction of when
more than one reading is available; here only one neighbor exists at all, so the convention and
the naive reading coincide, and §3.1.5's own Note independently agrees.

**The reduction reading (2) rejects a plausible alternative on record rather than by
assumption.** A reader could instead take Appendix D's captured `(0, 1)` row at face value —
"a row exists, so it governs" — the same shape `docs/decisions/sentence-medial-dividing-mark.md`'s
own point 6 leans on for its own, textually different coordinates. This file's own reading
rejects that shape here specifically because the row was checked and found to be the tables'
own total-grid boilerplate rather than a genuine D.2-style schedule: `ranged_cell(table3::CELLS,
19, 4)` — a coordinate `docs/decisions/sentence-medial-dividing-mark.md` itself treats as one
Appendix D "states nothing" for — answers `Some`, `limit: None`, the identical
`LEGEND_OF_TABLES_3_4_AND_5` citation, which is the direct evidence that a present row at this
coordinate is not by itself evidence of anything Appendix D specifically decided. See "What
would change it" for what would flip this reading.

**The citation reading (3) follows from a fact checked, not inferred: one of the two names has
zero readers and the other already has one.** A version of this decision that cited `B_2_NOTE_17`
throughout — matching the existing Table 1 cell exactly, the same principle the per-term loop
follows for every ordinary captured term — would be defensible in isolation, but it would leave
`RuleId::POSITIONING_OF_OPENING_BRACKETS_AT_LINE_HEAD` exactly as unread after this round as
before it, which is the fact task #42 exists to fix and not merely `Question::LINE_HEAD_OPENING_
BRACKET` being read by some evaluator function — the rule *citation*, not only the policy
*question*, is the thing `docs/conformance-deferrals.toml`'s own `3.1.5` entry names as absent.
This reading's own shape matches `sentence_medial_dividing_mark_spaces`'s point 6 (a synthesized
space citing its own governing §3.x rule rather than Table 1's), but not that function's own
stated ground: that function's coordinates carry no specific citation to compete with (the
generic `RuleId::SPACING_BETWEEN_CHARACTERS`), where `(0, 1)` already carries a real, specific
one. The two functions reach the same *shape* of citation choice by two different arguments, and
this file states its own rather than borrowing the sibling's.

## What would change it

Evidence that JLReq's own worked example for Figure 71 (not machine-readable from
`spec/snapshot/index.html`'s captured text and not transcribed into `spec/captured/`) shows a
publisher reducing pattern 2's own half em under some condition Appendix D's tables would
otherwise be silent about — a specific sentence assigning `(0, 1)` its own D.2-style priority,
the shape `special_reduction`'s own three governed coordinates already have — would move point 2
from "the generic legend row settles nothing" to "a real schedule exists," and this reading
would need to route the synthesis through `ranged_cell` at that point rather than stating
`Reduction::Rigid` directly.

A future round wiring the 改行行頭 (paragraph first-line indent) half of Figure 71 into
`jlreq_line::Paragraph` — the gap `jlreq-line`'s own "Slots" section names — would give pattern
3 a complete answer for the first time and might reopen point 3's own citation choice if that
round finds a cleaner single citation covering both halves of one pattern; until then, the two
halves are answered by two different crates and this file's own citation argument concerns only
the half `jlreq-spacing` answers.

Evidence that a future capture revision moves `RuleId::POSITIONING_OF_OPENING_BRACKETS_AT_LINE_HEAD`
onto some generated table cell directly — giving §3.1.5's own rule a reader that does not run
through this synthesis — would not by itself change this reading's own citation choice, because
the argument for it rests on which of the two paired addresses had *zero* readers at the time
this round ran, not on which one is structurally incapable of ever getting one another way.
