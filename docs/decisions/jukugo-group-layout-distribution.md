<!--
SPDX-FileCopyrightText: 2026 kumihan contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: which of §3.3.6's own two methods §3.3.7¶2's own `group` answer means

- Applies to: `jlreq_inline::place` (`place_jukugo_compound`'s own call into
  `place_group_run`, `Question::JUKUGO_RUBY_LAYOUT`).
- Standing: `Unstated`
- JLReq: §3.3.7

## The silence

§3.3.7's second paragraph states, for a kanji compound word in which some base character
needs three or more ruby characters:

> The available methods include the layout as specified in JIS X 4051, which is similar to
> the group-ruby method described in § 3.3.6 Positioning of Group-ruby with Respect to Base
> Characters (see Figure 130), and layout decided by the phonetic structure of the kanji
> compound word and the type of script of the adjacent characters (see Figure 131).

`Question::JUKUGO_RUBY_LAYOUT`'s own two answers are `group` (this sentence's first clause)
and `phonetic` (its second). This round implements `group`, and the sentence's own words
leave open exactly which of §3.3.6's own two distribution methods it names: §3.3.6 itself
states two, `Question::GROUP_RUBY_DISTRIBUTION`'s own `jis` and `flush`, for the identical
ruby-not-longer-than-base case this jukugo method is compared to. The sentence names "the
layout as specified in JIS X 4051," which sounds like one specific method rather than a
choice between two, and then says that layout "is similar to" §3.3.6's own group-ruby
method — a comparison, not a citation of §3.3.6's own name for either method. JLReq does not
say outright whether a jukugo compound's own `group` answer should read the document's
`Question::GROUP_RUBY_DISTRIBUTION` answer the way a genuine group-ruby construct does, or
whether it is pinned to `jis` regardless of what that question says.

## The reading

**§3.3.7¶2's own `group` answer forces `jis`, regardless of the document's own
`Question::GROUP_RUBY_DISTRIBUTION` answer.** `place_jukugo_compound` hands a compound-wide
synthetic `RubyRun` to `place_group_run` with its `jis` parameter fixed to `true`, never
reading `Question::GROUP_RUBY_DISTRIBUTION` for a jukugo construct at all. A policy that
answers `flush` for that question still yields `jis`'s own five-site-weighted geometry for a
jukugo compound under paragraph 2's `group` answer.

## Why

§3.3.6 itself names exactly one of its own two methods "the method specified in JIS X 4051,"
and does so twice, once for each direction of the length mismatch it discusses. The
shorter-than-base paragraph:

> To be more specific, where 2 units of inter-character spacing are used between ruby
> characters, add 1 unit of spacing between the start of the base text and the start of the
> ruby text, and between the end of the ruby text and the end of the base text. This will
> give a balanced appearance, and is the method specified in JIS X 4051 (see Figure 124).
> Another way is to first align the leading characters for both the base text and ruby text
> and the ends of both trailing characters, and then add the same amount of inter-character
> spacing between the rest of the ruby characters (see Figure 125).

The first sentence — `jis`'s own `[1, 2, 2, …, 2, 1]` distribution — is the one method §3.3.6
itself names "the method specified in JIS X 4051." The second — `flush` — carries no such
attribution anywhere in the section; §3.3.6's own longer-than-base paragraph repeats the
identical pattern, again naming only its own JIS-X-4051-labeled method by that name and
leaving its own "another way" unlabeled. §3.3.7¶2's own sentence names "the layout as
specified in JIS X 4051" using the identical phrase §3.3.6 uses for exactly one of its own
two methods, `jis`, and none of the words for the other. Read against §3.3.6's own text, "the
layout as specified in JIS X 4051" is consequently a citation of one specific, already-named
method, not a placeholder for "whichever of §3.3.6's two methods the document happens to
prefer."

