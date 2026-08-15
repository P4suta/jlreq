# ADR-0021: Table 6's expansion opportunity belongs to the boundary, not the conditional space

- Status: accepted, amended below (2026-08-09) to give the opportunity's own citation a
  carrier
- Date: 2026-08-08

## Context

ADR-0014 modeled `Expansion` as a field of `ConditionalSpace`, the same shape it gave
`Reduction`. The two are not the same shape, and the difference was reachable in the
generated data the whole time.

Table 1 states an amount per referent — `be`, `af`, or both at once — and Appendix D's
reduction floor rides along with whichever term it qualifies, because a reduction is a fact
about *shrinking a stated amount*, and a stated amount is a referent's own contribution.
Table 6 states something else: one cell per class pair, with no `be` or `af` column at all.
Its own legend never assigns an opportunity to a neighbor; §3.8.4 step (c) speaks only of
"places" that "do not have the inseparable character rule", spaced "equally with
proportional character size" — a fact about the *coordinate*, not about either character's
own em.

Carrying that fact on `ConditionalSpace` made it invisible wherever no `ConditionalSpace`
existed to carry it. `jlreq_spacing::evaluate::spaces_of` builds one `ConditionalSpace` per
Table 1 *term*, so a solid Table 1 cell — no term of either referent's — produced zero
`ConditionalSpace`s and consequently zero opportunity to read `Expansion` off, regardless of
what Table 6 stated at that same coordinate. cl-19 against cl-19 (kanji beside kanji) is
`blank` in Table 1 and `0-1/4 stage 3` in Table 6, measured directly against the captured
tables: of Table 6's 494 non-`blank`, non-`×` cells, 296 sit at a Table 1 coordinate that is
`blank`. Every one of those 296 was structurally unreachable through `jlreq_spacing::boundary`
— not merely untested, but unreachable by any input, on any policy — which made §3.8.4's own
expansion procedure inert on ordinary Japanese running text: kanji beside kanji, kana beside
kanji, and the like are exactly the adjacencies Table 1 leaves solid.

The same measurement rules out the alternative of leaving the carrier alone and patching
around it. Synthesizing a zero-amount `ConditionalSpace` at a solid coordinate to give
`Expansion` somewhere to live would require publishing a `Referent` — `Preceding` or
`Trailing` — for a boundary §3.8.4 (c) names no owner for at all. Doing so would be exactly
the fabricated-owner shape ADR-0014's own Decision already refused for ruby overhang: "the
second has no space to attach to, so the permission belongs to the boundary and distinguishes
the two." An expansion opportunity at a solid boundary has no space to attach to for the
identical reason.

## Decision

`Expansion` and `ExpansionStage` stay exactly as ADR-0014 defined them; only their carrier
changes. `ConditionalSpace` no longer has an `expansion` field or an `expansion()` accessor.
`jlreq_spacing::Boundary` gains `expansion() -> Expansion`, read once per coordinate by
`evaluate::boundary`'s own Table 6 lookup, independent of how many Table 1 terms — zero, one,
or two — that same coordinate produced.

This is [ADR 0019](0019-one-fact-one-carrier.md)'s own rule, applied to a fact this workspace
already had and had put in the wrong place: "every fact the library uses has exactly one
carrier", and the carrier is picked by what measured or fixed the fact, not by which type
happened to exist first. Table 6 is the measurement; it names a class pair, not a referent,
so the class pair — the boundary — is the carrier. It is also the identical move ADR-0014's
own Decision already made one section earlier, for the reason quoted above; this ADR states
explicitly what that Decision's own reasoning already implied and did not yet apply to
`Expansion` itself.

The move is safe in the one way that could have made it unsound: a boundary that already has
two `ConditionalSpace` terms could not also need an independent, boundary-level `Expansion`
without contradicting itself, because a term-carrying site already has an owner an amount
could expand from. Checked, not assumed (ADR-0009's discipline, which ADR-0014 already
applies to its own at-most-two-spaces bound) — across every one of Table 6's 494 non-blank
cells, zero sit at a Table 1 coordinate that carries two terms. `xtask attest`'s
`expansion-needs-no-referent` invariant asserts this over the captured tables on every run,
so a future revision of either table that broke the assumption would fail the build rather
than silently produce a private `pipeline::ExpansionSite` asked to carry two owners' worth of
expansion room in the one slot ADR-0014's own bound gives it.

`ConditionalSpace::reduction` and everything about the D.2 stage split are unaffected: a
reduction genuinely is a fact about a stated term, because there is nothing to reduce where
no amount was ever stated, and that half of ADR-0014's Decision was correct as written.

## Consequences

Every consumer of a boundary now reads two independent facts where it used to read one
combined one: `Boundary::spaces()` for what either neighbor contributes, and
`Boundary::expansion()` for whether — and how far — the coordinate itself may be opened up,
whether or not either neighbor has a term there. `crates/jlreq-line/src/ladder.rs`'s own
`Site` carries both, separately, so a term-free boundary can still produce exactly one
adjustable site rather than none.

The published conformance format gains a boundary-level `expansion` object beside `spaces`,
for the identical reason ADR-0014's own Consequences section states for the components it
already separated: an implementation that reports Table 6's opportunity only where Table 1
also gave the boundary a term agrees on every case the old suite could publish and disagrees
on the 296 coordinates this ADR makes newly assertable — which is exactly the gap a case
format sharing one field between two fact could not have shown.

