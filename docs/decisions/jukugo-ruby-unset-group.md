<!--
SPDX-FileCopyrightText: 2026 kumihan contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: what an occurrence with no declared group means for §C.2 note 8

- Applies to: `jlreq_line::feasible::same_run_refusal`
- Standing: `Unstated`
- JLReq: §C.2#8

## The silence

§C.2 note 8 states:

> A line break opportunity exists between two consecutive base characters belonging to
> different jukugo-ruby character complexes (cl-23). There is also a line break opportunity
> between two consecutive base characters belonging to the same jukugo-ruby character
> complex (cl-23) and between two runs of ruby text accompanying the corresponding base
> characters. However, a base character and the accompanying ruby text shall be
> indivisible, hence there is no line break opportunity between any two consecutive ruby
> characters in a run of ruby text accompanying a base character.

The note distinguishes three facts about the *text*: two base characters of different
complexes, two base characters of the same complex, and a base character together with its
own accompanying ruby. `jlreq_unit::Construct::group` is the level `same_run_refusal` reads
to tell the third case apart from the second once both sides already share one `RunId` —
but the note itself says nothing about an occurrence for which a caller's own `Runs` overlay
carries no `GroupId` at all. `Construct::new` does not require one (`jlreq_unit::Construct`'s
own doc: "no note anywhere needs a second level"), so the case is real and the note is silent
about it.

## The reading

**Absent a matching, declared group on both sides, `same_run_refusal` permits the break.**
It refuses only on positive evidence that two occurrences share one indivisible
base-and-ruby unit — equal, declared `GroupId`s — and treats `(None, None)` and the mixed
`(Some(_), None)` the identical way it treats two occurrences declared in different,
non-`None` groups: permitted, per the note's own second sentence.

## Why

Two facts, both from this workspace's own text rather than invented for this reading:

**`Construct::group`'s own doc names the level for exactly one purpose, and it is not the
base-to-base case.** "The group inside the run, where the specification needs one" is
introduced as answering "a break is allowed between two base characters of one jukugo-ruby
complex but not between the ruby characters attached to one base character" — the level
exists to separate ruby-to-ruby adjacency inside one base's own annotation from everything
else, not to gate the base-to-base permission the note's own second sentence already states
without qualification.

**`jlreq-line`'s own item stream cannot carry ruby text at all, so an occurrence reaching
this function under `ConstructKind::JukugoRuby` today is a base character.** A jukugo-ruby
run's accompanying reading is a nested `Segment` `jlreq_inline::Contribution` would place,
and the crate graph gives `jlreq-line` no edge to `jlreq-inline` (this crate's own module
doc; jukugo-ruby is M4-a). Every pair `same_run_refusal` can be asked about today is
consequently the base-to-base pairing the note's second sentence grants a break for
unconditionally on group, not the ruby-to-ruby pairing the third sentence forbids one for.

**Refusing on missing data would invent the prohibition rather than decline to.** Treating
`(None, None)` as "same" by the accident of `Option` equality would apply the group-level
indivisibility to the run-level case the note's own most explicit sentence permits — the
wrong direction for a library whose whole discipline is removing only an opportunity the
caller offered, never adding a prohibition the caller's own declaration does not support
(ADR-0003's spirit, read onto a construct declaration rather than onto a candidate).
Permitting instead of refusing is consequently not the safer of two guesses; it is the
reading the note's own words and this crate's own present reach both already point to.

## What would change it

A caller-declared role distinguishing a base occurrence from a ruby occurrence within one
`JukugoRuby` run — letting `same_run_refusal` tell the two apart without depending on group
at all — would settle this without an inference from silence. Absent that, a revision of
§C.2 note 8, or a JIS X 4051 commentary on it, conditioning the second sentence's own
permission on group membership rather than stating it without one would settle it from the
text directly. And once the crate graph gives `jlreq-line` an edge to `jlreq-inline` (M4-a)
and a real ruby text run can reach this function tagged `JukugoRuby` with no declared group,
evidence that such an occurrence genuinely needs the third sentence's own refusal rather
than the second sentence's own permission would be reason to invert this reading's own
default.

A conformance case now exercises this reading's own outcome:
`C.2/two-base-characters-in-one-jukugo-ruby-complex/break-permitted`
(`crates/jlreq-conform/cases/C.2.json`) declares two base characters of one jukugo-ruby
complex, both sides carrying no group exactly as `overlay_of` always builds them, and asserts
the break permitted — an implementation that inverted this reading, refusing whenever a
group is absent, would fail that case's own `permitted` entry. What the case cannot do, for
the reason `docs/conformance-deferrals.toml`'s own `C.2#8` entry now states: no case in this
suite can *declare* a `GroupId` at all (`crates/jlreq-conform/src/kumihan.rs`'s own
`overlay_of` never builds one — see its own doc), so the refusing half of this reading — two
occurrences with equal, declared groups — still has no published input to exercise it with.
The permissive half the new case does exercise is doubly grounded regardless of this
reading's own standing: §C.2#8's own second sentence grants a break between two base
characters of the same complex without conditioning it on group at all, so the new case's own
answer would hold even were this reading someday inverted for an occurrence genuinely without
one. The refusing half alone is covered by a unit test in `jlreq_line::feasible`'s own test
module —
`two_base_characters_of_one_jukugo_ruby_complex_with_no_declared_group_are_not_refused` —
which is a measurement of this workspace rather than a statement to another implementation.
