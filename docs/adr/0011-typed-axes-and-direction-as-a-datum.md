# ADR-0011: the axes are types, and direction is a datum three rules read

- Status: accepted
- Date: 2026-08-05

## Context

[ADR 0004](0004-writing-mode-abstraction.md) decided that vertical writing is a direction
rather than a second implementation, and that the core has no notion of x and y. It did not
say how that is held, and the obvious readings of it are both wrong.

The first wrong reading is that discipline holds it. It does not. The horizontal assumption
does not arrive as a variable named `x`; it arrives when someone adds a line's advance to
its stacking offset because both happen to be integers. Naming conventions do not stop
that.

The second wrong reading is that the word "vertical" must be unpronounceable inside the
library. Making the direction type opaque and giving it two boolean predicates produces a
type with two inhabitants and two total predicates that each separate them, which is the
mode flag again under another name. A lexical ban on the token fares no better: the
generated rule inventory has to carry JLReq's own section headings, and §2.3.2 is titled
"Major Differences between Vertical Writing Mode and Horizontal Writing Mode."

The honest question is how many rules JLReq genuinely conditions on the direction, and the
answer is three, not zero. Tate-chu-yoko (縦中横) exists only vertically, §3.3.5 says
katatsuki (肩付き) ruby alignment "should not be adopted" horizontally, and §3.1.3 sets
ideographic numerals with `、` and `・` solid in vertical writing — the last being a
spacing amount, in the layer
a lexical ban would have made incapable of expressing it. Everything else JLReq states
twice is exact axis mapping, and several of those pairs collapse into a single rule: the
ruby side, the reference-mark alignment, and the first-line and last-line escape rules are
each one statement about the block axis written out twice because the specification speaks
in physical axes.

## Decision

Inline and block are distinct types. An offset along the axis a line advances on and an
offset along the axis lines stack on are different types with no conversion in either
direction and no arithmetic accepting the other, so mixing them is a compile error rather
than a review finding. There is no width, no height, no x, and no y in the workspace.
Mapping the two axes onto screen coordinates happens in the caller's renderer, and no
helper for it is offered — such a helper needs a handedness, an origin, and a sign that
jlreq cannot validate, and in vertical writing the block axis runs right to left, so a
wrong sign renders a plausible mirrored page.

Separation by type is only as strong as the conversions that exist, and this decision states
honestly how strong that is, because the earlier version of it overclaimed. No *typed*
conversion exists: there is no `From`, no `Into`, no `Deref`, no `Ord` and no arithmetic
accepting the other axis, on any of the four types or between one of them and the caller's
scalar, and `docs/api-frozen.toml` names every one of them so a later `impl` block is a gate
failure. But an axis type must be constructible from the caller's numbers and readable back
into them, because the caller supplies advances and draws glyphs. That constructor and that
accessor are a round-trip pair laundered through `i32`, and no arrangement of types removes
them: any scheme that lets a value in and out lets it in on one axis and out on the other in
two well-typed steps. Claiming otherwise, in a file whose whole purpose is to be believed by
a later reviewer, would be worse than the leak.

So the untyped channel is narrowed rather than denied. Building an axis value from a plain
integer, and reading one back out, may happen only inside the module that defines the four
types and inside items named in `docs/scalar-sites.toml` — a code-owner-guarded file of the
same shape as the direction allowlist, each entry carrying the crate, the item, and the
reason a raw quantity is unavoidable there. The list is short by construction, because the
inherent arithmetic surface is closed over each axis type and the public outputs are already
typed; what remains is the ratio the badness function needs, the bridge to the conformance
case format, and the two items inside `jlreq-unit`'s own arithmetic module that turn a
count this library computed back into a typed value — the inline cursor's position and the
distribution's parts. Everything outside the list is a gate failure, so a cross-axis
assignment has to be argued for in a reviewed file rather than written.

The four types hold their integer in a *private* field, and the crate reaches the channel
through a named crate-visible pair rather than through the tuple field. A `pub(crate)` field
would have been a second channel that names no method, invisible to a gate that matches
method names — and `InlineExtent(raw)` and `value.0` are exactly the forms a cross-axis slip
inside `jlreq-unit` would take. For the same reason the macro that generates the closed
arithmetic is defined in the arithmetic module and *expanded* in the two modules that
declare the types, so its `Self(units)` never opens the channel in a third place.

Direction is an ordinary value on the composition input, and exactly three rules read it.
Each is an entry in the generated rule inventory marked direction-conditional there, and a
gate asserts that the set of rules whose evaluation consults the direction equals that
generated set. The gate needs a mechanism, and a token scan is not one: the direction is
threaded by signature through most of the workspace, and the generated inventory carries
JLReq's own heading "Major Differences between Vertical Writing Mode and Horizontal Writing
Mode." So the gate is defined on the only construct that can actually branch — a named
variant of the direction — and reads three sources.

In hand-written core sources, with comments and string literals stripped, a variant of the
direction may appear only inside an item named in `docs/direction-sites.toml`, a
code-owner-guarded file whose entries each carry the crate, the item, the rule address, and
the reason. Naming the type is unrestricted, because passing a value through a signature is
not a branch. In generated sources a variant may appear only in a direction-conditional
predicate row. And the union of the rules named by those two — the allowlisted sites and the
generated predicate rows — must equal the set of rules the inventory marks
direction-conditional, which today is §3.1.3, §3.2.5, and §3.3.5 and nothing else. Adding a
fourth is a change to generated data plus a code-owner review, not an incidental branch.

The claim ADR 0004 actually makes — that there is no second code path — is proved rather
than asserted. Every conformance case not marked direction-specific is composed twice, once
each way, and the inline results must be bit-identical. A leak shows up as a failing case
over the whole corpus rather than as a code review that has to notice it.

## Consequences

The three direction-conditional rules are read in three different places, and the earlier
version of this decision had one of them wrong. §3.2.5 is an availability fact — JLReq defines
no horizontal tate-chu-yoko at all — so it is refused where the construct is built. §3.3.5 is
*not* one: it says katatsuki "should not be adopted" for horizontal writing, which is a
recommendation over a construct that is perfectly well defined there, so refusing it at
construction would have published a prohibition the specification does not state. It is a
policy question whose JLReq value follows the recommendation, resolved once during lowering —
which is the allowlisted site that reads the direction — and a caller who overrides it is
honored and told. §3.1.3 is a spacing amount and reaches the boundary evaluator as data.

This corrects ADR 0004's implication that the inline layer has no direction-conditional
composition logic at all. It has three rules' worth, they are enumerated, and the
enumeration is machine-checked — which is a stronger statement than "there is no branch,"
because it is falsifiable.

What the gate cannot see is a branch that reads the direction indirectly, through a boolean
some other function derived from it. That residue is named rather than glossed, and it is
what the parity gate covers: an indirect branch changes an inline result and a case fails.
The two gates are complementary and neither is claimed to do the other's work.

The parity gate's own residue is named for the same reason. It compares inline results, so a
block-axis value assigned to an inline slot shows up, while an inline value assigned to a
block slot does not — and in vertical writing the block axis runs right to left, so that leak
renders a plausible mirrored page. The scalar allowlist above is what covers it, which is why
the two mechanisms are introduced together rather than one being presented as sufficient.

The parity gate runs from M1, long before vertical composition is a feature, so ADR 0004
stops being an aspiration checked at M5 and becomes a property proved on every commit.
