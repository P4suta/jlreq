<!--
SPDX-FileCopyrightText: 2026 kumihan contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Reading: how §3.6.2's specified-character tab kind names its own occurrence

- Applies to: `jlreq_line::tab::TabKind::Character`
- Standing: `Unstated`
- JLReq: §3.6.2, ADR-0018

## The silence

§3.6.2 lists "Alignment with a specified character tab setting" as the fourth of its four
tab kinds:

> Alignment with a specified character tab setting: the start position of a specified
> character or sign (for example, a period) in the text is aligned to the tab position.
>
> 指定文字そろえタブ：タブ処理を行う対象の文字列の中にある指定された文字（例えばピリオド）
> の先頭をタブ位置に合わせて配置する．

That is the whole of what §3.6.2 says about this kind, and it leaves three questions
unanswered. First, whether "a specified character" is a declaration a caller states
directly — naming the occurrence itself — or a search kumihan performs over the run's own
text for a caller-named code point. Second, what happens when the named character occurs
**zero** times in the run the tab type governs. Third, what happens when it occurs **more
than once**. Nothing in §3.6.2, in §3.6.1's own general statement of tab setting's inputs,
or in §3.6.3's own placement algorithm answers either of the second or third questions, and
the worked example the sentence gives ("for example, a period") illustrates the kind rather
than closing the question of what counts as "specified" or how many times it may appear.

## The reading

**`TabKind::Character` names the occurrence directly, by [`jlreq_unit::ItemIndex`], not by
a bare `char` kumihan searches the run's own text for.** The variant is `Character { at:
ItemIndex }`: `at` is an ordinal into the same running-text stream the governing
[`jlreq_line::tab::tab_line`] call's own `text` argument indexes — the same index space
every other item-addressing type in this workspace uses (ADR-0018) — not a byte offset and
not a code point.

Because the caller names the occurrence rather than kumihan discovering it by searching,
the second and third questions are not separately decidable inside this crate at all, and
this reading does not answer either of them by inventing a rule — it removes the occasion
for either to arise. "Zero occurrences" cannot happen: `tab_line` validates that `at` names
an item inside the run's own bounds and refuses the call
(`jlreq_line::ComposeError::OutOfRange`) when it does not, so a well-formed call always
names an item that exists. "More than once" is moot: the caller names exactly one
occurrence directly, so there is never a set of candidate occurrences for an algorithm to
choose among in the first place.

## Why

Three reasons, none of them about taste.

**ADR-0018's occurrence model is what an item already is.** "One item is one occurrence: a
code point together with the character frame the caller's advance covers and the role it
plays" is this workspace's own governing decision for how a character-identity question is
answered everywhere else in the crate graph — never by re-deriving a fact from a bare code
point when the caller's own item stream already carries it. §3.6.2's own worked example ("a
period") is illustrative of what a caller might specify, not a statement that a bare code
point is the only vocabulary the specification permits for naming it.

**Appendix A's twenty-five code-point-*sequence* keys cannot be named by a bare `char` at
all.** `jlreq_unit::Item`'s own doc records this directly: several of Appendix A's keys are
ordered pairs of code points (`<02E5, 02E9>`, a cl-27 falling tone contour, is one), and a
caller who wanted to specify one of those as the "specified character" for §3.6.2's own
purposes could not express it as a single `char` no matter how the search were written. An
`ItemIndex` names the occurrence regardless of how many code points it spans, because it
names the *item*, not the code point the item happens to carry.

**Naming the occurrence removes an invented rule rather than replacing it with a different
one.** A search-based reading (a `char` and a scan of the run's own text) would still have
to answer the zero- and several-occurrence questions somehow — "refuse the call", "align to
nothing", "align to the first occurrence", "align to the last" are all coherent answers
JLReq's own text does not choose among — and every one of them is an invention this reading
does not need, because the caller who placed a tab sign before the target string in the
first place (§3.6.1) already knows exactly which occurrence they mean. Asking them to name
it directly asks for nothing they do not already have to know to have written the
declaration at all.

## What would change it

A revision of §3.6.2 that states a deterministic answer to the zero- or
several-occurrence question — "align to the first occurrence of the specified character",
for instance — would be evidence for offering a **second**, search-based constructor
alongside `TabKind::Character { at: ItemIndex }`: one that takes a `char` (or, faithful to
Appendix A's own multi-code-point keys, a `&[char]`) and a byte range to search, and applies
whatever rule the revision states. That second constructor is not this reading's own
rejection so much as its complement — the occurrence-naming reading answers "which one",
and a revision stating a deterministic search rule would answer the question this reading
declines to invent an answer for, rather than contradicting it. Evidence that publishers
disagree about which occurrence "the specified character" means in practice — one document
meaning the first, another the last, a third every occurrence set independently — would be
recorded as a `disagreements` entry on a conformance case for this kind, once a
search-based second reading exists to publish as the alternative.
