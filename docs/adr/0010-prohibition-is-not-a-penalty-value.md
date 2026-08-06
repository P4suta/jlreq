# ADR-0010: prohibition is structural, not a large number

- Status: accepted
- Date: 2026-08-05

## Context

M3 composes a paragraph by minimizing a cost over candidate break sequences. The standard
formulation encodes a forbidden break as a penalty of ten thousand or of `i32::MAX`, and
lets the optimizer discover that taking it is expensive.

That works until the paragraph is pathological. A long Western word, a URL, or a run of
inseparable characters can leave every alternative worse than the sentinel, and the
optimizer then takes the forbidden break — silently, producing a line starting with `。`
that a Japanese reader recognizes instantly. Saturating arithmetic makes it worse: two
large finite penalties saturate to the sentinel and become indistinguishable from a
prohibition. The failure is rare, invisible in testing, and exactly the class of error this
library exists to eliminate.

JLReq's prohibitions are not strong preferences. §C.3's preamble states that breaking after
an opening bracket and before a closing bracket, a full stop, or a comma "is prohibited at
all levels," and §3.1.8 states the line-end prohibition with no relaxing note anywhere.

The specification also has a third category. The `×` mark in Tables 1 through 6 is glossed
in English as "not allowed due to line breaking rules or other restrictions"; the Japanese
is decisive and says the placement is prohibited by 行頭禁則, 行末禁則, or another rule. It
is the kinsoku prohibition restated at a line edge, so it is policy-dependent, and it is an
outcome the composer must avoid rather than an assertion that the caller's text is
malformed.

## Decision

A prohibition is not representable as a cost. The type that answers "may a line end here"
has no ordering, no arithmetic, and no conversion to any numeric type, so there is no
expression that turns a prohibition into demerits — not because writing one is discouraged,
but because no function with that signature exists.

Hard constraints are applied by construction rather than by comparison. The optimizer
consumes a set of feasible breaks that only the kinsoku evaluator can build, and neither the
feasible set nor an individual feasible break has a public constructor. A forbidden break is
therefore absent from the search space rather than expensive within it, and a caller who
wants to force one cannot.

The `×` is one of those constraints and not a separate mechanism. Its Japanese gloss says the
placement is prohibited by 行頭禁則, 行末禁則, or another rule, which is the kinsoku
prohibition restated at a line edge — so a candidate whose resulting line edge would produce
a `×` is refused by the same computation that refuses every other prohibited break, and it
appears in the rejection report with its rule like every other. The violation kind of the
same name is what remains for the case the specification forces on us: when no arrangement is
feasible, composition still emits lines, and the `×` it could not avoid is reported there.
One concept, two appearances, and which one a case exercises is decided by whether a feasible
alternative existed.

A policy that the specification makes self-contradictory is refused the same way. §C.3
defines its strictest level as applying no §C.2 alternate rule, so a policy naming both is
not a thing to validate later but a value that is never built: setting a choice returns a
result, and the conflicting pair is the error. No entry point checks a policy, because an
invalid one has no representation to reach them with.

Infeasibility carries evidence instead of discarding it. When no arrangement satisfies the
measure, the result names the shortfall, the deepest adjustment stage reached, and the rule
that blocked the alternative — which is what a caller needs in order to report the problem
rather than guess at it, and what an infinity throws away.

Composition never refuses to produce lines. A paragraph that cannot be composed within the
rules returns lines together with the violations incurred, each naming its rule, because
every real adopter must render something and the alternative is that each writes its own
emergency breaker outside JLReq and outside our record.

The soft half of the objective is a vector of independent counts compared
lexicographically, not a weighted sum. JLReq states its preferences as an order — §3.8.2
says expansion is applied only when there is no spacing left to reduce — and a scalar cost
would require inventing an exchange rate the specification does not give. Componentwise
addition under a lexicographic order is a totally ordered group, so the dynamic program
runs over it unchanged, in exact integers, with no sentinel in it.

The order itself is not fixed in the type, and it is part normative and part silence. One
relation the specification does state: §3.8.2 says that "normally line adjustment by
inter-character spacing reduction is preferred. Only when there is no spacing that can be
reduced is line adjustment by inter-character spacing expansion applied", and §3.1.12's
worked example applies exactly that to a choice between two breaks — the opening bracket at
the line end is ideally avoided by reclaiming a full em so the next line's first character
moves up, and only because that reduction is impossible is the bracket pushed down and the
line expanded. Ranking the expansion component before the reduction component reproduces both
sentences, so that pair is normative and every permutation the library offers holds it fixed.

Where the four remaining components sit is a silence. §C.3's closing paragraph describes what
its four *levels* achieve — "the very strict rule is for the best appearance at the line
head, while the strict rule is best to avoid inter-character spacing adjustment" — which is
guidance on choosing a level, not a rule for ranking two candidate paragraphs. Their
placement is therefore kumihan's, published as a reading of a silence, with two named orders
and a conformance case pinning each. The demerit type implements no ordering of its own,
because a derived one would advertise as the library's the very order the library declines to
fix.

Nothing that follows from the objective is a second mechanism either. §3.1.12's pull-up is
the reduction-preferring comparison above, applied to two candidate breaks that both exist;
it is reported by the line that took it rather than offered by the feasible set. And hanging
punctuation (ぶら下げ) is a stage of the same ladder, because §2.5.1 says it is "only
necessary for full stops (cl-06) and commas (cl-07) when they would otherwise need to be
wrapped to the line head" and §3.8.2's note says it is used "in order to avoid the addition
of inter character spacing" — a line that fits without it does not use it, and a line that
needs it uses it before expansion. Reduction, then hanging, then expansion, in one ladder
that the greedy and the optimal search share, which is what makes the cross-search agreement
gate satisfiable rather than aspirational.

What a caller tunes is consequently not a demerit vector. The optimizer's one knob is a
badness tolerance — TeX's quantity: a single bounded integer with an ordinary constructor —
above which a line stops being considered. A demerit vector is an output and has no literal
form, so the knob and the verdict cannot be mistaken for one another, and the cap on badness
is a value ordinary lines reach rather than a sentinel that means something.

Badness is therefore the one quantity in the objective that *does* have a numeric
constructor and a numeric accessor, and that is deliberate rather than an omission from the
denylist. It is an input, it is bounded, its cap is reachable, and no prohibition is ever
expressed in it — the thing the denylist protects is the type that answers "may a line end
here", and badness answers "how well does this line read". The distinction is recorded here
so that a later reader can tell the exception from an oversight.

## Consequences

The classic infinite-penalty failure cannot occur, including on pathological input, and it
cannot be reintroduced by a later change without deleting a type. That claim is held by a
gate rather than by the shape alone, because the shape is one `impl` block away from being
undone. The gate reads a committed list of types and forbids, for every one of them, an
implementation or a derive of any trait that would supply an order, a numeric conversion, or
an arithmetic operator — matched on the trait's name rather than on its path, so importing
it first does not evade the check — and it additionally asserts that neither the feasible set
nor the feasible break has a public function returning itself other than the one factory.
Naming only the set would have left a later constructor on the break free to defeat it, so
both are listed. Both halves are lexical and both fail closed on a type added to the
workspace and not placed in the list.

Reduction depth and expansion depth stay separate components, because §3.8.2 orders the two
ladders absolutely and merging them would let a little expansion outrank more reduction.
And the unbounded last expansion stage of §3.8.4 is a kind rather than a magnitude, for the
same reason the prohibition is: an unbounded quantity written as a large number is a
sentinel waiting to be compared.
