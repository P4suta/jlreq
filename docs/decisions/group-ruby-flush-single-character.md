<!--
SPDX-FileCopyrightText: 2026 kumihan contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: what `flush` does for a group-ruby run of exactly one ruby character

- Applies to: the private ruby placement round in
  [`pipeline`](../../crates/kumihan/src/pipeline.rs), selected by the typed
  `GroupRubyDistribution::Flush` style setting.
- Standing: `Unstated`
- JLReq: §3.3.6

## The silence

§3.3.6's second paragraph states two methods for a group-ruby run whose reading is shorter
than its base. The second — `Question::GROUP_RUBY_DISTRIBUTION`'s `flush` answer — reads:

> Another way is to first align the leading characters for both the base text and ruby text
> and the ends of both trailing characters, and then add the same amount of inter-character
> spacing between the rest of the ruby characters.

That sentence names three things in order: a leading character to align, a trailing
character to align, and "the rest" to space out evenly between them. It says nothing about
what happens when the run has exactly one ruby character, so that one character is
simultaneously the leading character the sentence aligns to the base's own start and the
trailing character it aligns to the base's own end — and ADR-0002 forbids moving a caller-
supplied advance to satisfy two different placements at once. There is also no "rest" left
to space: with one character there are zero interior gaps, so the sentence's own final
clause has nothing to distribute the base's surplus into. JLReq does not say which half of
the sentence controls when the two are jointly unsatisfiable, or what a reader should do
with the surplus that neither alignment nor an interior gap can hold.

## The reading

**`flush` places a group-ruby run of exactly one ruby character at its base's own start,
with the entire surplus left unapplied.** `place_ruby_span` computes this by construction
rather than by a special case: `flush`'s own leading offset is `InlineExtent::ZERO` always,
never derived from `distribute`, and its own interior weights are built over the run's
`count - 1` interior sites. At `count == 1` that is zero sites, so
the `flush` weight vector in
[`pipeline.rs`](../../crates/kumihan/src/pipeline.rs) is empty and `proportional_shares`
yields no shares at all. The run's one character is therefore placed exactly at the base's
own start with nothing consumed from the surplus in either direction.

This is inline-axis geometry throughout, and direction-independent: nothing above reads
writing mode, and the logical placement calculation branches on no writing mode. Physical
horizontal/vertical mapping happens only after this position has been fixed.

## Why

This reading honors the sentence's own leading clause — "first align the leading
characters... for both the base text and ruby text" — literally, and treats the trailing
half and the interior-spacing half as inapplicable rather than overridden: there is no
second character to be the trailing one, and no interior gap to receive the spacing the
final clause describes. The alternative — falling back to `jis`'s own centering, so that a
single-character `flush` run agrees with a single-character `jis` run — is arguable from the
same silence and was considered and rejected. Two things count against it. First, `jis` and
`flush` already agree at `count == 1` in the one case §3.3.6 itself resolves without
ambiguity: `n == 1` is also `n == 2`'s neighbor at the ratio §3.3.6 paragraph 1 governs
(surplus `== InlineExtent::ZERO`, base and reading equal in length), where both methods
place the run flush with the base's own start by construction, not by a special reading of
either method's own text (`jlreq_inline::place`'s own module doc, "What this round
implements"). Manufacturing a *second*, surplus-dependent agreement between the two methods
at `count == 1` would make `flush` indistinguishable from `jis` at exactly the ratio where
the two are supposed to diverge — the whole reason JLReq states two methods rather than one
— which is a worse reading of a section whose own point is that the two are different.
Second, `jis`'s own centering reads the sentence's *other* method, not this one: falling
back to it silently would mean `flush`'s own implementation sometimes runs `jis`'s
arithmetic under `flush`'s own name, which is the kind of unstated substitution
`docs/conformance-deferrals.toml`'s own discipline exists to name rather than hide. Placing
the run at the base's own start, with the surplus applied nowhere, is what the sentence's own
words most directly support once its trailing and interior clauses have nothing to act on.

## What would change it

A revision of JLReq, or a JIS X 4051 commentary, that states what a single-character
`flush` run does with the base's own surplus — split it evenly on both sides after all,
push it entirely to one side, or confirm that it is left unapplied, as this reading
holds — would settle the question outright and this reading would be revisited to match it.

Protocol-v1 case
`3.3.6/group-ruby-single-character-flush-start-aligned` in the bundled
[`suite.ndjson`](../../crates/kumihan-conformance/suite.ndjson) fixes the observable result;
the public Rust tests independently compare the `jis` and `flush` positions.