Nothing about ruby overhang's own model changes; it was already carried this way, and this
ADR is that same reading extended to the one place ADR-0014 had not yet applied it.

## Amendment (2026-08-09): the citation is `Option<RuleId>`, not folded into `Expansion`

Every other captured cell in this workspace's six matrices publishes both an amount and the
rule that states it — `ConditionalSpace::rule`, `Breakable::No`'s own `rule`,
`Placement::Forbidden`'s own `rule`, `Delegation.rule`. `Boundary::expansion` was the one
exception: the field this ADR's own original text gives it carries `Expansion` alone, and
every generated `RawRangedCell` of Table 6 — the cells
`crates/jlreq-spacing/src/generated/table6.rs` compiles — already carries a `rule: RuleId`
beside the amount it transcribes, unread by anything until this amendment.
Five published conformance cases (`E.2.json`, `E.json`) declared a `rule` against exactly
this field months before anything checked it, and three `[[deferred]]` entries
(`docs/conformance-deferrals.toml`'s own `E.2#8`, `E.2#9`, `E.2#11`) said in their own words
that a coordinate answering `Expansion::None` "is indistinguishable from a bare absence" —
which was true, and was this gap.

**The decision.** `Boundary` gains `expansion_rule() -> Option<RuleId>`, read from the same
Table 6 row `expansion()` itself reads (or from the note that governs one of the two
note-conditioned rows, `(8, 8)` and `(27, 13)`), independent of whether that row's own answer
was a real ceiling or the row's own denial of one. `None` means what it means everywhere
else in this crate's public surface where an `Option` names a table row: no row exists at
this coordinate at all. `Some` means a row (Table 6's own, or the note that governs it) spoke
about this coordinate — including when what it said was that there is no opportunity, which
is `Some(rule)` beside `Expansion::None`, not a contradiction: the amount and the citation are
two different questions, "how far may this be opened" and "which sentence answers that",
and the first answering "not at all" does not make the second unanswerable.

**Why not a field of `Expansion` itself.** `Expansion` is a kind, not a record (ADR-0010):
`Range { ceiling, stage }`, `Residual`, and `None`, matched exhaustively wherever a caller
needs the amount and nothing else. Giving `Expansion::None` a `rule` field two of its three
variants would never populate — `Range` and `Residual` already carry their own citation
nowhere, because until this amendment nothing needed either to — would make one variant of a
closed enum structurally different from its siblings for no reason visible at any call site
that matches on it, and would force every existing `match expansion { .. }` in this workspace
(`jlreq-line`'s own `ladder.rs`, `jlreq-conform`'s own `case_expansion_of`) to either destructure
a citation it does not want or bind and discard it. A boundary-level accessor beside
`expansion()` — the identical shape ADR-0021's own original Decision already gives `spaces()`
beside `ruby_overhang()` beside `expansion()` itself — costs nothing at any of those sites and
answers a genuinely different question from `expansion()`'s own.

**Why the `Option` is not smoothed into a always-`Some` citation.** A version of this
accessor that fell back to a generic "spacing between characters" rule wherever Table 6 held
no row — the shape `empty_boundary()` already gives `breakable` and `placement` for cl-17 and
cl-18 — would have thrown away exactly the distinction this amendment exists to publish: "no
row" and "a row states nothing new" are different facts, and Table 6's own six-matrix cousins
already keep them apart (`break_cell` returning `None` is `Standing::Unstated`; a `break_cell`
returning a real, all-zero-bitmask row is `Standing::Normative`, `evaluate_breakable`'s own
doc states the distinction directly). `Boundary::expansion_rule` draws the identical line for
Table 6 that `evaluate_breakable` already draws for Table 2, rather than inventing a third
reading for the one matrix ADR-0021's own original text moved off `ConditionalSpace`.

**`rules_fired` reports it.** `crates/jlreq-spacing/src/evaluate.rs`'s own `rules_fired`
widened from five slots to six once this citation existed to report, fixing an adjacent
off-by-one in the same motion: the delegation write at the fifth slot never advanced the
running index that fed it, which cost nothing while nothing was written after it, and would
have overwritten the delegation the moment the expansion citation was appended the same way.

**`jlreq-conform` carries and compares it.** `CaseExpansion` gained `rule: Option<String>`,
published on every one of `case_expansion_of`'s three branches including `"none"`, and
`check_expansion` compares it under semantics distinct from every other field the runner
checks: conditional on the expectation stating one, passed over (not failed) when the
expectation states one and the answer publishes none, and a real failure only when both sides
publish an address and it differs. `check_class`'s own doc already gives an implementation
the right to answer a classification without publishing a rule for it (ADR 0006); this is
the identical right extended to one field of a boundary answer rather than a whole answer,
which is why the comparison is conditional rather than symmetric with, say, `kind` or
`stage`.

**What this closes and what it does not.** The five published cases' own declared addresses
were audited against `crates/jlreq-spacing/src/generated/table6.rs` and against JLReq's own
text for the cited note; all five agree with the generated cell and none needed correction —
recorded in full in `CHANGELOG.md`. `docs/conformance-deferrals.toml`'s own `E.2#8`, `E.2#9`
and `E.2#11` entries are repaired to state what is true now — the coordinate publishes a
citation a case can assert against — rather than the "indistinguishable from absence" claim
this amendment removes; they stay `[[deferred]]`, because publishing the citation is not the
same act as writing the case that measures it (ADR 0006's own phase split), and `E.2#11` in
particular still names an open question about whether its own coordinate measures the
specification at all, which this amendment does not settle.