The sentence's own second clause — "which is similar to the group-ruby method described in
§ 3.3.6" — is a comparison to the *section*, offered to orient a reader already told which
specific method is meant, not a second, independent instruction to import the section
whole. "Similar to" is a hedge introducing a resemblance, not a citation incorporating an
alternative the first clause never named; reading it as silently reopening the choice §3.3.6
itself already closed by name would make the sentence's own two clauses redundant with each
other in the wrong direction — the first clause would be doing no narrowing work at all if
the second reopened everything it had just settled.

The alternative reading — that "similar to the group-ruby method" imports §3.3.6 in full,
`flush` included, and a jukugo compound should therefore track whatever
`Question::GROUP_RUBY_DISTRIBUTION` says — was considered and rejected. It reads the
sentence's second clause as controlling and its first as merely introductory, the reverse of
what the first clause's own specific, named citation supports; and it would mean
§3.3.7¶2's own `group` answer sometimes produces `jis`'s own geometry and sometimes `flush`'s
depending on an answer to a *different* section's question, `Question::GROUP_RUBY_DISTRIBUTION`,
which §3.3.7 itself never mentions and which this jukugo method's own name gives no textual
hook to read at all.

## What would change it

A revision of JLReq, or a JIS X 4051 commentary, that states §3.3.7¶2's own `group` answer
tracks whatever method a document has chosen for ordinary group-ruby — naming
`Question::GROUP_RUBY_DISTRIBUTION` or its JIS X 4051 equivalent directly, rather than citing
one specific method by the identical name §3.3.6 already uses for it alone — would settle
the question outright and this reading would be revisited to match it.

This is inline-axis geometry throughout, and direction-independent: nothing in
`place_jukugo_compound` or `place_group_run` reads writing mode, the identical fact
`docs/decisions/group-ruby-flush-single-character.md`'s own closing section already states
for a neighboring reading. No `docs/direction-sites.toml` entry is expected for this reading
or for the round that implements it.

Task #90, the independently authored conformance phase for this round's jukugo-ruby
placement (ADR-0006), has since run and published exactly the case this section promised:
`crates/jlreq-conform/cases/3.3.7.json`'s own `3.3.7/jukugo-ruby-placement/paragraph-two-
whole-compound-attachment` carries a third `permitted` entry naming both `ruby.jukugo_layout:
group` and `ruby.group_distribution: flush`, asserting the *identical* `jis` geometry its own
first entry (naming neither) already asserts — this reading's own forcing, published as a
named, explicit contradiction of the expectation a reader would otherwise reasonably form:
this fixture's own surplus is genuinely non-zero, the identical ratio
`3.3.6/group-ruby-placement/jis-versus-flush-distribution`'s own case already shows `jis` and
`flush` visibly diverging at for an *ordinary* group-ruby run, so declaring `flush` here looks
like it should move the offsets. `crates/jlreq-conform/tests/suite.rs`'s own
`section_3_3_7_is_also_measured_under_flush` runs this file's cases a second time against a
`Kumihan::new(Policy)` declaring `ruby.group_distribution: flush`, and its own green result
checks the forcing's own numbers against this implementation's real output — but the file's
own first entry, matching every policy, would already have asserted the identical numbers
even had the third never been authored, so this run's own selection of the third entry over
the first is not itself what a green result proves, unlike the shape
`docs/decisions/group-ruby-flush-single-character.md`'s own closing section describes for its
own sibling reading, where the two candidate entries genuinely disagree and only one green
outcome is possible. What the third entry adds beyond what the first alone already checks is
publication: a reader of this case file sees the forcing named against the one axis they
would otherwise expect it to answer to, rather than having to notice that the first entry's
own silence on that axis already implies it. `crates/jlreq-inline/src/place.rs`'s own unit
test `jukugo_paragraph_two_group_answer_forces_jis_regardless_of_group_ruby_distribution`
continues to measure it directly too, over both of `Question::GROUP_RUBY_DISTRIBUTION`'s own
answers, on the identical fixture.
