# Reading: where the four non-ladder demerit components sit in `Preference`'s ordering

- Applies to: `jlreq_line::Preference`, `jlreq_line::Demerits`
- Standing: `Unstated`
- JLReq: §3.8.2, §3.1.12, §C.3 (silence)

## The silence

JLReq states no paragraph-level objective. One relation among a line's own cost is
nonetheless normative, and every reading below holds it fixed. §3.8.2:

> Normally line adjustment by inter-character spacing reduction is preferred. Only when
> there is no spacing that can be reduced is line adjustment by inter-character spacing
> expansion applied.

§3.1.12's worked example applies exactly that to a choice between two candidate breaks: an
opening bracket at the line end is ideally avoided by reclaiming a full em so the next
line's own first character moves up (追い込み, pull-up), and only because that reduction is
impossible is the bracket pushed down and the line expanded instead (追い出し, push-down).
Reduction is preferred over expansion, absolutely, with no condition either sentence
attaches to it.

Where the specification is silent is everything else: how the other four components of a
line's own cost — structural penalties, the last-resort re-leveling stage, summed badness,
and hanging punctuation — sit relative to that fixed pair and to one another. The nearest
thing JLReq offers is §C.3's closing paragraph:

> the very strict rule is for the best appearance at the line head, while the strict rule
> is best to avoid inter-character spacing adjustment

which is guidance on choosing one of §C.3's own four kinsoku strictness *levels*, not a
rule for ranking two already-composed candidate paragraphs against one another. It says
what each level achieves; it does not say which of two paragraphs — one that adjusted a
little on every line, one that adjusted a lot on one line and left the rest solid — is the
better outcome.

## The reading

`Question::ADJUSTMENT_PREFERENCE` (`adjustment.preference`, rule address §C.3) publishes
two permutations of `Demerits`'s six components, applied lexicographically by
`Preference::compare`, and `Preference::from_policy` reads the caller's choice by name:

- **`least-adjustment`** — `Policy::JLREQ`'s own default value, and the declaration order
  of `Demerits` itself: `structural`, `last_resort`, `expansion_depth`, `reduction_depth`,
  `badness`, `hanging`. It minimizes how deep into the ladders any line goes.
- **`even-texture`**: `structural`, `last_resort`, `badness`, `expansion_depth`,
  `reduction_depth`, `hanging`. It minimizes how uneven the lines look, tolerating deeper
  but more uniform adjustment.

Both orderings rank `expansion_depth` ahead of `reduction_depth` — the one relation §3.8.2
and §3.1.12 fix — so no choice of this question ever reorders that pair; the two
permutations differ only in where `badness` falls relative to it.

Every answer `Preference::compare` produces carries `Standing::Unstated`, because the
ranking is kumihan's own construction over a silence and not the specification's, and
`Demerits` deliberately implements neither `PartialOrd` nor `Ord` for the same reason: a
derived order would advertise as the specification's a permutation the specification only
partly states.

`docs/decisions/README.md`'s own rule is that the conformance suite carries every reading
here with *all* of its readings, not only the one this project takes. No case in
`crates/jlreq-conform/cases/` sets `adjustment.preference` today, so that is a requirement
the suite has not yet met, stated here rather than left to look met.

## Why

Publishing two named orderings, rather than picking one silently inside `Preference`'s own
implementation, is the same choice `docs/decisions/README.md` states for every reading in
this directory: a library that quietly filled the silence would publish invention as
requirement. `Demerits`'s own refusal to derive `Ord` makes the same argument at the type
level — an ordering strong enough to compare two lines' cost is exactly the kind of claim
this project does not make silently once JLReq stops supplying it.

Two orderings rather than one is what the silence leaves room for, once §3.8.2's own pair
is held fixed: `Demerits`'s remaining four components can still be ranked more than one
defensible way, and the two orderings this project publishes are named for what each one
visibly minimizes. `least-adjustment` ranks `badness` last among the four free components,
which is what keeps the ladder-depth counters — `expansion_depth` and `reduction_depth` —
decisive first: two candidates are compared first by how deep either ladder had to go, and
only a tie there falls through to how badly any one line was stretched or squeezed.
`even-texture` moves `badness` ahead of both depth counters instead, so two candidates are
compared first by how badly the worst line fared, tolerating a line that went deeper into a
ladder as long as the visible result across the paragraph is more even. Both keep
`structural` and `last_resort` first and `hanging` last, and both keep the one normative
pair in its fixed relative order — nothing about either preference touches what §3.8.2
already decided.

`structural`'s own first-rank position was reserved rather than merely declared until
§3.5.4's widow term gave it a value a real paragraph can carry (`docs/decisions/
widow-threshold.md`); this reading does not revisit the ranking now that the component is
populated — a reachable `structural` is exactly the case the ranking was already built to
decide correctly, not a new fact that reopens it.

## What would change it

`Question::ADJUSTMENT_PREFERENCE` already exists, with both choices published and
`least-adjustment` as `Policy::JLREQ`'s own default, so this is not a placeholder waiting on
a policy question to be added. What would change the reading is a revision of JLReq that
states a paragraph-level objective, or that otherwise settles where `structural`,
`last_resort`, `badness` and `hanging` sit — the specification deciding what it leaves open
today.

Evidence that publishers or other implementations systematically prefer a different order
would not change the reading on its own — it would be recorded as a `disagreements` entry
on a conformance case once one exists for this question, which is what that field is for.
