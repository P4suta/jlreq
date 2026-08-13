<!--
SPDX-FileCopyrightText: 2026 kumihan contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: what makes a cl-27 occurrence "a European numeral" in §C.2 note 11

- Applies to: `jlreq_spacing::evaluate::boundary` (§C.2 note 11's `is_european_numeral`)
- Standing: `Unstated`
- JLReq: §C.2#11, §A.19, §A.24, §A.27

## The silence

§C.2 note 11 states:

> A line break opportunity generally exists between preceding Western characters (cl-27)
> and trailing postfixed abbreviations (cl-13), unless the preceding Western character
> (cl-27) is used as a symbol of a quantity or a European numeral, in which case a line
> break is not allowed between them.

The note names two exceptions to the general permission, and only the first has a
declaration path anywhere in this workspace: "used as a symbol of a quantity" is exactly
the job `Role::QuantitySymbol` exists to let a caller state, its own doc already citing
this note (`crates/jlreq-unit/src/item.rs`). "Or a European numeral" has none. There is no
`Role::EuropeanNumeral`, and the note does not say whether an implementation is meant to
ask the caller (a declared fact, the way `Role::QuantitySymbol` is) or to read the
occurrence's own code point (a fact intrinsic to the character, needing no declaration at
all). Both readings are coherent with the sentence; the note does not choose between them.

## The reading

**Read directly from the occurrence's own Appendix A key, not from a declared role.** A
cl-27 item is one of the ten European numerals exactly when its key is one of U+0030
through U+0039 — the same closed set §A.19, §A.24 and §A.27 all enumerate under the name
"the ten European numerals" (`crates/jlreq-class/src/classify.rs`'s own `western_rule`
comment, which reads the identical set to decide cl-19 against cl-24 against cl-27 for
these same ten keys). `jlreq_spacing::evaluate`'s private `is_european_numeral` takes the
item's own [`jlreq_class::Member`] (via `jlreq_class::members` over the item's cluster) and
answers `true` when it is exactly one code point and that code point is an ASCII digit —
which is `U+0030`–`U+0039` and nothing else, since no other code point satisfies
`char::is_ascii_digit`. No case may declare "this occurrence is a European numeral" the way
one declares `"role": "quantity-symbol"`; whether it is one is decided from the key alone.

`docs/decisions/README.md`'s own rule is that the conformance suite carries every reading
here with *all* of its readings, not only the one this project takes. This reading has none
beside its own to carry: the alternative was a declared `Role::EuropeanNumeral` a caller
could set, considered below and rejected outright, so there is no live, policy-selectable
second answer for a case to publish, and `cases.schema.json`'s own `standing` rule
(`xtask/src/conform.rs`'s `check_standing`) requires an `unstated` or `adjudicated` case to
carry at least two `permitted` readings. The behavior is still checked — by a unit test in
`jlreq_spacing::evaluate`'s own test module rather than by a conformance case — which is the
scope limit `docs/conformance-deferrals.toml`'s own `C.2#11` entry states rather than hides.

## Why

Three reasons, none of them about taste.

**The set is already closed and already named, elsewhere in this crate graph, for the
identical ten keys.** `western_rule` reads "is this code point one of the ten European
numerals" to decide a *different* class question — cl-19 against cl-24 against cl-27 — for
the very same set §A.19, §A.24 and §A.27 enumerate together. Reading the identical fact
again here, for a different note, keeps one definition of "European numeral" in the
workspace rather than two that could drift apart.

**A digit is not a job a caller assigns; it is a fact about the code point.**
`Role::QuantitySymbol`, `Role::DecimalPoint`, `Role::DigitGroupSeparator` and
`Role::UnitSymbol` all exist because the *same* code point plays different jobs in
different documents — a comma may or may not be separating digit groups, and only the
caller's own document knows which. Whether `U+0035` is one of the ten European numerals is
never in question the same way: it either is `U+0035` or it is not, and asking a caller to
assert it would be asking them to restate something the code point already says, with a new
way to get it wrong (declaring the role on a letter, or omitting it on a digit) that buys
nothing the code point itself does not already answer.

**§3.2.6's own Note already treats this set as a code-point-level fact for a class
decision.** "Half- and fixed-width European numerals, when mixed with Japanese text, are
treated as members of the grouped numerals (cl-24) class" is the sentence that gives cl-24
its western-numeral membership at all, and it decides that membership from the code point
and the frame alone, with no role. Reading the same ten keys as "a European numeral" again
here, for a different note, is consistent with how the rest of this workspace already
treats them, rather than a new category invented for §C.2 note 11 alone.

The alternative — a `Role::EuropeanNumeral` a caller declares — was considered and
rejected. It would let a caller declare the role on a letter (contradicting the code point)
or omit it on a genuine digit (silently losing the refusal), neither of which is possible
under the code-point reading; and it would multiply the ways one ten-element set is
discovered in this codebase, for no fact the code point does not already carry.

## What would change it

A revision of §C.2 note 11 that defines "a European numeral" as something other than
§A.24's ten keys — full-width numerals, for instance, which §3.2.4 already puts in cl-19
rather than cl-27, so they cannot reach this note's cl-27 condition at all today — or a
revision that makes the exception depend on the caller's intent rather than the code point
(a document where a bare digit in cl-27 running text is not meant as a numeral for this
note's purposes) would need a declared role after all. Evidence that publishers apply the
refusal differently depending on context the code point alone cannot see would be recorded
as a `disagreements` entry on a conformance case for this note, once a declared role such as
the rejected `Role::EuropeanNumeral` gives a second, policy-selectable reading something to
publish — which is what that field is for — rather than changing this reading silently.
