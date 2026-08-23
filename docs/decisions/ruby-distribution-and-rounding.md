<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: how a ruby run distributes its surplus, and how the odd unit rounds

- Applies to: the ruby placement round in
  [`pipeline`](../../crates/jlreq/src/pipeline.rs), and the same round in
  [`engines/ocaml/lib/pipeline.ml`](../../engines/ocaml/lib/pipeline.ml)
- Standing: `Unstated`
- JLReq: §3.3.5, §3.3.6, §3.3.8, §F.2, §F.3
- Observed by: `just census ruby` (37,030 requests), in the variants that set a reading at
  two fifths of the base character's em

## The silence

Four questions, all about what happens to the space a reading and its base characters do not
share exactly.

**§3.3.6 is stated over a plural that a run of one does not have.** Both of its methods are
stated over

> inter-character spacing between each adjacent base character

and the end gaps that go with it. A run over a *single* base character has no adjacent base
character, so neither method has an argument, and §3.3.6 does not say which section takes
over.

**§3.3.6's `1 : 2 : … : 2 : 1` does not say what its two outer units are.** The ratio puts
one unit before the first base character and one after the last. Whether those two units are
spacing inserted on the line, or the same quantity §3.3.8 would have let the reading spend
as an overhang over the neighbor, is not stated — and the two produce different lines
wherever §3.3.8's permission is available.

**§3.3.5 and §3.3.8 rule 1 are silent about two roundings.** A reading wider than its base
character has to be centered on it, and the difference is often odd. Which way the odd unit
falls is not stated. Neither is whether the space that a reading's overflow forces open at
the two boundaries follows `adjustment.remainder` — the same question, asked of a different
quantity.

**§F.3's formula refers to its own result.** It states the total as

> Total inter-character spacing = (the sum of the length of those ruby characters forced
> out from the corresponding base character) - (the sum of the length of those ruby
> characters which overhang other base characters) - (the sum of the length of those ruby
> characters which overhang other non-base characters).

The second and third terms are geometric facts about a compound whose base characters have
*already been pushed apart by the total being computed*. An engine cannot evaluate the
right-hand side in the order it is written, because two of its three terms are not known
until the left-hand side is.

## The reading

**A run over one base character whose reading is longer is set by §3.3.5 and not by
§3.3.6.** The reading is centered on the base character and hangs over both neighbors as far
as §3.3.8 allows. `ruby.group_distribution` selects nothing for such a run — `flush` and
`jis` alike — which is observable because the two answers *do* differ, at the same ratio, for
a run over two.

**A group run's leading and trailing shares are spacing on the line and never an overhang.**
For a run over two or more base characters, the outer units of §3.3.6's ratio are inserted
even where §3.3.8's own permission would have let the reading go over the neighbor instead.
A mono run's two shares are the opposite: always an overhang, as far as the permission goes,
and spacing only for what is left.

**§3.3.5's centering takes the lower half of an odd difference, and the space its overflow
forces takes `adjustment.remainder`.** A reading 1665 units wide on a 1000-unit base
character opens 333 units before that base character and 332 after — the remainder answer's
own order — while the reading itself starts 332 back from it rather than 333. One is a
center and the other is two adjustment sites, and they round differently on purpose.

**§F.3's total is the least total the compound fits at.** The answer is the smallest value at
which every ruby character has somewhere to go — a ruby character's em into the base
character beside it, and §3.3.8's own allowance outside the compound — which the engine
finds by bisection rather than by evaluating the formula forwards.

## Why

**§3.3.6 with no adjacent base character is not a degenerate case of §3.3.6; it is §3.3.5.**
A method stated over a set of inter-character boundaries has nothing to distribute when the
set is empty, and reading it as "distribute nothing, then apply the end gaps" would give the
two methods the same answer — which is what makes the alternative reading detectable: under
it, `ruby.group_distribution` would select at a run of one and produce two identical layouts,
which is a policy question that answers nothing. §3.3.5 is the section that already governs a
reading against its own base character, and a run of one is exactly that shape.

**The outer units are not the same quantity as the overhang, because the ratio placed
them.** §3.3.6's ratio is a *distribution*: it says how a known surplus is divided among a
known set of sites, and its first and last sites are inside the run's own extent. §3.3.8's
permission is about a surplus that has nowhere to go inside the run at all. Spending a
distributed unit as an overhang would take a quantity the ratio assigned to a site and place
it somewhere the ratio does not reach, and would make `1 : 2 : … : 2 : 1` a ratio over a set
whose membership depends on what happens to stand beside the compound. The mono case is the
opposite because §3.3.5 assigns nothing: what a mono run has is surplus, and surplus is what
§3.3.8 is about.

**Two roundings, because they are two different quantities.** A center is a geometric fact
about one object against one other, and a center that consulted a paragraph-level style
setting would make the same reading sit in two places depending on an answer that has
nothing to do with it. The space an overflow forces is an *adjustment site*: it is space
added to the line, of the kind `adjustment.remainder` exists to order. Giving the two the
same rule would either make centering style-dependent or make an adjustment site ignore the
setting that governs every other adjustment site — and the first of those is the one an
engine notices, because §3.3.5's centering is asked of every mono ruby on every line.

**"The least total it fits at" is the only forward reading of §F.3.** The formula's terms
are functions of the layout, so the equation is a fixed point rather than an evaluation
order, and a fixed point is what bisection finds. Taking the *least* such total is what
§F.2's own subject — how far a reading may reach into the base characters beside it —
already implies: a larger total would push base characters apart further than any ruby
character needed. At a ruby em that divides the base character exactly, the least total and
a forward reading of the formula are the same number, which is why half the suite's own §F
cases cannot tell them apart, and why the `ruby` census sets some of its readings at two
fifths of the base character's em instead of at §3.3.3's half.

## What would change it

A revision of §3.3.6 that states what its methods do for a run of one settles the first
reading. A revision that says whether the ratio's outer units may be spent outside the run
settles the second. A statement of the rounding in §3.3.5, or a note attaching
`adjustment.remainder` to the centering as well as to the adjustment sites, settles the
third; the concrete evidence would be a conformance case at an odd difference carrying both
roundings as `disagreements`.

For §F.3, a revision that states the total as a fixed point — or that states an evaluation
order for the three terms — settles the fourth. Until then, any engine that reads the
formula forwards will agree with this one at every ruby em that tiles the base characters and
disagree at every one that does not, which is a difference a conformance case at a
two-fifths em would make visible in a single line.
