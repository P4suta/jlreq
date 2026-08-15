# Reading: what §3.5.4 leaves open about widow adjustment of paragraphs

- Applies to: `jlreq_line::compose` (`Paragraph::with_widow_threshold`, `demerits_of`,
  `ViolationKind::Widow`)
- Standing: `Unstated`
- JLReq: §3.5.4 (silence)

## The silence

§3.5.4's body, in full, `spec/derived/rules.tsv`'s own row for this address:

> The intent of widow adjustment of paragraphs is to avoid that the last line of a paragraph
> contains less than a given number of characters. This is also called "widow" processing.

Its grammar is the argument this reading is built from, not a preamble to it. It states an
*intent* ("the intent of... is to avoid"), not a prohibition — contrast §3.1.7 and §3.1.8's
own flat "shall not". It says *avoid*, not *must not* or *shall not*, the same softer verb
the section title itself uses ("adjustment", not "prohibition"). It supplies no count — "a
given number" is a parameter the specification hands the caller, which is exactly what
`Paragraph::with_widow_threshold` already is, wired to nothing until this round. And it says
nothing at all about what an implementation should do when the intent cannot be met for a
given paragraph. Four questions follow directly from what is missing:

1. **What counts as "a character."** The word is unqualified, and this crate's own break
   search does not work in code points; it works in `ItemIndex` spans.
2. **Whether a paragraph that occupies a single line can have a widow at all.** The sentence
   carries no exemption for one, read literally, but its own point — a short *stub*
   following fuller lines — presupposes an earlier line to be short *relative to*.
3. **The penalty's own shape** once a threshold cannot be met on some line no arrangement
   can rescue: flat (every shortfall costs the same) or proportional (a smaller shortfall
   costs less than a larger one).
4. **What "cannot be met" itself means for `compose` to do** — JLReq states no fallback, and
   ADR-0010 already narrows the space of legal answers considerably.

## The reading

**1. "A character" is an item: `end.get() - start.get()` over the line's own `ItemIndex`
range.** ADR-0008 makes classification — and by extension, everything else this crate counts
or compares — a function of an *occurrence*, not a code point; there is no total function
from a code point to anything in this workspace to count by instead. `demerits_of` already
receives the line as a `Range<ItemIndex>` (the same shape `adjust_line`'s own signature
already threads), so `widow_facts_of` reads the count off it directly rather than
re-deriving it from bytes or from `Text`.

The one wrinkle is stated rather than left for a reader to wonder about: a last item
`crate::ladder::hang` let hang past the measure (`Line::hanging`) is still a character *on*
that line, not excluded from this count. `hang`'s own `last` parameter is always the item
immediately before the line's own `end`, inside the line's own range and never past it — a
character rendered beyond the visual measure is still one of the characters the line
carries, and nothing about hanging removes it from `have`.

**2. A one-line paragraph can have a widow; the reading is literal.** §3.5.4's own sentence
states no exemption for a paragraph whose only line is its last, and the exempting reading
would add a condition the specification does not state — precisely the invention
`docs/decisions/README.md` exists to forbid. The reading costs nothing extra to take: for a
paragraph with exactly one candidate arrangement, the widow term is a constant addend across
every (in this case, the only) path `run_dp` compares, and `run_dp`'s own translation
invariance argument (`compose.rs`'s own C2) says a constant addend changes no comparison's
outcome — there is nothing for `Search::Optimal` to steer toward or away from, only a fact
to report. The caller who set a threshold such a paragraph cannot reach receives an
honest `ViolationKind::Widow` and a `structural` demerit that changes no choice, exactly the
same shape ADR-0010 already uses for every other unsatisfiable constraint this crate reports
rather than silently drops.

**3. The penalty is shortfall-proportional:
`u32::from(threshold).saturating_sub(have)`, not a flat one.** When no arrangement can meet
the threshold at all (question 4, below), every unsatisfiable arrangement scores identically
under a flat penalty, and the search's own choice among them is handed to whichever
component of `Demerits` happens to rank next — an accident of `Preference`'s own ordering,
not a fact about which arrangement actually came closer to what §3.5.4 asks for. A
proportional penalty keeps the search preferring the last line that falls short by *less*
over one that falls short by *more*, all the way down to the last unsatisfiable case, which
is the only reading under which "structural ranks first" (`docs/decisions/
adjustment-preference.md`) does real work on every input rather than only on the
satisfiable ones.

**4. Both mechanisms ADR-0010 already licenses, together: graceful degradation through
`Demerits::structural`, plus an honest `ViolationKind::Widow` when the arrangement finally
chosen still falls short.** Not a third option invented for this rule specifically —
exactly the pattern `docs/decisions/tolerance-exhaustion.md` already uses for a different
unsatisfiable-constraint case, restated here because the same three candidates were open
and the same one survives:

- *Refusal* — `compose` declining to produce lines for a paragraph whose widow threshold
  cannot be met — is dead on arrival. ADR-0010 states plainly that "composition never
  refuses to produce lines... because every real adopter must render something," and
  nothing about §3.5.4's own soft "intent" wording licenses an exception to that for this
  rule specifically.
- *Relaxation* — lowering the effective threshold until some arrangement satisfies it —
  reintroduces exactly the schedule-and-step-size problem
  `docs/decisions/tolerance-exhaustion.md` already rejected by name for a structurally
  identical situation: how much to relax by, and how many times to retry, are both magic
  constants this reading would have to invent with nothing in JLReq to justify either
  number.
- *Violation, with graceful degradation underneath it*, is what remains, and it is
  ADR-0010's own stated pattern applied rather than a new one: "the classic infinite-penalty
  failure cannot occur" (no break kinsoku forbids is ever taken to chase this threshold) and
  every real gap is reported as evidence rather than hidden. `demerits_of`'s own shortfall
  term steers `Search::Optimal` toward the least-bad arrangement whenever more than one
  exists to choose between; `ViolationKind::Widow`, naming
  `RuleId::WIDOW_ADJUSTMENT_OF_PARAGRAPHS` through `Violation::rule`, reports the arrangement
  finally chosen still falling short, so a caller — and a conformance case — has something
  JLReq-shaped to check.

That final clause is not incidental. `Demerits` is this crate's own invented objective
function; JLReq specifies no such thing, and a conformance case may never assert a demerit
value as if it were the specification's own answer (the rule round 8's own brief states and
every later round has held to). Wiring only the `structural` term, with no violation beside
it, would leave round 22 with nothing JLReq-shaped to assert for the unsatisfiable case, and
the rule would never be able to leave `[[deferred]]`. The violation is consequently the
point, not a garnish on top of the demerit.

