# ADR-0014: the unit of spacing data is the conditional space, not the table cell

- Status: accepted
- Date: 2026-08-05

## Context

The obvious model of Appendix B is that a cell holds an amount. Every implementation that
models it that way gets Appendix D wrong, and the specification says so in three places.

§B.2 note 3 states that the space between two middle dots "shall be the sum of a quarter em
of the preceding middle dots and a quarter em of the trailing middle dots." That is two
quantities in one cell, taken from two different characters' ems.

§B.2 note 5 does the same across classes: a full stop followed by a middle dot is "the sum
of the half em spacing of the full stop and the quarter em spacing of the middle dot."

§D.2 note 3 then gives the two halves of one such cell different reduction priorities —
the comma's half em is fifth in Table 3 and third in Table 5, while the middle dot's
quarter em is fourth in Table 3 and second in Table 4. A cell holding one number cannot
express a quantity whose two halves are reduced at different times.

JLReq's own English for these objects is "the conditional half em space accompanying the
preceding comma," and it never treats the cell as the thing being adjusted.

## Decision

The atom is one conditional space: an amount, the neighbor whose em the fraction is taken
of, whether and how far it may be reduced, whether and how far it may be expanded, and the
rule that states it. A boundary carries at most two of them, and the bound of two is
structural rather than empirical — a space between two characters has exactly two owners,
which is also why Appendix B's referent vocabulary is exactly `be` and `af`. The bound is
also checked rather than trusted: `xtask attest` requires every captured Table 1 cell and
every override the notes produce to yield at most one contribution per referent, so a
transcription that read a three-term sum out of the legend fails the build instead of
silently losing a term at the far end.

Reducibility is a kind rather than a range, because §3.1.9 says twice that at the line end
"the possibilities are only half em spacing or solid. Other spacing, such as quarter em
spacing should not be used." A conditional space is therefore rigid, continuously reducible
to a floor, or two-valued — Appendix D writes those last two as `1/2–0` and `1/2=0`, and an
implementation with one continuous notion of shrink emits the quarter em the specification
forbids.

The priority ordinal a conditional space carries is typed by which ladder it belongs to.
Appendix D's six reduction steps and Appendix E's four expansion steps are two orderings of
two different things, and §3.8.2 orders the ladders themselves absolutely — expansion is
reached only when nothing is left to reduce. A single ordinal type shared by both would let
"stage 2" mean two things in one report and in one published case field, in a design that
types apart every other pair of same-shaped quantities.

Ruby overhang does not live on the conditional space. Appendix B's legend defines two
structurally different permissions: `1/2 be hang` lets ruby extend over that spacing and
"shall not be extended over the other character," capped by whatever survives line
adjustment, while `ruby hang` is set solid and lets ruby extend over the adjacent character
itself. The second has no space to attach to, so the permission belongs to the boundary and
distinguishes the two.

Amounts are not confined to Table 1's vocabulary. §3.1.6 requires one em after a dividing
punctuation mark ending a sentence, an amount Table 1's legend has no token for, so the
amount type is a quantity rather than an enumeration of the tokens.

Where a cell's answer is not a value at all — §B.2 notes 9 through 11 delegate the
same-run case to the ruby and superscript placement procedures — the boundary says so and
names the procedure, rather than inventing a number.

## Consequences

Every consumer of a boundary handles up to two components with separate priorities, where a
scalar model would have handled one number. That is the cost, and it is the price of being
able to state §D.2 note 3 at all.

The published conformance format carries the components, not their sum. An implementation
that assigns the two quarter ems of a middle-dot pair to the wrong owners, or drains them
in the wrong order, sums to the same total and would score as agreeing under a scalar
format; under this one it does not. That is why this is a one-way door: it is baked into
every case file the suite publishes.
