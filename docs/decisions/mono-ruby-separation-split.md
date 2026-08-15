<!--
SPDX-FileCopyrightText: 2026 kumihan contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: how a mono-ruby run's overhang surplus is split between its two boundaries

- Applies to: `jlreq_inline::lower` (`Contribution::separations` for `RubyStyle::MonoRuby`),
  and, since task #78, `jlreq_inline::place` (its own centering difference for
  §3.3.5(b)/(c)'s nakatsuki case) — the same `distribute(_, &[one(), one()], _)` split
  answering two different questions about the same run, not two readings. `place`'s own
  module doc cites this file rather than restating question 1's own argument.
- Standing: `Unstated`
- JLReq: §3.3.5, §3.3.8 rule 1, §3.3.1

## The silence

§3.3.8 rule 1 states:

> Ruby text shall not hang over the ideographic characters (cl-19) adjacent to the base
> characters.

§3.3.1's note, describing Figure 107's 凝視 (mono-ruby, three ruby characters over 凝, one
over 視):

> There is quarter em spacing between the base characters "凝" and "視". So when this line
> happens to appear in the middle of a paragraph, there needs to be some line adjustment
> processing.

Together the two state a fact and not an amount: when a run's reading is wider than its own
base item's supplied advance and the neighbor across a boundary is cl-19, the base characters
are pushed apart by the excess *before* composition begins, because rule 1 forbids the excess
from resting on the neighbor instead. What neither sentence states is the arithmetic three
narrower questions ask for, none of them closed by rule 1's own wording:

1. **How much of one run's own excess lands on each of its two boundaries** — split
   evenly, assigned wholly to one side, or something else — when both a leading and a
   trailing neighbor are cl-19.
2. **Whether that split depends on which `RubyAlignment`** — nakatsuki (中付き) or
   katatsuki (肩付き) — the ruby carries. §3.3.5 states geometry for placement in both
   readings; rule 1 states a prohibition and does not say whether the natural-advance
   surplus this reading is about inherits that geometry or is computed once, alignment-free.
3. **How two runs' own demands at one shared boundary combine**, when two adjacent
   annotated base characters each overflow toward the item between them — added together,
   or the greater of the two.

## The reading

**1. The surplus splits evenly between a run's two boundaries.** For one `RubyStyle::MonoRuby`
run, `surplus = reading_extent.sub_sat(base_advance)` clamped at `InlineExtent::ZERO`, taken
through `jlreq_unit::distribute(surplus, &[Advance::new(1)?, Advance::new(1)?],
policy.remainder())` — the leading share for the boundary before the base item, the trailing
share for the boundary after it.

**2. The split is the same under both `RubyAlignment`s, computed once during lowering rather
than read from the per-construct or policy alignment.** A caller who declared
`RubyAlignment::Katatsuki` still gets the identical, alignment-free split this reading states.

**3. Two runs' shares at one shared boundary combine by `InlineExtent::max`, not by summing.**
When base item `i`'s trailing share and base item `i + 1`'s leading share both name the
boundary between them, `lower` emits one `Separation` there, `least` equal to the greater of
the two shares rather than their sum.

A `Separation` is emitted at a boundary only when the item on the far side of it — the one
the surplus would otherwise overhang — resolves to `Class::Ideographic` (cl-19) under
`jlreq_class::classify::resolve`, and only for `RubyStyle::MonoRuby`. A boundary whose far
item is not cl-19, one that does not exist because the run sits at either end of
`constructs.text()`, and every boundary of a `RubyStyle::GroupRuby` or `RubyStyle::JukugoRuby`
run, receive none this round: their own budgets are the unfilled slots
`Question::RUBY_OVERHANG_KANA` and `Question::RUBY_OVERHANG_INDENT` name for the permitted
overhang cases §3.3.8 rules 2 through 6 state, neither of which `jlreq_inline::lower` or
`jlreq_inline::place` reads yet, not a citable zero this reading asserts.
`Question::GROUP_RUBY_DISTRIBUTION` once appeared in this list too; it is removed here
because it was never the right name for *this* gap even before it was filled. The question
governs §3.3.6's own geometry — how a group-ruby run's characters sit against its *own*
base, real now in `jlreq_inline::place` (M4-a round 5) — not how far that run may overhang
an *adjacent* base character across a boundary, which is the narrower fact this paragraph
states and which remains unnamed by any question in `spec/derived/questions.tsv` for
group-ruby specifically, distinct from `Question::JUKUGO_RUBY_LAYOUT`, which is the correct
name for jukugo-ruby's own remaining gap.