## Why

**Q1's answer follows from ADR-0008 rather than being asserted.** Nothing in this crate
counts by code point anywhere else — an item is the one granularity `Text`, `ItemIndex` and
every generated table already share — so treating "a character" as anything but an item
would be the one place this crate quietly changed its own unit of measure.

**Q2's literal reading is the one `docs/decisions/README.md`'s own discipline requires, and
the translation-invariance argument is what makes it free rather than merely permissible.**
A library that added an unstated exemption "because it seems more in the spirit of widow
processing" is exactly the invention that document exists to prevent; here it is also
unnecessary, because a one-line paragraph's own widow term never changes what
`Search::Optimal` chooses in the first place — the honest report costs a caller nothing but
information.

**Q3's argument is the one already stated: only a proportional penalty makes `structural`'s
own first-rank position do real work in the one case — an unsatisfiable threshold — where a
flat penalty would make it silently inert.** `docs/decisions/adjustment-preference.md`
already ranks `structural` first in both published orderings; a flat penalty would make that
placement decorative on exactly the paragraphs where a caller most needs the search to
discriminate.

**Q4's argument is ADR-0010 applied, not re-derived.** ADR-0010 was written before this
round and settles the shape of every "no legal answer satisfies this constraint" case this
crate has met since — `docs/decisions/tolerance-exhaustion.md` is the other one on record.
Refusal contradicts ADR-0010's own text directly. Relaxation was already rejected, by name,
for the identical reason (an invented schedule) in that sibling reading, and nothing about
§3.5.4 makes the widow case different enough to deserve a different verdict. What remains —
steer where a choice exists, report where it does not — is not a third option so much as the
one ADR-0010 already describes, applied to a rule that had not yet needed it.

## What would change it

A revision of JLReq that states a character-counting rule more specific than "an occurrence
of an Appendix A key" — for instance, one that explicitly excludes a hung character from the
count, or that states widow adjustment does not apply to a paragraph of one line — would
settle questions 1 and 2 outright, because both readings here are argued from the
specification's own silence rather than from a stated rule they would then contradict.

Evidence that real adopters treat an unsatisfiable widow threshold as a hard failure worth
refusing composition over — rather than a soft preference reported alongside whatever the
search otherwise produces — would not by itself overturn question 4's reading: ADR-0010's
own "every real adopter must render something" is a stronger, workspace-wide commitment than
one rule's own preference, and changing it would mean revisiting ADR-0010 itself, not this
reading alone.

A future round wiring a genuinely graduated schedule of widow tolerance — for instance,
a caller-declared willingness to relax the threshold by a stated amount rather than an
implementation-invented step size — would give question 4's rejected "relaxation" candidate
a real, non-magic parameter to relax by, and would be worth revisiting against this reading
at that point; nothing in §3.5.4's own text asks for one today.

The conformance suite (`crates/jlreq-conform/cases/3.5.4.json`, three cases, task #58 round
22 — the independently authored phase ADR-0006 requires, following this round's own
implementation the way `docs/decisions/README.md`'s own rule requires) now carries Q1 and Q2
above, each with its own alternative recorded rather than only the reading taken: Q2 is a
one-line paragraph whose declared threshold exceeds its own item count, with the exempting
alternative in that case's own `forbidden`; Q1 is a threshold-equal/threshold-past-the-count
pair over a last line built so its item count diverges from both its code-point count and
its byte count, so the pair discriminates item-counting from either of those. Q4 is carried
through the same channel both of those cases already use — a non-empty `lines` beside a real
violation is the claim that composition degrades gracefully rather than refusing, and the
Q1 pair's own second case additionally rejects the relaxation alternative by name. Q3, the
penalty's own shape, is not carried: no `Policy` question selects a flat penalty, so no case
in this format can compare the proportional reading against a reachable alternative, a limit
of the format rather than of this round's own effort, named in `docs/conformance-deferrals.toml`'s
own `[[owned]]` entry for this rule rather than left to look settled. The hanging wrinkle
Q1's own reading states (a hung last item still counts) is likewise uncarried: it requires
`adjustment.hanging_punctuation = hanging`, which `Policy::JLREQ` does not select, so a case
exercising it would be published but never attempted — the same entry names that gap too.