## Why

**Question 1's even split is §3.3.5(a)'s own sentence, applied to the case rule 1 actually
forces.** For three or more ruby characters over one base, nakatsuki alignment states:
"position a ruby text so that its horizontal center is aligned with that of its base
character" — a centered run overflows its base symmetrically by construction, so the amount
rule 1 forbids resting on either neighbor is, absent any other information, half the excess
on each side.

**Question 2's alignment-free answer is not a shortcut; it is what §3.3.5(b) itself reduces
to once rule 1 removes its own alternative.** Katatsuki's own text for the three-or-more-ruby
case states two methods. The first, §3.3.5(b)(i), is center alignment again — the identical
sentence nakatsuki uses. The second, §3.3.5(b)(ii), decides "whether ruby hangover is allowed
on the character before its base character, or on the character after, or on both", which is
an asymmetric choice — but every one of those three options *is* an overhang onto the
adjacent character, and rule 1 forbids exactly that whenever the adjacent character is cl-19.
In precisely the case this reading's own `Separation` fires, katatsuki's own asymmetric
method has no option left to choose among, and (b)(i) — symmetric, alignment-free — is what
remains. So a caller who declared katatsuki is not being read against their own choice; they
are reading the one method §3.3.5 leaves standing once its neighbor is cl-19.

**Question 3's `max` follows from what rule 1 actually forbids.** Rule 1 states a minimum:
neither run's own ruby may rest on the shared neighbor at all, so each run's own share alone
already states the least space that neighbor's own boundary needs to stay clear of that run's
overhang. Summing the two shares would force more space than either run alone requires. The
additive reading belongs to a different sentence — rule 2's own note, about two ruby runs
converging on a shared *kana* base until they touch each other, which recommends inserting
"one em spacing between" the two ruby runs themselves. That is a statement about kana
neighbors and about the ruby text's own legibility, not about rule 1's prohibition on
overhanging cl-19, and it is out of scope with `RUBY_OVERHANG_KANA` for the same reason every
other kana-side permission is this round.

## What would change it

A revision of JLReq, or a JIS X 4051 commentary, that states an amount or a side for the
surplus a mono-ruby run's excess forces — rather than only the prohibition rule 1 states and
the centered geometry §3.3.5(a) states for placement — would settle question 1 outright.

A reading of §3.3.5(b)(ii) that extends its adjacent-character-and-script-dependent choice to
the natural-advance case this reading governs, rather than confining it to placement once
line adjustment has already happened, would give katatsuki a genuinely different answer from
nakatsuki for `separations()` and would need this reading revisited for question 2 — and for
question 3 alongside it, since an asymmetric split changes which of two adjacent runs' shares
land on a shared boundary at all.

Task #74, the independently authored conformance phase for this round's mono-ruby lowering
(ADR-0006), is what first exercised this reading's own outcome against a published case
rather than only the unit tests `crates/jlreq-inline/src/lower.rs` carries. Two published
`lower` cases now measure it, in `crates/jlreq-conform/cases/3.3.8.json`:
`3.3.8/forced-separation/only-beside-ideographic-neighbors` asserts existence and absence
alone, under `standing: "normative"` and no asserted amount, since rule 1 states the
prohibition and no arithmetic; `3.3.8/forced-separation/even-split-by-remainder-policy`
asserts question 1's own even split, under `standing: "unstated"`, with both
`adjustment.remainder` readings published side by side rather than either one asserted as
JLReq's own requirement — the identical discipline the §E.2#11 deferral already argues for a
different coordinate.
