// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Placement: [`Attachment`], [`Attachments`] and [`place`] — where a mono-ruby or
//! group-ruby run's own characters actually go against the base characters [`crate::lower`]
//! attached them to.
//!
//! # §0: which reading of §3.3.5's lead sentence this module takes
//!
//! §3.3.5's English rendering reads its centering clause as scoped to the
//! Western/European-numeral branch alone ("If mono-ruby characters have their own character
//! widths... they are set according to their own widths **and then** the ruby text is
//! placed so that its center matches that of its base character"); the Japanese rendering's
//! own「そのうえで…原則である」reads as a second sentence governing both branches. This
//! module does not need to choose, because under ADR-0002 kumihan never asks how wide a
//! character is — the caller supplied every advance. "Set solid" (mono-ruby whose
//! characters are Japanese) and "set according to their own widths" (mono-ruby whose
//! characters have their own widths) name the *same* operation once the widths are already
//! given: lay the run out at the supplied advances of [`crate::Ruby`]'s annotation items
//! with zero inter-character space, which is what [`place_solid_run`] does regardless of
//! which characters the caller's reading happens to contain. There is no frame or class
//! branch here and none should be added; nothing this module reads distinguishes a
//! Japanese ruby character from a Western one, and the arithmetic below is unchanged either
//! way. This divergence is therefore stated here rather than in `spec/derived/defects.tsv`,
//! because it is not load-bearing: both readings of the lead sentence produce the identical
//! call to [`place_solid_run`].
//!
//! # What this round implements, of §3.3.5's four cases
//!
//! §3.3.5 states four positioning cases in its own order, and three of the four are real
//! geometry here:
//!
//! - **(b) one ruby character on one base, nakatsuki (中付き).** Centered: the difference
//!   between the base's own supplied advance and the run's own summed advance is split
//!   evenly between the run's leading and trailing sides
//!   (`docs/decisions/mono-ruby-separation-split.md`, whose own "Applies to" line now names
//!   this module alongside [`crate::lower`] as a second reader of the identical question —
//!   see that file for why the centering remainder and the §3.3.8 rule 1 surplus split are
//!   one question, not two).
//! - **(b) one ruby character on one base, katatsuki (肩付き), run not longer than the
//!   base.** Inline-start aligned: the run begins exactly at the base item's own placement,
//!   with no distribution at all.
//! - **(c) three or more ruby characters on one base, nakatsuki adopted.** The identical
//!   centering computation as (b)'s nakatsuki case, including when the run is genuinely
//!   longer than the base — the difference is then negative, both shares are negative, and
//!   the run starts before its base and ends after it. §3.3.5(c) states only the centering
//!   method for nakatsuki, with no alternative, so this case never declines.
//!
//! §3.3.5's own case (a) — two hiragana ruby characters exactly filling one base character —
//! is deliberately not a fifth branch. At that exact ratio the centering difference is zero
//! and the run's own extent equals the base's, so nakatsuki's centering and katatsuki's
//! start-alignment agree on every character's own offset without either branch asking about
//! character count; a unit test below demonstrates that agreement falling out rather than
//! being special-cased.
//!
//! §3.3.5(b)'s and (c)'s own text distinguishes "one character" from "three or more" only
//! because ordinary mono-ruby proportions never let a one- or two-character run outrun a
//! full-size base. The geometry itself does not read a character count anywhere above: what
//! decides start-alignment from decline under katatsuki, and what decides a positive from a
//! negative centering share under nakatsuki, is a comparison between two already-measured
//! extents. §3.3.5(c)'s own two-branch structure is what that comparison reduces to once
//! stated in extents rather than in the character counts that usually produce them, and (a)
//! and (b) are the same comparison at the ratios where it never trips.
//!
//! # What this round declines: §3.3.5(c)'s katatsuki-with-overflow choice
//!
//! **(c) three or more ruby characters on one base, katatsuki adopted, run longer than the
//! base.** §3.3.5(c) states two methods for this case in so many words — center the run
//! after all (identical to the nakatsuki branch), or decide which of the two adjacent
//! characters the overhang falls on, "depending on the type of script of the adjacent
//! characters... and the number of ruby characters" — which is `permission = "stated"` in
//! `spec/derived/questions.tsv`'s own vocabulary, not JLReq being silent. What is missing is
//! not the specification's answer but a policy `Question` to read it through:
//! `spec/derived/questions.tsv` carries `ruby.alignment`, `ruby.group_distribution`,
//! `ruby.jukugo_layout`, `ruby.overhang_kana` and `ruby.overhang_indent` for §3.3.5's
//! neighborhood, and none of the five is this choice. [`place`] does not pick between the
//! two methods on its own authority: it emits no [`Attachment`] for such a run and reports
//! it through [`Attachments::declined`] instead. Giving this choice a `Question` — a
//! hand-written row in `xtask/src/policy.rs` quoting §3.3.5(c)'s own `en` rendering, a
//! regenerated policy space, and refreshed `derive`/`generate`/`attest --digests`
//! artifacts — is task #81, deliberately a round of its own; this module does no part of
//! it.
//!
//! # What this round implements: §3.3.6 paragraphs 1 and 2 (group-ruby, ruby not longer than base)
//!
//! [`RubyStyle::GroupRuby`] gains real geometry this round, for the half of §3.3.6 whose
//! ruby is not longer than its base. §3.3.6's own first paragraph states the equal-length
//! case ("each text is set solid and the center of both texts are aligned with each other")
//! and its second states the shorter-ruby case in two methods,
//! `Question::GROUP_RUBY_DISTRIBUTION`'s two answers: `jis`, "add 1 unit of spacing between
//! the start of the base text and the start of the ruby text, and between the end of the
//! ruby text and the end of the base text" where "2 units of inter-character spacing are
//! used between ruby characters" — a proportional split over `n + 1` sites with weights
//! `[1, 2, 2, …, 2, 1]`, summing to `2n`, taken through [`distribute`]
//! ([`group_jis_weights`]); and `flush`, "first align the leading characters for both the
//! base text and ruby text and the ends of both trailing characters, and then add the same
//! amount of inter-character spacing between the rest of the ruby characters" — a fixed
//! [`InlineExtent::ZERO`] leading offset and an equal split over the `n - 1` *interior*
//! sites only ([`group_flush_weights`]).
//!
//! Both methods read the base run's own extent from `placements`
//! (`extent_between(placements[first], placements[last] + text.items()[last].advance())`),
//! never from a re-derived sum of `ctx.text`'s own item advances: §3.3.6's own parenthetical
//! formula — "number of characters * advance-width of each character" — names the identical
//! number whenever the base run is set solid, which paragraphs 1 and 2 both require ("each
//! text is set solid", "set the base text solid"), and the placements-derived span is also
//! correct when composition genuinely put something between two of the run's own base items
//! (a forced mono-ruby separation elsewhere on the line, for instance), where the two
//! numbers would otherwise silently disagree.
//!
//! §3.3.6's own first paragraph — the equal-length case — is, as in §3.3.5(a), not a third
//! branch: at `surplus == InlineExtent::ZERO` every weight's own share is zero under both
//! methods, the run starts exactly at the base run's own start, and the two texts' centers
//! coincide, which *is* paragraph 1's own rule. A unit test below demonstrates the two
//! methods agreeing there, the identical shape this module's own §3.3.5(a) test already
//! uses. At `n == 1`, `jis`'s own weights degenerate to `[1, 1]` — centered, coinciding with
//! paragraph 1's rule at the ratio where a single ruby character exactly fills its base —
//! and `flush`'s own weights are empty, which is
//! `docs/decisions/group-ruby-flush-single-character.md`'s own published reading of a case
//! §3.3.6's second method does not itself resolve (a single ruby character has no "rest of
//! the ruby characters" to space, and it is simultaneously the leading and the trailing
//! character the sentence's own two alignment clauses name, which ADR-0002 forbids moving
//! twice at once to satisfy).
//!
//! An unrecognized `Question::GROUP_RUBY_DISTRIBUTION` answer name falls to `jis` rather
//! than panicking: every one of `Policy`'s five presets answers `jis`
//! (`spec/derived/questions.tsv`'s own `ruby.group_distribution` row), so `jis` is this
//! round's own honest default for an input the workspace's closed-choice generation should
//! never actually produce.
//!
//! # What this round declines: §3.3.6 paragraph 3 (group-ruby, ruby longer than base)
//!
//! §3.3.6's own third paragraph states two methods for a ruby genuinely longer than its
//! base, and both work by spreading the *base* characters apart, never by moving the ruby:
//! "add a certain amount of inter-character spacing between each adjacent base character,"
//! stated once for the JIS X 4051 method and again, differently distributed, for its own
//! alternative. [`place`] structurally cannot do that: `placements` is
//! `jlreq_line::Line::placements`, an answer already fixed before [`place`] is ever called,
//! and [`place`] emits [`Attachment`]s for annotation items only — there is no channel back
//! to widen a base item's own placement from here. The correct home for this half is
//! `crate::lower::lower_group` emitting forced [`jlreq_unit::Separation`]s before
//! composition ever sees the base run, exactly the mono-ruby analogue
//! `crate::lower::collect_mono_separation` already performs for §3.3.8 rule 1's own
//! narrower prohibition — a future round's work, not this one's.
//!
//! So: a [`RubyStyle::GroupRuby`] run whose ruby is genuinely longer than its base — the
//! identical extent comparison katatsuki's own mono-ruby decline already makes,
//! `base_extent.max(ruby_extent) != base_extent` — produces no [`Attachment`] and is
//! reported through [`Attachments::declined`] instead. A silent drop is forbidden here for
//! the identical reason it is forbidden for §3.3.5(c)'s own unresolved choice above.
//!
//! I read §3.3.8 in full for the physical positioning of a longer ruby text against its
//! *neighbors* — cl-19 never (rule 1), kana and half-em spaces and inseparable characters
//! and brackets up to named limits (rules 2 through 6) — and none of its own six rules
//! states a group-ruby run's *own* offset against its own base; every one governs how far a
//! ruby may hang over an *adjacent* character once the run itself is already positioned.
//! §3.3.6 alone states that positioning, and paragraph 3 is what this section declines.
//!
//! # What this round implements: §3.3.7 (jukugo-ruby)
//!
//! §3.3.7 states two paragraphs, chosen per compound by a count that does *not* reduce to
//! an extent comparison the way §3.3.5(a)-through-(c)'s own three cases do (this module's
//! own doc, above, "What this round implements, of §3.3.5's four cases"). Its own words:
//! "If the number of ruby characters are two or less for each ideographic characters
//! (cl-19) which participates in a kanji compound word... then for each run of ruby text
//! associated with each base character, compose ruby characters as described in § 3.3.5"
//! (paragraph 1); "If there is any ideographic character (cl-19) in a given kanji compound
//! word which needs three or more ruby characters, the jukugo-ruby layout cannot be used.
//! In this case, attach the ruby text to the kanji compound word as a whole" (paragraph 2).
//! [`place_jukugo`] reads that count directly off each declared [`RubyRun`]'s own annotation
//! width — `run.annotation().end - run.annotation().start`, per base character — rather
//! than comparing extents: paragraph 2's own condition is genuinely "needs three or more
//! ruby characters," not "is longer than its base," so a base character measured wide
//! enough could carry three narrow ruby characters without ever outrunning it and paragraph
//! 2 still applies. §3.3.5(a)-through-(c)'s own reduction to an extent comparison, argued at
//! length above, does not transfer here, and this paragraph states why rather than leaving
//! a reader to expect the identical argument and not find it.
//!
//! **Paragraph 1** delegates to [`place_mono_run`] unmodified, once per declared
//! [`RubyRun`] — the identical function [`RubyStyle::MonoRuby`] itself calls, reading the
//! identical [`crate::lower::Contribution::alignment_of`] resolution ([`crate::lower`]'s own
//! module doc states the hoist that makes this resolution exist for a jukugo construct at
//! all) and applying the identical nakatsuki/katatsuki geometry, decline included: a jukugo
//! run whose own ≤2-character reading still overflows its own base character under
//! katatsuki declines exactly as an ordinary mono-ruby run does, because paragraph 1's own
//! delegation is to §3.3.5's *method*, not to its arithmetic in isolation. Splitting the
//! compound across two lines needs nothing special here either: each [`place`] call places
//! only the base characters its own `items` covers, and [`place_mono_run`]'s own single-item
//! skip is precisely the indexing convention already documented below, working exactly as
//! designed for a construct whose own runs never share one base item.
//!
//! **Paragraph 2** attaches the whole compound's own reading as one unit, the way
//! [`RubyStyle::GroupRuby`] does, and [`place_jukugo_compound`] computes it by handing one
//! compound-wide synthetic [`RubyRun`] — the whole declared base range, read against the
//! first declared run's own annotation start through the last's own end, which
//! [`Ruby::new`]'s own `check_runs` validation guarantees spans the compound's whole reading
//! contiguously — to [`place_group_run`], forced to `jis` regardless of the document's own
//! `Question::GROUP_RUBY_DISTRIBUTION` answer. That forcing is `decision:jukugo-group-
//! layout-distribution`'s own published reading of a genuinely unstated question — see that
//! file for the argument in full — and it is what makes `Question::JUKUGO_RUBY_LAYOUT`'s
//! own `group` answer observable at all: a policy answering `flush` for
//! `Question::GROUP_RUBY_DISTRIBUTION` still yields `jis`'s own geometry for a jukugo
//! compound, a divergence a future case can measure directly. Reusing [`place_group_run`]
//! means reusing its own ruby-longer-than-base decline too (§3.3.6 paragraph 3's own
//! base-spreading half, which this crate cannot perform for the identical reason stated
//! above for group-ruby itself) — a jukugo compound is not exempt from that blocker merely
//! for being a different [`RubyStyle`], because it is routed through the identical
//! arithmetic once paragraph 2's own `group` answer is chosen.
//!
//! `Question::JUKUGO_RUBY_LAYOUT`'s own `phonetic` answer declines outright instead, every
//! time it is reached: §F's own phonetic-structure distribution is not implemented this
//! round (the next section states the scope in full), so [`place_jukugo_compound`] reports
//! it through [`Attachments::declined`] rather than guessing at a geometry no code here
//! computes. An unrecognized `Question::JUKUGO_RUBY_LAYOUT` answer name falls to `group`
//! rather than panicking, [`place_group_run`]'s own precedent for
//! `Question::GROUP_RUBY_DISTRIBUTION`: every one of `Policy`'s five presets answers `group`
//! (`spec/derived/questions.tsv`'s own `ruby.jukugo_layout` row), so `group` is this round's
//! own honest default for an input the workspace's closed-choice generation should never
//! actually produce.
//!
//! Paragraph 2's own base range can straddle one [`place`] call's own `items` in a way
//! [`RubyStyle::GroupRuby`]'s own base range structurally cannot — this module's own doc,
//! "Indexing convention," states why group-ruby's own straddle never happens in practice and
//! jukugo's own genuinely can, and states [`place_jukugo_compound`]'s own answer to it in
//! full.
//!
//! # What this round declines: §3.3.7 paragraph 2's own `phonetic` answer, and §F entire
//!
//! §F ("Positioning of Jukugo-ruby") is the detail paragraph 2's own third sentence promises
//! for the phonetic-structure method — "layout decided by the phonetic structure of the
//! kanji compound word and the type of script of the adjacent characters" — and none of it is
//! implemented this round: not its distribution principles (§F.1, §F.2), not its behavior
//! under line-adjustment expansion (§F.3, §F.4), and not the overhang ceiling paragraph 2's
//! own fourth sentence states, in the identical two-threshold shape §3.3.6's own Note already
//! declines above ("a full character width (or one and a half times the full-width) of a
//! ruby character") — doubly moot here, because that sentence governs the phonetic method
//! alone, which this round declines wholesale rather than reading any part of. Every
//! `Question::JUKUGO_RUBY_LAYOUT` answer of `phonetic` consequently declines the compound it
//! names, unconditionally, through [`Attachments::declined`] — never a silent drop, for the
//! identical reason a silent drop is forbidden everywhere else in this module.
//!
//! # `Attachments::declined` now reports four reasons, not two
//!
//! [`Attachments::declined`] is no longer reserved for §3.3.5(c)'s own choice and §3.3.6
//! paragraph 3's own choice alone: this round adds two further reasons, both
//! [`RubyStyle::JukugoRuby`]'s own — the `phonetic` decline just stated, and the
//! straddled-compound decline "Indexing convention" states below — while the two inherited
//! reasons both widen their own reach rather than staying mono-ruby's and group-ruby's own
//! private property. §3.3.5(c)'s own katatsuki-with-overflow choice now also catches a
//! jukugo paragraph-1 run, because [`place_jukugo`] routes it through the identical
//! [`place_mono_run`]; §3.3.6 paragraph 3's own base-spreading blocker now also catches a
//! jukugo paragraph-2 compound answering `group`, because [`place_jukugo_compound`] routes
//! it through the identical [`place_group_run`]. [`Attachments::declined`]'s own doc states
//! all four in full; the rule that holds across all of them is the one this module has
//! argued from the start — `declined` names constructs this call deliberately did not
//! place, each reason stated in this module's own doc, never a construct this call simply
//! has no code for yet.
//!
//! # §3.3.6's Note is a declared slot, not implemented
//!
//! §3.3.6's second paragraph carries a Note: "When the length of the ruby text is far
//! shorter than that of the base text, the method specified in JIS X 4051 could result in
//! spacing twice the size of a ruby character for the leading and the trailing spacing,
//! which might give a misleading appearance. Therefore, a criterion for deciding whether or
//! not to adopt the method of JIS X 4051 is to see if the amount of the leading and the
//! trailing spacing exceeds the full-width size (or up to 1.5 times the size) of a ruby
//! character." That parenthesis states *two* thresholds — a full ruby em, or "up to" one and
//! a half — not one, so wiring it here would invent a number JLReq does not itself choose
//! between. This round does not implement it, and does not silently omit it either: it is a
//! slot, named here rather than left for a reader to discover as a silent gap in `jis`'s own
//! reach. Closing it would need either a policy `Question` of its own, with the two
//! thresholds as its answers, or a `docs/decisions/` reading picking one — neither exists
//! yet, and [`place_group_run`] applies `jis` regardless of how far short the ruby falls,
//! exactly as `Question::GROUP_RUBY_DISTRIBUTION` states it without this further cap.
//!
//! # The omitted `overhang` parameter
//!
//! `docs/design/api-spine.md`'s own sketch of this function takes an
//! `overhang: &[RubyOverhang]` parameter. This round omits it, a deliberate deviation from
//! that sketch (which this module doc, and the sketch itself, now both state, so the two do
//! not silently disagree). Nothing this round's [`place`] does reads a per-boundary
//! overhang allowance: the one positioning rule that would consult one is exactly
//! §3.3.5(c)'s katatsuki-with-overflow choice this module declines, and §3.3.8 rule 1's own
//! forced separation is already [`crate::lower`]'s, computed before this function ever
//! runs. A parameter accepted and never read is the silent defect [`crate::Constructs`]'s
//! own doc already refuses to publish for the eight unbuilt constructs; the identical
//! argument applies to an unread parameter of a function that does exist. Task #81 is also
//! where a genuine consumer would first appear — reading `overhang` to choose between
//! §3.3.5(c)(ii)'s two hangover methods — and the parameter returns then, not before.
//!
//! # Two `Lowered` buffers, not one
//!
//! [`place`] takes `contribution: &Contribution<'_>`, produced by an earlier
//! [`crate::lower`] call against some [`Lowered`], and separately `out: &mut Lowered` to
//! write its own answer into. These are almost always two different [`Lowered`] instances:
//! `contribution` borrows from whichever one [`crate::lower`] wrote, and the borrow checker
//! refuses a second mutable borrow of that same instance while the first is still alive, so
//! a caller cannot pass the same buffer for both in one call. A caller composing many
//! paragraphs therefore typically keeps two persistent [`Lowered`] buffers — one
//! [`crate::lower`] writes into, one [`place`] does — rather than one, which is a real
//! call-site consequence of this signature and not a defect in it.
//!
//! # Indexing convention
//!
//! `placements` is indexed exactly as `jlreq_line::Line::placements` documents its own
//! return: one entry per item of `items`, in order, so `placements[k]` is the item at
//! `items.start + k`, never the item at absolute ordinal `k`. A [`crate::Ruby`]'s own base
//! item, read from [`crate::Constructs`], is an absolute [`jlreq_unit::ItemIndex`] and is
//! translated through `items.start` before it ever indexes `placements`. A base item lying
//! outside `items` — on a different composed line, or split away from its own run by a
//! break — is skipped silently rather than reported: a caller composing line by line calls
//! [`place`] once per line, over that line's own `items`, and a construct outside the
//! current line's range is exactly the run a *different* call already placed, or will.
//!
//! That convention bounds-checks *one item* — a mono-ruby run's single base item — against
//! `items`, and [`place_mono_run`]'s own skip is correct for exactly that reason: a
//! different `place` call, over a different composed line, already placed the base item this
//! one's `items` excludes, or will. A [`RubyStyle::GroupRuby`] run's own base is a *range*,
//! potentially spanning several items, and the convention does not extend to it unexamined:
//! a range that straddles `items`' own boundary is not necessarily placed whole by any other
//! single call, the way one excluded item always is. [`place_group_run`] therefore
//! bounds-checks the *whole* base range and skips the entire run — no partial placement —
//! the moment any part of it falls outside `items`, rather than computing an index from
//! `base.start.get().saturating_sub(items.start.get())` first and only discovering the
//! straddle when a lookup happens to fail: that saturating subtraction silently clamps to
//! zero when `base.start < items.start`, which would read the *wrong* placement (the line's
//! own first item's) rather than refusing to read one at all, exactly the hazard
//! [`place_mono_run`]'s own single-item guard already exists to prevent, extended to a
//! range. In practice this is a defensive floor rather than a case with a real geometry
//! answer being suppressed, **for [`RubyStyle::GroupRuby`] specifically**: `crate::lower::
//! lower_group` gives the whole compound one shared [`jlreq_unit::RunId`], and
//! `jlreq-line`'s own same-run break refusal (§C.2#8/#13, `crates/jlreq-line/src/
//! feasible.rs`) refuses a break inside one run, so no line `jlreq-line` composes can ever
//! hand [`place`] an `items` range that straddles one group-ruby run's own base — the skip
//! exists for an input `jlreq-line` does not produce, not for one it does.
//!
//! **[`RubyStyle::JukugoRuby`]'s own straddle is the opposite case: a genuinely reachable
//! input, not a defensive floor, and [`place_jukugo_compound`] answers it with a decline
//! rather than a silent skip.** §C.2#8's own second sentence states plainly that "there is
//! also a line break opportunity between two consecutive base characters belonging to the
//! same jukugo-ruby character complex," and `crate::lower::lower_jukugo` gives the compound
//! one shared `RunId` but a *fresh* [`jlreq_unit::GroupId`] per base item precisely so that
//! break survives — `docs/decisions/jukugo-ruby-unset-group.md`'s own reading of
//! `jlreq_line::feasible::same_run_refusal` is what confirms `jlreq-line` actually permits
//! it. So a paragraph-2 compound's own base range genuinely can straddle one composed
//! line's own `items`, and §3.3.7¶2's own instruction — "attach the ruby text to the kanji
//! compound word **as a whole**" — has no whole left to attach once the line has split it:
//! JLReq states no method for that case, so [`place_jukugo_compound`] declines rather than
//! placing a fragment or guessing which half owns the reading, reported through
//! [`Attachments::declined`] exactly as every other deliberate non-placement in this module
//! is. A compound split across two lines is consequently declined *twice*, once by each of
//! the two `place` calls whose own `items` only partially covers it — the correct per-line
//! answer, not a double-report defect, because each call genuinely only sees its own partial
//! overlap and neither one alone can honestly claim the compound as placed.
//!
//! No `crates/jlreq-conform` case can ever exercise this decline: `Compose::place`'s own
//! adapter method (`crates/jlreq-conform/src/kumihan.rs`) always derives `items` as
//! `ItemIndex::new(0)..ItemIndex::new(input.items.len())` — a case has no notion of "this
//! line" narrower than its own whole declared base stream, so no case-declared input can
//! ever straddle it. This is consequently a fact this crate's own unit tests below are the
//! *only* place that measures, for this suite, permanently — the identical shape
//! `docs/decisions/jukugo-ruby-unset-group.md`'s own closing section already states for
//! `same_run_refusal`'s own refusing half.
//!
//! Paragraph 1 is unaffected by any of this, and the contrast is worth one sentence: per-base
//! mono placement is *correct* under a split precisely because each `place` call places only
//! its own line's base items, and [`place_mono_run`]'s own single-item skip — stated two
//! paragraphs above — is the indexing convention working exactly as documented, not a second
//! instance of this same hazard.
//!
//! # `Attachment::side` and `Attachment::block`
//!
//! [`Attachment::side`] answers [`Side::BlockStart`] for every attachment this round
//! produces — §3.3.4 and §3.3.9 both state that side, "to the right in vertical writing
//! mode, and above in horizontal", one rule in physical axes stated twice. But every call
//! this round makes answers the identical constant, which is the same unobservability this
//! project's own `docs/conformance-deferrals.toml` §3.3.4 entry already argues against a
//! `BlockDemand` that never varies: shipping the accessor is right, because the spine names
//! it and a later round's other constructs will answer [`Side::BlockEnd`], but nobody should
//! mistake its existence for §3.3.4 being answered. §3.3.4 stays deferred to M4.
//!
//! [`Attachment::block`] answers [`BlockOffset::ZERO`] for the same reason, but it is a
//! different kind of constant and its citation says so: this signature carries no
//! block-axis input at all — no line's own block position, nothing — so zero is not a
//! geometric claim about how far a ruby character sits from anything; it is the origin a
//! caller's own block-axis placement of the base line is added to. Its own citation is
//! `n/a`, not §3.3.4: §3.3.4 is [`Attachment::side`]'s to (not) answer, and one fact
//! belongs to one accessor (ADR-0019).
//!
//! # What is not here
//!
//! Task #80 authored the independent conformance phase this module's own former text named
//! as missing: `crates/jlreq-conform` gained an eighth `"place"` kind — its own
//! `cases.schema.json`, `Compose::place` trait method and `check_place` — and three cases in
//! `crates/jlreq-conform/cases/3.3.5.json` measure the three real §3.3.5 geometries above
//! and the katatsuki-with-overflow decline, over both `RubyAlignment`s, exactly the
//! separately-authored implementation-then-conformance pairing ADR-0006 asks for. A fourth
//! case once measured group-ruby's *absence* of placement — `attachments: []` alongside
//! `declined: []`, over exactly the fixture (a 1000-unit base against two 500-unit ruby
//! characters) that M4-a round 5's own §3.3.6 paragraph 1 arithmetic now places, at offsets 0
//! and 500 — and that round deleted it rather than retargeting it: its premise, that a
//! group-ruby construct produces no attachment, is what that round's own implementation
//! falsified, and a replacement fixture or expectation belonged to task #85, ADR-0006's own
//! separately-authored conformance phase, rather than to the implementation round itself.
//!
//! Task #85 is that phase for §3.3.6, and it has since run: `crates/jlreq-conform/cases/
//! 3.3.6.json` publishes four cases naming rule `3.3.6` — the equal-length agreement
//! (paragraph 1), the `jis`-versus-`flush` divergence at three or more ruby characters and at
//! exactly one (paragraph 2, the second over both of `Question::GROUP_RUBY_DISTRIBUTION`'s
//! answers and the published `decision:group-ruby-flush-single-character` reading), and the
//! ruby-longer-than-base decline (paragraph 3), asserting the specific declined construct
//! ordinal per this module's own doc above. `crates/jlreq-conform/tests/suite.rs`'s own
//! `section_3_3_6_is_also_measured_under_flush` exercises the `flush` reading against a
//! second `Kumihan::new(Policy)` the identical way `section_3_3_5_is_also_measured_under_
//! katatsuki` already does for §3.3.5, so both of the question's answers are genuinely
//! checked rather than only published. §3.3.6 moves to `[[owned]]` in `docs/conformance-
//! deferrals.toml` on the strength of those four cases; its own entry states the honest scope
//! limit — paragraph 3 is cased as a decline, never implemented, and paragraph 2's Note is
//! cased nowhere: closing it needs a policy `Question` of its own or a `docs/decisions/`
//! reading choosing between its own two stated thresholds, and a case round authors neither,
//! the identical discipline that kept this module from inventing one when it implemented the
//! rest of paragraph 2.
//!
//! What genuinely is not here, still: no `Attachments::rules_fired`, and no new accessor of
//! that shape for §3.3.6 or §3.3.7 either, for the identical reason — each is one rule
//! address, [`crate::lower`] already records §3.3.5's the moment it resolves an alignment,
//! and §3.3.6's and §3.3.7's own geometry is observable through the `Attachment`s and
//! declined construct refs [`place`] itself emits, so a `rules_fired`-shaped accessor would
//! answer a question this crate's own output already answers, a second carrier ADR-0019
//! forbids. No §3.3.6 Note (the JIS X 4051 cap, named above as its own declared slot); no
//! §F, none of it, for [`RubyStyle::JukugoRuby`]'s own `phonetic` answer, which this round
//! declines wholesale rather than implementing any part of ("What this round declines:
//! §3.3.7 paragraph 2's own `phonetic` answer, and §F entire," above) — no emphasis dots, no
//! warichu, no tate-chu-yoko placement — the other eight constructs `docs/design/
//! api-spine.md` names remain unwritten; see `ROADMAP.md` (M4).
//!
//! Task #90 was §3.3.7's own separately-authored conformance phase, ADR-0006's discipline
//! applied a second time after task #85 closed it for §3.3.6, and it has since run:
//! `crates/jlreq-conform/cases/3.3.7.json` publishes three cases naming rule `3.3.7` — the
//! paragraph-1/paragraph-2 fixture pair (the identical base and reading, differing only in
//! how `runs` partitions it, which is what isolates the count discriminator this module's own
//! doc argues for above rather than an extent one) and a `lower` case for the one fact no
//! `place` case can observe, `Contribution::alignment_discouraged` for a jukugo construct in
//! horizontal writing. `crates/jlreq-conform/tests/suite.rs`'s own `section_3_3_7_is_also_
//! measured_under_phonetic`, `_under_flush` and `_under_katatsuki` exercise all three of this
//! file's readings against a second `Kumihan::new(Policy)` apiece, the identical shape
//! `section_3_3_6_is_also_measured_under_flush` already gives §3.3.6. §3.3.7 has moved to
//! `[[owned]]` in `docs/conformance-deferrals.toml` on the strength of those three cases; its
//! own entry states the honest scope limit — §F entire stays uncased because it stays
//! unimplemented, paragraph 2's own fourth-sentence overhang ceiling is doubly moot for a
//! `phonetic` method no case reaches past the decline, `lower_jukugo`'s own absent
//! `Separation` for a jukugo compound's surplus is stated in both `place` cases' own rationale
//! rather than left for their derived placements to imply by coincidence, and the
//! straddled-compound decline stays unit-test-only observable, because `Compose::place`'s own
//! adapter always derives `items` as a case's whole declared base stream and no case under
//! this format can ever construct the straddle — the identical scope-versus-coverage
//! distinction this module's own doc already draws for §3.3.6 between M4-a round 5 and round
//! 6, above.
//!
//! JLReq: §3.3.5, §3.3.6, §3.3.7, `decision:group-ruby-flush-single-character`, `decision:jukugo-group-layout-distribution`

use alloc::vec::Vec;
use core::ops::Range;

use jlreq_class::{Annotation, AnnotationIndex, Member, Text};
use jlreq_spec::{Policy, Question};
use jlreq_unit::{
    Advance, BlockOffset, ConstructRef, Distribution, InlineExtent, InlineOffset, ItemIndex, RunId,
    Side, Size, distribute,
};

use crate::Constructs;
use crate::lower::{Contribution, Lowered, one, sum_advances, two};
use crate::ruby::{Ruby, RubyAlignment, RubyRun, RubyStyle};

/// Place the mono-ruby, group-ruby and jukugo-ruby annotations of one composed line against
/// the placements `jlreq-line` already resolved for it.
///
/// `constructs` is the same value a prior [`crate::lower`] call read, and `contribution` is
/// that call's own answer; `items` and `placements` are one composed line's own
/// `jlreq_line::Line::items` and `jlreq_line::Line::placements`, read by the caller and
/// handed in rather than a whole `Line`, because this crate has no edge to `jlreq-line` to
/// name that type at all (ADR-0015). See this module's own doc for the indexing convention
/// `items`/`placements` share, which base items or base ranges outside `items` this call
/// silently skips or declines, why `out` is ordinarily a second [`Lowered`] and not the one
/// `contribution` borrows from, and exactly which of §3.3.5's four positioning cases,
/// §3.3.6's three paragraphs and §3.3.7's two paragraphs this function answers.
///
/// Infallible: an out-of-range or otherwise inconsistent input is skipped rather than
/// refused, because nothing this function reads can corrupt `out`'s own invariants the way
/// [`crate::lower`]'s overlapping-construct refusal exists to catch — there is no second
/// construct here to collide with, only a placement to compute or decline.
///
/// JLReq: §3.3.5, §3.3.6, §3.3.7
#[must_use]
pub fn place<'a>(
    constructs: &Constructs<'_>,
    contribution: &Contribution<'_>,
    items: Range<ItemIndex>,
    placements: &[InlineOffset],
    policy: Policy,
    out: &'a mut Lowered,
) -> Attachments<'a> {
    out.attachments.clear();
    out.declined.clear();

    let ctx = PlaceCtx {
        contribution,
        text: constructs.text(),
        items,
        placements,
        policy,
    };

    for ruby in constructs.ruby().iter().copied() {
        let annotation = ruby.annotation();
        match ruby.style() {
            RubyStyle::MonoRuby => {
                for run in ruby.runs().iter().copied() {
                    place_mono_run(&ctx, annotation, run, out);
                }
            },
            RubyStyle::GroupRuby => {
                // `Question::GROUP_RUBY_DISTRIBUTION`'s two names, read the same way
                // `crate::lower::default_alignment` reads `Question::RUBY_ALIGNMENT`. An
                // unrecognized name falls to `jis` rather than panicking: every one of
                // `Policy`'s five presets answers `jis` (`spec/derived/questions.tsv`'s own
                // `ruby.group_distribution` row), so `jis` is the honest default for an
                // input this workspace's own closed-choice generation should never actually
                // produce. Read here, once per ruby, rather than inside `place_group_run`
                // itself: §3.3.7¶2's own `group` answer reuses that function's identical
                // geometry but *forces* `jis` regardless of this question
                // (`decision:jukugo-group-layout-distribution`), so the read has to live at
                // the call site that owns the choice, not inside the geometry both callers
                // share.
                let jis = ctx.policy.get(Question::GROUP_RUBY_DISTRIBUTION).name() != "flush";
                for run in ruby.runs().iter().copied() {
                    place_group_run(&ctx, annotation, run, jis, out);
                }
            },
            RubyStyle::JukugoRuby => {
                place_jukugo(&ctx, annotation, ruby, out);
            },
        }
    }

    Attachments {
        attachments: &out.attachments,
        declined: &out.declined,
    }
}

/// Everything one [`place`] call holds constant across every run it places, mono-ruby,
/// group-ruby or jukugo-ruby alike, bundled so [`place_mono_run`] and [`place_group_run`]
/// stay under `clippy::too_many_arguments` without threading five unrelated parameters
/// through each by hand.
struct PlaceCtx<'c, 'r> {
    /// [`crate::lower`]'s own answer for the [`Constructs`] this call places against.
    contribution: &'c Contribution<'c>,
    /// The base stream, read for one thing only: a base item's own supplied advance.
    text: Text<'r>,
    /// The composed line's own item range, which `placements` is indexed relative to.
    items: Range<ItemIndex>,
    /// The composed line's own glyph-box origins, one per item of `items`.
    placements: &'r [InlineOffset],
    /// The document's own policy, read for [`Policy::remainder`] always, and, for
    /// `Question::JUKUGO_RUBY_LAYOUT`'s own answer, by [`place_jukugo`]. Not read here for
    /// `Question::GROUP_RUBY_DISTRIBUTION` any longer: [`place_group_run`] takes that
    /// question's answer as an explicit `jis` parameter now, because §3.3.7¶2's own `group`
    /// answer needs to force it regardless of what this policy says
    /// (`decision:jukugo-group-layout-distribution`), so the read moved to each of
    /// `place_group_run`'s own two call sites.
    policy: Policy,
}

/// Resolve one [`RubyRun`]'s own identity and alignment, then place it solid or decline it,
/// entirely per this module's own doc.
fn place_mono_run(
    ctx: &PlaceCtx<'_, '_>,
    annotation: Annotation<'_>,
    run: RubyRun,
    out: &mut Lowered,
) {
    let base = run.base().start;
    if base < ctx.items.start || base >= ctx.items.end {
        return;
    }
    // `base >= ctx.items.start` already holds, so this subtraction never actually
    // saturates; the fallback below is unreached in practice and answered rather than
    // assumed, the same discipline `crate::lower`'s own `write_slot` takes.
    let Ok(index) = usize::try_from(base.get().saturating_sub(ctx.items.start.get())) else {
        return;
    };
    let Some(&base_placement) = ctx.placements.get(index) else {
        return;
    };

    let Some(construct) = ctx.contribution.runs().of(base) else {
        return;
    };
    let run_id = construct.run();
    let construct_ref = ctx.contribution.construct_of(run_id);
    let Some(alignment) = ctx.contribution.alignment_of(construct_ref) else {
        return;
    };

    let base_extent = ctx
        .text
        .items()
        .get(base.get() as usize)
        .map_or(InlineExtent::ZERO, |base_item| base_item.advance());
    let run_extent = sum_advances(annotation, run.annotation());

    let run_start = match alignment {
        RubyAlignment::Nakatsuki => {
            let difference = base_extent.sub_sat(run_extent);
            let weights = [one(), one()];
            let mut shares = distribute(difference, &weights, ctx.policy.remainder());
            let leading = shares.next().unwrap_or(InlineExtent::ZERO);
            shift_by(base_placement, leading)
        },
        RubyAlignment::Katatsuki => {
            if base_extent.max(run_extent) == base_extent {
                // Not longer than the base: start-aligned, no distribution at all.
                base_placement
            } else {
                out.declined.push(construct_ref);
                return;
            }
        },
    };

    place_solid_run(
        annotation,
        run.annotation(),
        run_start,
        construct_ref,
        run_id,
        out,
    );
}

/// Lay `range` of `annotation` out solid — no inter-character space — starting at
/// `run_start`, recording one [`Attachment`] per annotation item.
///
/// This is [`place`]'s own answer to §0's EN/JA divergence: whether the caller's ruby
/// characters are Japanese ("set solid") or carry their own widths ("set according to
/// their own widths") makes no difference here, because both phrases name laying a run out
/// at its own items' supplied advances with nothing between them, which is exactly what
/// this function does regardless of what a character's own advance happens to be.
fn place_solid_run(
    annotation: Annotation<'_>,
    range: Range<AnnotationIndex>,
    run_start: InlineOffset,
    construct: ConstructRef,
    run: RunId,
    out: &mut Lowered,
) {
    let mut cursor = run_start;
    for raw in range.start.get()..range.end.get() {
        let index = AnnotationIndex::new(raw);
        let Some(&item) = annotation.items().get(raw as usize) else {
            continue;
        };
        out.attachments.push(Attachment {
            construct,
            run,
            size: annotation.size_of(index),
            side: Side::BlockStart,
            inline: cursor,
            block: BlockOffset::ZERO,
            item: Some(index),
            symbol: None,
        });
        cursor = shift_by(cursor, item.advance());
    }
}

/// `offset`, shifted `by` — an already-computed centering share (signed) or a solid run's
/// own accumulated advance (never negative) — the identical crossing `jlreq_line::compose`'s
/// own `shift_by` makes for the identical reason: `InlineOffset` and `InlineExtent` share
/// no arithmetic (ADR-0011), so this reads back the two raw unit counts and re-enters the
/// typed channel through `InlineOffset::new`. See this crate's own `docs/scalar-sites.toml`
/// entry for why this crossing is unavoidable here specifically. Saturating on overflow
/// rather than refusing, for the identical reason `jlreq_line::compose::shift_by`'s own doc
/// states: every `by` this module passes is bounded by one ruby run's or one base
/// character's own extent, itself bounded by the shared length bound, so the fallback is
/// reached only past inputs no caller-stated measure produces in practice.
fn shift_by(offset: InlineOffset, by: InlineExtent) -> InlineOffset {
    InlineOffset::new(offset.units().saturating_add(by.units())).unwrap_or(offset)
}

/// `end`, less `start`, as an inline extent — the base run's own extent, measured from a
/// composed line's own already-resolved placements (this module's own doc, "What this round
/// implements: §3.3.6 paragraphs 1 and 2"). `InlineOffset` and `InlineExtent` share no
/// arithmetic (ADR-0011), so this reads the two raw unit counts back out through `.units()`
/// in dot-call form — invisible to the `ops` gate's own `Type::method` scanner — and
/// re-enters the typed channel through the one bare path this crossing writes,
/// `InlineExtent::new`. See this crate's own `docs/scalar-sites.toml` entry for why this
/// crossing is unavoidable here specifically; `jlreq_line::tab::distance_to`'s own entry
/// argues the identical shape one crate over, though that one reads a single offset back
/// against an implicit zero rather than the difference of two already-resolved ones.
/// Saturating to zero past the bound for the same reason [`shift_by`]'s own doc states: no
/// caller-stated measure produces an input past it in practice.
fn extent_between(start: InlineOffset, end: InlineOffset) -> InlineExtent {
    InlineExtent::new(end.units().saturating_sub(start.units())).unwrap_or(InlineExtent::ZERO)
}

/// Resolve one group-ruby [`RubyRun`]'s own base extent, place §3.3.6 paragraphs 1 and 2's
/// geometry under whichever of `jis`/`flush` the caller names, or decline paragraph 3's own
/// ruby-longer-than-base half — entirely per this module's own doc.
///
/// Two call sites choose `jis` two different ways, which is why it arrives as a plain
/// parameter rather than being read from `ctx.policy` in here: [`RubyStyle::GroupRuby`]'s
/// own arm in [`place`] reads `Question::GROUP_RUBY_DISTRIBUTION` and passes its answer
/// straight through; [`place_jukugo`]'s own paragraph-2 path passes `true` unconditionally,
/// *forcing* `jis` regardless of what that question says
/// (`decision:jukugo-group-layout-distribution`). Moving the read out of this function is
/// what makes the forcing possible without a second, diverging copy of the geometry below.
fn place_group_run(
    ctx: &PlaceCtx<'_, '_>,
    annotation: Annotation<'_>,
    run: RubyRun,
    jis: bool,
    out: &mut Lowered,
) {
    let base = run.base();
    // Bounds-check the *whole* range against `items`, not one item — this module's own doc,
    // "Indexing convention", states why a range needs this and a single mono-ruby base item
    // does not.
    if base.start >= base.end || base.start < ctx.items.start || base.end > ctx.items.end {
        return;
    }
    let Ok(first_index) = usize::try_from(base.start.get().saturating_sub(ctx.items.start.get()))
    else {
        return;
    };
    let Ok(last_index) = usize::try_from(
        base.end
            .get()
            .saturating_sub(1)
            .saturating_sub(ctx.items.start.get()),
    ) else {
        return;
    };
    let Some(&base_start) = ctx.placements.get(first_index) else {
        return;
    };
    let Some(&last_placement) = ctx.placements.get(last_index) else {
        return;
    };
    let Some(last_item) = ctx
        .text
        .items()
        .get(base.end.get().saturating_sub(1) as usize)
    else {
        return;
    };
    let base_extent = extent_between(base_start, shift_by(last_placement, last_item.advance()));

    let Some(construct) = ctx.contribution.runs().of(base.start) else {
        return;
    };
    let run_id = construct.run();
    let construct_ref = ctx.contribution.construct_of(run_id);

    let ruby_range = run.annotation();
    // `count` is the declared width of `ruby_range`, not a count of items actually walked;
    // the two cannot disagree for a `Ruby` built through `Ruby::new`, whose own `check_runs`
    // refuses a run whose `annotation` range reaches past the reading it was declared over
    // (`RubyError::AnnotationOutOfRange`), so [`place_group_solid_run`]'s own `continue` past
    // a missing annotation item is unreached in practice for this caller, the same
    // discipline [`place_mono_run`]'s own bounds check already states for its single item.
    // Read here, ahead of `ruby_range`'s own move into `sum_advances` below (`Range` is not
    // `Copy`), rather than cloned back out afterward.
    let count = ruby_range.end.get().saturating_sub(ruby_range.start.get());
    let ruby_extent = sum_advances(annotation, ruby_range.clone());

    if base_extent.max(ruby_extent) != base_extent {
        // §3.3.6 paragraph 3: the ruby is genuinely longer than the base. Both of the
        // section's own methods spread the *base* characters apart, which this function
        // cannot do — this module's own doc, "What this round declines: §3.3.6 paragraph 3",
        // states why in full.
        out.declined.push(construct_ref);
        return;
    }

    let surplus = base_extent.sub_sat(ruby_extent);

    let weights: Vec<Advance> = if jis {
        group_jis_weights(count)
    } else {
        group_flush_weights(count)
    };
    let mut shares = distribute(surplus, &weights, ctx.policy.remainder());
    // `jis`'s own weights carry a leading site (the run's own first share); `flush`'s own
    // weights are the interior sites alone, and its leading offset is the fixed
    // `InlineExtent::ZERO` §3.3.6's own second method states directly rather than a share
    // `distribute` computes (`docs/decisions/group-ruby-flush-single-character.md`).
    let leading = if jis {
        shares.next().unwrap_or(InlineExtent::ZERO)
    } else {
        InlineExtent::ZERO
    };

    place_group_solid_run(
        annotation,
        ruby_range,
        shift_by(base_start, leading),
        shares,
        construct_ref,
        run_id,
        out,
    );
}

/// §3.3.6's own `jis` (JIS X 4051) weights: `[1, 2, 2, …, 2, 1]` over `count + 1` sites — the
/// section's own "2 units of inter-character spacing between ruby characters... 1 unit of
/// spacing" at each of the run's two ends, read as a proportional split
/// ([`jlreq_unit::distribute`]'s own doc: "serves every rule whose divisor depends on the
/// text"). At `count == 1` this degenerates to `[1, 1]` — no interior site at all — which is
/// centered, the identical geometry paragraph 1's own equal-length case falls out to (this
/// module's own doc, §4.3).
fn group_jis_weights(count: u32) -> Vec<Advance> {
    let mut weights = Vec::new();
    weights.push(one());
    for _ in 1..count {
        weights.push(two());
    }
    weights.push(one());
    weights
}

/// §3.3.6's own `flush` weights: equal shares over the `count - 1` *interior* sites only —
/// no leading or trailing site at all, because the method's own leading clause ("first align
/// the leading characters... and the ends of both trailing characters") is honored by
/// [`place_group_run`]'s own fixed [`InlineExtent::ZERO`] leading offset rather than by a
/// zero-weight site here: a zero-weight site does not stay zero under
/// [`jlreq_unit::distribute`], whose own `level`/`extra`/`step` remainder machinery hands
/// units out across every site the weights slice names, including one weighted zero. At
/// `count == 1` this is empty, which is
/// `docs/decisions/group-ruby-flush-single-character.md`'s own published reading.
fn group_flush_weights(count: u32) -> Vec<Advance> {
    let interior = count.saturating_sub(1);
    let mut weights = Vec::new();
    for _ in 0..interior {
        weights.push(one());
    }
    weights
}

/// Lay `range` of `annotation` out with `gaps` interposed between adjacent characters —
/// §3.3.6 paragraphs 1 and 2's own geometry, one [`Attachment`] per annotation item exactly
/// as [`place_solid_run`] emits them, but with a distributed share after every item except
/// the last rather than nothing between them. A sibling of [`place_solid_run`], not a
/// generalization of it: mono-ruby's own solid setting reads no gap at all, and threading
/// one through that function to serve this one further call site would risk exactly the
/// mono-ruby offset drift this module's own doc rules out.
fn place_group_solid_run(
    annotation: Annotation<'_>,
    range: Range<AnnotationIndex>,
    run_start: InlineOffset,
    mut gaps: Distribution<'_>,
    construct: ConstructRef,
    run: RunId,
    out: &mut Lowered,
) {
    let last = range.end.get().saturating_sub(1);
    let mut cursor = run_start;
    for raw in range.start.get()..range.end.get() {
        let index = AnnotationIndex::new(raw);
        let Some(&item) = annotation.items().get(raw as usize) else {
            continue;
        };
        out.attachments.push(Attachment {
            construct,
            run,
            size: annotation.size_of(index),
            side: Side::BlockStart,
            inline: cursor,
            block: BlockOffset::ZERO,
            item: Some(index),
            symbol: None,
        });
        cursor = shift_by(cursor, item.advance());
        if raw != last {
            if let Some(gap) = gaps.next() {
                cursor = shift_by(cursor, gap);
            }
        }
    }
}

/// §3.3.7: discriminate paragraph 1 from paragraph 2 by a character *count*, not an extent
/// comparison (this module's own doc, "What this round implements: §3.3.7 (jukugo-ruby)",
/// states why the usual reduction does not apply here), then place each paragraph's own
/// geometry.
fn place_jukugo(
    ctx: &PlaceCtx<'_, '_>,
    annotation: Annotation<'_>,
    ruby: Ruby<'_>,
    out: &mut Lowered,
) {
    let runs = ruby.runs();
    let paragraph_one = runs.iter().all(|run| {
        let range = run.annotation();
        range.end.get().saturating_sub(range.start.get()) <= 2
    });
    if paragraph_one {
        // Every base character carries two or fewer ruby characters: delegate each run to
        // §3.3.5's own method, unmodified — the identical function `RubyStyle::MonoRuby`
        // itself calls.
        for run in runs.iter().copied() {
            place_mono_run(ctx, annotation, run, out);
        }
        return;
    }
    place_jukugo_compound(ctx, annotation, ruby, out);
}

/// §3.3.7 paragraph 2: some base character needs three or more ruby characters, so the whole
/// compound is attached as one unit rather than per base character — entirely per this
/// module's own doc, "What this round implements: §3.3.7 (jukugo-ruby)" and "Indexing
/// convention" (the straddled-compound answer).
fn place_jukugo_compound(
    ctx: &PlaceCtx<'_, '_>,
    annotation: Annotation<'_>,
    ruby: Ruby<'_>,
    out: &mut Lowered,
) {
    let base = ruby.base();
    if base.start >= base.end {
        return;
    }
    let Some(construct) = ctx.contribution.runs().of(base.start) else {
        return;
    };
    let construct_ref = ctx.contribution.construct_of(construct.run());

    let no_overlap = base.end <= ctx.items.start || base.start >= ctx.items.end;
    if no_overlap {
        // This line's own call never reaches the compound at all: a different `place` call,
        // over a different composed line, already placed it, or will (this module's own
        // doc, "Indexing convention").
        return;
    }
    if base.start < ctx.items.start || base.end > ctx.items.end {
        // The compound straddles this call's own `items` — genuinely reachable for
        // `RubyStyle::JukugoRuby`, unlike `RubyStyle::GroupRuby`'s own unreachable straddle
        // (this module's own doc, "Indexing convention", states why in full). §3.3.7¶2's
        // own instruction is to attach the reading to the compound "as a whole," and a
        // compound this line has split in half has no whole left to attach it to.
        out.declined.push(construct_ref);
        return;
    }

    if ctx.policy.get(Question::JUKUGO_RUBY_LAYOUT).name() == "phonetic" {
        // §F's own phonetic-structure distribution is not implemented this round (this
        // module's own doc, "What this round declines: §3.3.7 paragraph 2's own `phonetic`
        // answer, and §F entire").
        out.declined.push(construct_ref);
        return;
    }

    // The first declared run's own annotation start through the last's own end: `Ruby::new`'s
    // own `check_runs` validation guarantees the declared runs cover the whole reading
    // contiguously in order starting at `AnnotationIndex::new(0)`, so this span is the
    // compound's whole reading without re-deriving it from `annotation.items().len()`. A
    // `Ruby` with a non-empty base (already checked above) always has at least one run
    // (`RubyError::RunCount`), so the fallback below is unreached in practice and answered
    // rather than assumed, the same discipline [`place_mono_run`]'s own bounds check already
    // states for its single item.
    let runs = ruby.runs();
    let (Some(first), Some(last)) = (runs.first().copied(), runs.last().copied()) else {
        return;
    };
    let ruby_range = first.annotation().start..last.annotation().end;
    // §3.3.7¶2's own `group` answer (or the fallback for an unrecognized name — the
    // identical precedent `place_group_run`'s own former internal read established for
    // `Question::GROUP_RUBY_DISTRIBUTION`): reuse `place_group_run`'s identical §3.3.6
    // geometry over one compound-wide synthetic `RubyRun`, forced to `jis` regardless of the
    // document's own `Question::GROUP_RUBY_DISTRIBUTION` answer
    // (`decision:jukugo-group-layout-distribution`). Reusing that function also reuses its
    // own ruby-longer-than-base decline (§3.3.6 paragraph 3's own base-spreading half),
    // which this crate cannot perform for a jukugo compound for the identical reason it
    // cannot for a group-ruby run.
    place_group_run(ctx, annotation, RubyRun::new(base, ruby_range), true, out);
}

/// One annotation, placed against its base. The caller draws it with the size and the
/// offsets given.
///
/// Named `Attachment` and not `Annotation`, because [`Annotation`] is the input stream: a
/// stream of ruby characters and one placed mark are different things and one name for
/// both would be the confusion ADR-0016 is about, one level up. Copy, per the small
/// integer-only-type preamble `docs/design/api-spine.md` states: every field is itself
/// `Copy` and none is larger than the axis types this workspace already passes by value.
///
/// JLReq: n/a (ADR-0015, ADR-0016)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Attachment {
    construct: ConstructRef,
    run: RunId,
    size: Size,
    side: Side,
    inline: InlineOffset,
    block: BlockOffset,
    item: Option<AnnotationIndex>,
    symbol: Option<Member>,
}

impl Attachment {
    /// Which declared construct this came from, in the caller's own coordinates. Resolved
    /// through [`Contribution::construct_of`], so a caller can attribute every mark it is
    /// handed to the ruby it asked for (ADR-0015).
    ///
    /// JLReq: n/a (ADR-0015)
    #[must_use]
    pub const fn construct(self) -> ConstructRef {
        self.construct
    }

    /// The run identity [`crate::lower`] allocated for this attachment's own base item, so
    /// the same-run predicates and this placement agree about what "one run" is without a
    /// second identity space to keep in sync (ADR-0015).
    ///
    /// JLReq: n/a (ADR-0015)
    #[must_use]
    pub const fn run(self) -> RunId {
        self.run
    }

    /// The size to draw at: the annotation stream's own declared size at this item, the
    /// single carrier of the ruby em (ADR-0019).
    ///
    /// JLReq: §3.3.3
    #[must_use]
    pub const fn size(self) -> Size {
        self.size
    }

    /// Which side of the base this attachment sits on. [`Side::BlockStart`] for every
    /// attachment this round produces. [`Side`] is direction-abstract by construction —
    /// block-start, not "above" or "to the right" — so this answers *which* side, never
    /// the *physical* one §3.3.4 states; see this module's own doc for why that constant
    /// answer does not settle §3.3.4, which stays deferred for exactly the physical half
    /// this accessor cannot carry.
    ///
    /// JLReq: §3.3.4
    #[must_use]
    pub const fn side(self) -> Side {
        self.side
    }

    /// This attachment's own inline-axis origin, in the composed line's own coordinates —
    /// the same frame `placements` was given in.
    ///
    /// JLReq: §3.3.5
    #[must_use]
    pub const fn inline(self) -> InlineOffset {
        self.inline
    }

    /// This attachment's own block-axis origin. [`BlockOffset::ZERO`] for every attachment
    /// this round produces, and not a claim about §3.3.4 or any other rule — see this
    /// module's own doc for why this accessor's constant answer is an absence of input
    /// rather than a derived geometric fact.
    ///
    /// JLReq: n/a (this signature carries no block-axis reference frame)
    #[must_use]
    pub const fn block(self) -> BlockOffset {
        self.block
    }

    /// The annotation stream's own item this attachment draws, or `None` for a construct
    /// that repeats one member rather than placing a stream — no mono-ruby or group-ruby
    /// attachment ever answers `None` here; that answer is reserved for emphasis dots,
    /// unimplemented this round (ADR-0016).
    ///
    /// JLReq: n/a (ADR-0016)
    #[must_use]
    pub const fn item(self) -> Option<AnnotationIndex> {
        self.item
    }

    /// The repeated member a construct with no stream of its own draws, or `None` for
    /// ruby, which places a stream instead. Always `None` this round: only ruby is
    /// implemented, and ruby never carries a symbol (ADR-0016).
    ///
    /// JLReq: n/a (ADR-0016)
    #[must_use]
    pub const fn symbol(self) -> Option<Member> {
        self.symbol
    }
}

/// Every [`Attachment`] one [`place`] call produced, and every run — mono-ruby, group-ruby
/// or jukugo-ruby alike — it declined to place instead.
///
/// JLReq: n/a (ADR-0015)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Attachments<'a> {
    attachments: &'a [Attachment],
    declined: &'a [ConstructRef],
}

impl<'a> Attachments<'a> {
    /// Every annotation this call placed, in the order [`place`] walked the declared ruby.
    ///
    /// JLReq: §3.3.5
    #[must_use]
    pub const fn attachments(&self) -> &'a [Attachment] {
        self.attachments
    }

    /// Every run this call considered and deliberately did not place, for one of four
    /// stated reasons — never a construct this round simply has no code for yet (this
    /// module's own doc, "`Attachments::declined` now reports four reasons, not two").
    ///
    /// **§3.3.5(c)'s own katatsuki-with-overflow choice**: every katatsuki
    /// [`RubyStyle::MonoRuby`] run whose own run extent exceeds its base's — the extent
    /// comparison §3.3.5(c) states two methods for, neither of which any `Question` reads
    /// yet (task #81; this module's own doc states why) — and, for the identical reason,
    /// every katatsuki run of a [`RubyStyle::JukugoRuby`] compound's own §3.3.7¶1, which
    /// delegates to the identical method. The gate is that extent comparison alone, not a
    /// character count: this module's own doc argues why ordinary mono-ruby proportions
    /// confine it to three-or-more-character runs in practice, which is the case JLReq's own
    /// text names it under, but a one- or two-character katatsuki run whose own extent
    /// happens to outrun its base declines here exactly the same way.
    ///
    /// **§3.3.6 paragraph 3's own base-spreading method**: every [`RubyStyle::GroupRuby`]
    /// run whose ruby is genuinely longer than its base — this module's own doc, "What this
    /// round declines: §3.3.6 paragraph 3", states why this crate cannot perform either of
    /// the section's own two methods, both of which spread the *base* characters rather
    /// than moving the ruby — and, for the identical reason, every [`RubyStyle::JukugoRuby`]
    /// compound whose reading is genuinely longer than its base under §3.3.7¶2's own `group`
    /// answer, which reuses the identical geometry (`decision:jukugo-group-layout-
    /// distribution`).
    ///
    /// **§3.3.7¶2's own `phonetic` answer**: every [`RubyStyle::JukugoRuby`] compound
    /// `Question::JUKUGO_RUBY_LAYOUT` names `phonetic` — §F's own distribution is not
    /// implemented this round (this module's own doc, "What this round declines: §3.3.7
    /// paragraph 2's own `phonetic` answer, and §F entire").
    ///
    /// **A straddled jukugo compound**: every [`RubyStyle::JukugoRuby`] compound §3.3.7¶2's
    /// own "attach... as a whole" instruction reaches whose base range this call's own
    /// `items` only partially covers — this module's own doc, "Indexing convention," states
    /// why this is reachable for jukugo-ruby and not for group-ruby, and why the identical
    /// compound is declined again by whichever other `place` call covers its remaining half.
    ///
    /// A caller that ignores this accessor simply draws nothing for the construct it names
    /// — no fallback geometry is guessed here — and `jlreq::diagnose`
    /// (`docs/design/api-spine.md`, still unwritten) is where such a report is meant to
    /// surface to a caller that never asks for it directly.
    ///
    /// One [`ConstructRef`] per declined run, not per declined construct: a caller who
    /// declared one [`crate::Ruby`] over several base characters may see its own construct
    /// here more than once if more than one of its own runs overflowed under katatsuki,
    /// because a single [`ConstructRef`] names the whole declared ruby and not the one run
    /// inside it that overflowed — [`crate::lower`] allocates one per declared
    /// [`crate::Ruby`], not one per [`RubyRun`]. A [`RubyStyle::GroupRuby`] ruby always
    /// declares exactly one run ([`crate::RubyError::RunCount`]), so it can appear here at
    /// most once regardless; a [`RubyStyle::JukugoRuby`] compound declined under §3.3.7¶2
    /// likewise names its one construct once per declining `place` call, never once per base
    /// character.
    ///
    /// JLReq: §3.3.5, §3.3.6, §3.3.7
    pub fn declined(&self) -> impl Iterator<Item = ConstructRef> + '_ {
        self.declined.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use jlreq_class::{Annotation, AnnotationIndex, Text};
    use jlreq_spec::{Choice, Policy, Question};
    use jlreq_unit::{
        Advance, ByteOffset, Direction, InlineExtent, InlineOffset, Item, ItemIndex, Scale, ScaleId,
    };

    use super::place;
    use crate::Constructs;
    use crate::lower::{Lowered, lower};
    use crate::ruby::{Ruby, RubyAlignment, RubyRun, RubyStyle};

    /// The named answer of a question, for a test to select — `crates/jlreq-spec/src/
    /// policy.rs`'s own test-module `choice` helper, the established idiom reused here
    /// rather than duplicated with different behavior.
    fn choice(question: Question, name: &str) -> Choice {
        question
            .permits()
            .iter()
            .find(|choice| choice.name() == name)
            .copied()
            .unwrap_or_else(|| panic!("`{name}` is not one of {question:?}'s answers"))
    }

    /// `Policy::JLREQ` with `ruby.group_distribution` overridden to `flush`, the second of
    /// `Question::GROUP_RUBY_DISTRIBUTION`'s two answers.
    fn flush_policy() -> Policy {
        Policy::JLREQ
            .with(choice(Question::GROUP_RUBY_DISTRIBUTION, "flush"))
            .expect("`ruby.group_distribution` conflicts with nothing else in `Policy::JLREQ`")
    }

    /// `Policy::JLREQ` with `ruby.jukugo_layout` overridden to `phonetic`, the second of
    /// `Question::JUKUGO_RUBY_LAYOUT`'s two answers.
    fn phonetic_policy() -> Policy {
        Policy::JLREQ
            .with(choice(Question::JUKUGO_RUBY_LAYOUT, "phonetic"))
            .expect("`ruby.jukugo_layout` conflicts with nothing else in `Policy::JLREQ`")
    }

    /// Horizontal writing, for the [`lower`] call every fixture here needs to build a
    /// `Contribution` at all. None of these tests exercises §3.3.5's own
    /// direction-conditional discouragement (that is `crate::lower`'s own test module's
    /// job); this is the allowlisted item for `crate::place`, the same practice
    /// `docs/direction-sites.toml` already establishes for a fixture that needs a direction
    /// and exercises none of the three direction-conditional rules.
    fn horizontal() -> Direction {
        Direction::Horizontal
    }

    /// A one-em square size at `units`.
    fn scale(units: i32) -> Scale {
        Scale::square(Advance::new(units).unwrap()).expect("a positive em")
    }

    /// One item at `start`, `advance` wide, at the base size.
    fn item(start: u32, advance: i32) -> Item {
        Item::new(
            ByteOffset::new(start),
            InlineExtent::new(advance).unwrap(),
            ScaleId::BASE,
        )
    }

    /// A run of `n` ruby-sized items, `advance` wide each.
    fn ruby_items(n: usize, advance: i32) -> Vec<Item> {
        (0..n)
            .map(|index| item(u32::try_from(index).unwrap().saturating_mul(3), advance))
            .collect()
    }

    #[test]
    fn nakatsuki_and_katatsuki_place_a_one_character_run_at_different_offsets() {
        // 1000-unit base, one 500-unit ruby character: nakatsuki centers (shift +250),
        // katatsuki start-aligns (shift +0). This is the round's own load-bearing
        // observable — the same run, two different resolved offsets.
        let base_items = [item(0, 1000)];
        let base_scales = [scale(1000)];
        let text = Text::new("鬼", &base_items, &base_scales).expect("one ideograph");
        let reading_items = ruby_items(1, 500);
        let reading_scales = [scale(500)];
        let annotation = Annotation::new("き", &reading_items, &reading_scales).expect("one kana");
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(1),
        )];
        let base_ruby = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(1),
            annotation,
            &runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run over one base item");

        let items = ItemIndex::new(0)..ItemIndex::new(1);
        let placements = [InlineOffset::new(0).unwrap()];

        let mut offsets = Vec::new();
        for alignment in [RubyAlignment::Nakatsuki, RubyAlignment::Katatsuki] {
            let ruby = base_ruby.with_alignment(alignment);
            let declared = [ruby];
            let constructs = Constructs::over(text).with_ruby(&declared);
            let mut lower_scratch = Lowered::new();
            let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut lower_scratch)
                .expect("a well-formed mono-ruby lowers");
            let mut place_scratch = Lowered::new();
            let attachments = place(
                &constructs,
                &contribution,
                items.clone(),
                &placements,
                Policy::JLREQ,
                &mut place_scratch,
            );
            offsets.push(attachments.attachments()[0].inline());
        }

        assert_eq!(
            offsets[0],
            InlineOffset::new(250).unwrap(),
            "nakatsuki: 500 of 1000 units centers with a 250-unit leading share"
        );
        assert_eq!(
            offsets[1],
            InlineOffset::new(0).unwrap(),
            "katatsuki: start-aligned with no distribution"
        );
        assert_ne!(
            offsets[0], offsets[1],
            "the two alignments resolve to different offsets for the identical run"
        );
    }

    #[test]
    fn two_ruby_characters_exactly_filling_the_base_agree_under_both_alignments() {
        // §3.3.5(a): two 500-unit ruby characters against a 1000-unit base. The centering
        // difference is zero and the run's own extent equals the base's, so nakatsuki and
        // katatsuki fall out to the identical geometry without either branch asking how
        // many ruby characters there are.
        let base_items = [item(0, 1000)];
        let base_scales = [scale(1000)];
        let text = Text::new("鬼", &base_items, &base_scales).expect("one ideograph");
        let reading_items = ruby_items(2, 500);
        let reading_scales = [scale(500)];
        let annotation =
            Annotation::new("きき", &reading_items, &reading_scales).expect("two kana");
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(2),
        )];
        let base_ruby = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(1),
            annotation,
            &runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run over one base item");

        let items = ItemIndex::new(0)..ItemIndex::new(1);
        let placements = [InlineOffset::new(0).unwrap()];

        for alignment in [RubyAlignment::Nakatsuki, RubyAlignment::Katatsuki] {
            let ruby = base_ruby.with_alignment(alignment);
            let declared = [ruby];
            let constructs = Constructs::over(text).with_ruby(&declared);
            let mut lower_scratch = Lowered::new();
            let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut lower_scratch)
                .expect("a well-formed mono-ruby lowers");
            let mut place_scratch = Lowered::new();
            let attachments = place(
                &constructs,
                &contribution,
                items.clone(),
                &placements,
                Policy::JLREQ,
                &mut place_scratch,
            );
            let placed = attachments.attachments();
            assert_eq!(placed.len(), 2, "one attachment per ruby character");
            assert_eq!(
                placed[0].inline(),
                InlineOffset::new(0).unwrap(),
                "{alignment:?}: the run starts exactly at the base's own placement"
            );
            assert_eq!(
                placed[1].inline(),
                InlineOffset::new(500).unwrap(),
                "{alignment:?}: solid setting places the second character after the \
                 first character's own advance"
            );
        }
    }

    #[test]
    fn three_ruby_characters_longer_than_the_base_center_with_negative_shares_under_nakatsuki() {
        // 1000-unit base, three 600-unit ruby characters (1800 total): the centering
        // difference is 1000 - 1800 = -800, split into two -400 shares. The run starts
        // 400 units before the base's own placement.
        let base_items = [item(0, 1000)];
        let base_scales = [scale(1000)];
        let text = Text::new("鬼", &base_items, &base_scales).expect("one ideograph");
        let reading_items = ruby_items(3, 600);
        let reading_scales = [scale(600)];
        let annotation =
            Annotation::new("かかか", &reading_items, &reading_scales).expect("three kana");
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(3),
        )];
        let ruby = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(1),
            annotation,
            &runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run over one base item");
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);
        let items = ItemIndex::new(0)..ItemIndex::new(1);
        let placements = [InlineOffset::new(1000).unwrap()];

        let mut lower_scratch = Lowered::new();
        let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut lower_scratch)
            .expect("a well-formed mono-ruby lowers");
        let mut place_scratch = Lowered::new();
        let attachments = place(
            &constructs,
            &contribution,
            items,
            &placements,
            Policy::JLREQ,
            &mut place_scratch,
        );
        let placed = attachments.attachments();
        assert_eq!(placed.len(), 3);
        assert_eq!(
            placed[0].inline(),
            InlineOffset::new(600).unwrap(),
            "the run starts 400 units before the base's own 1000-unit placement"
        );
        assert!(
            attachments.declined().next().is_none(),
            "nakatsuki never declines: §3.3.5(c) states only the centering method for it"
        );
    }

    #[test]
    fn katatsuki_declines_a_run_longer_than_its_base() {
        // The identical fixture as the negative-share test, but katatsuki: §3.3.5(c)
        // states two methods for this case and this round resolves neither (task #81).
        let base_items = [item(0, 1000)];
        let base_scales = [scale(1000)];
        let text = Text::new("鬼", &base_items, &base_scales).expect("one ideograph");
        let reading_items = ruby_items(3, 600);
        let reading_scales = [scale(600)];
        let annotation =
            Annotation::new("かかか", &reading_items, &reading_scales).expect("three kana");
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(3),
        )];
        let ruby = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(1),
            annotation,
            &runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run over one base item")
        .with_alignment(RubyAlignment::Katatsuki);
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);
        let items = ItemIndex::new(0)..ItemIndex::new(1);
        let placements = [InlineOffset::new(0).unwrap()];

        let mut lower_scratch = Lowered::new();
        let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut lower_scratch)
            .expect("a well-formed mono-ruby lowers");
        let mut place_scratch = Lowered::new();
        let attachments = place(
            &constructs,
            &contribution,
            items,
            &placements,
            Policy::JLREQ,
            &mut place_scratch,
        );
        assert!(
            attachments.attachments().is_empty(),
            "a declined run produces no attachment"
        );
        let run = contribution
            .runs()
            .of(ItemIndex::new(0))
            .expect("the base item joined a run");
        let expected = contribution.construct_of(run.run());
        let declined: Vec<_> = attachments.declined().collect();
        assert_eq!(
            declined,
            [expected],
            "the declined construct is reported exactly once"
        );
    }

    #[test]
    fn a_line_whose_items_do_not_start_at_zero_reads_placements_relative_to_its_own_start() {
        // Four base items, but this "line" is only items 2..4; the base with the ruby is
        // item 3, so it must read placements[1], not placements[3] (out of bounds) or
        // placements[0] (the wrong item).
        let base_items = [item(0, 1000), item(3, 1000), item(6, 1000), item(9, 1000)];
        let base_scales = [scale(1000)];
        let text = Text::new("鬼門方角", &base_items, &base_scales).expect("four ideographs");
        let reading_items = [item(0, 500)];
        let reading_scales = [scale(500)];
        let annotation = Annotation::new("き", &reading_items, &reading_scales).expect("one kana");
        let runs = [RubyRun::new(
            ItemIndex::new(3)..ItemIndex::new(4),
            AnnotationIndex::new(0)..AnnotationIndex::new(1),
        )];
        let ruby = Ruby::new(
            text,
            ItemIndex::new(3)..ItemIndex::new(4),
            annotation,
            &runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run over item 3");
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);

        let mut lower_scratch = Lowered::new();
        let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut lower_scratch)
            .expect("a well-formed mono-ruby lowers");

        let items = ItemIndex::new(2)..ItemIndex::new(4);
        let placements = [
            InlineOffset::new(0).unwrap(),
            InlineOffset::new(1000).unwrap(),
        ];
        let mut place_scratch = Lowered::new();
        let attachments = place(
            &constructs,
            &contribution,
            items,
            &placements,
            Policy::JLREQ,
            &mut place_scratch,
        );
        let placed = attachments.attachments();
        assert_eq!(
            placed.len(),
            1,
            "item 3 lies inside items 2..4 and must be placed, not skipped"
        );
        assert_eq!(
            placed[0].inline(),
            InlineOffset::new(1250).unwrap(),
            "placements[1] (item 3's own placement, 1000) plus a 250-unit centering share"
        );
    }

    /// Build one [`RubyStyle::GroupRuby`] over the whole of `base_items`, read by an
    /// annotation over the whole of `reading_items`, sharing every test below's own fixture
    /// plumbing (`Text::new`, `Annotation::new`, `Ruby::new`) so each test states only its
    /// own numbers.
    fn group_ruby_fixture<'r>(
        text: &'r str,
        base_items: &'r [Item],
        base_scales: &'r [Scale],
        reading: &'r str,
        reading_items: &'r [Item],
        reading_scales: &'r [Scale],
        runs: &'r [RubyRun],
    ) -> (Text<'r>, Ruby<'r>) {
        let text = Text::new(text, base_items, base_scales).expect("a well-formed base stream");
        let annotation =
            Annotation::new(reading, reading_items, reading_scales).expect("a well-formed reading");
        let whole_base =
            ItemIndex::new(0)..ItemIndex::new(u32::try_from(base_items.len()).unwrap());
        let ruby = Ruby::new(text, whole_base, annotation, runs, RubyStyle::GroupRuby)
            .expect("one run over the whole base");
        (text, ruby)
    }

    /// Place `ruby` against `items`/`placements` under `policy`, returning the `Attachments`
    /// this call produced by writing into fresh scratch buffers — the two-`Lowered`-buffer
    /// shape this module's own doc states.
    fn place_group(
        text: Text<'_>,
        ruby: Ruby<'_>,
        items: core::ops::Range<ItemIndex>,
        placements: &[InlineOffset],
        policy: Policy,
    ) -> (Vec<InlineOffset>, usize) {
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);
        let mut lower_scratch = Lowered::new();
        let contribution = lower(&constructs, policy, horizontal(), &mut lower_scratch)
            .expect("a well-formed group-ruby lowers");
        let mut place_scratch = Lowered::new();
        let attachments = place(
            &constructs,
            &contribution,
            items,
            placements,
            policy,
            &mut place_scratch,
        );
        let offsets = attachments
            .attachments()
            .iter()
            .map(|attachment| attachment.inline())
            .collect();
        let declined = attachments.declined().count();
        (offsets, declined)
    }

    #[test]
    fn group_ruby_jis_distributes_the_surplus_one_two_two_one_over_three_characters() {
        // §3.3.6's own `jis` method, worked by hand: a 1200-unit base (three 400-unit
        // items, placements 0/400/800, so `extent_between(0, 800 + 400) == 1200`) against a
        // 900-unit reading (three 300-unit ruby characters): surplus 300. `jis`'s own
        // weights for three characters are `[1, 2, 2, 1]` (four sites, summing to 6), and
        // 300 divides 6 exactly (`300 * 1 / 6 == 50`, `300 * 2 / 6 == 100`), so the
        // remainder rule never has to break a tie and the shares are exactly [50, 100, 100,
        // 50] — leading, two interior gaps, trailing.
        //
        // The run therefore starts at `0 + 50 == 50`; the first character is 300 units
        // wide, so the next site is `50 + 300 + 100 == 450`; the third is
        // `450 + 300 + 100 == 850`. The trailing share (50) is never consumed — nothing sits
        // after the last character — but it is still checked below, from the base's own end.
        let base_items = [item(0, 400), item(3, 400), item(6, 400)];
        let base_scales = [scale(400)];
        let reading_items = ruby_items(3, 300);
        let reading_scales = [scale(300)];
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(3),
            AnnotationIndex::new(0)..AnnotationIndex::new(3),
        )];
        let (text, ruby) = group_ruby_fixture(
            "鬼門方",
            &base_items,
            &base_scales,
            "かかか",
            &reading_items,
            &reading_scales,
            &runs,
        );
        let items = ItemIndex::new(0)..ItemIndex::new(3);
        let placements = [
            InlineOffset::new(0).unwrap(),
            InlineOffset::new(400).unwrap(),
            InlineOffset::new(800).unwrap(),
        ];

        let (offsets, declined) = place_group(text, ruby, items, &placements, Policy::JLREQ);
        assert_eq!(declined, 0);
        assert_eq!(offsets.len(), 3);
        assert_eq!(
            offsets[0],
            InlineOffset::new(50).unwrap(),
            "the leading share is 300 * 1 / 6 == 50"
        );
        assert_eq!(
            offsets[2],
            InlineOffset::new(850).unwrap(),
            "600 (two characters' own advance) + 100 (one interior gap) further than the \
             leading character, at 50 + 300 + 100 + 300 == 850"
        );
        let leading_share = offsets[0].units();
        let interior_gap = offsets[1].units() - offsets[0].units() - 300;
        assert_eq!(
            interior_gap,
            leading_share * 2,
            "the interior gap (weight 2) is exactly twice the leading share (weight 1), \
             §3.3.6's own '2 units... 1 unit' ratio"
        );
        let base_end = InlineOffset::new(1200).unwrap();
        let trailing_gap = base_end.units() - (offsets[2].units() + 300);
        assert_eq!(
            trailing_gap, leading_share,
            "the trailing share also reads 50 here because 300 divides `jis`'s own six \
             units of weight exactly; nothing enforces this equality in general, only this \
             fixture's own exact division"
        );
    }

    #[test]
    fn group_ruby_flush_keeps_both_ends_flush_and_splits_interior_gaps_equally() {
        // The identical fixture as the `jis` test above, under `flush` instead: leading is
        // the fixed zero §3.3.6's own second method states directly, and the interior
        // weights are the `count - 1 == 2` equal sites `[1, 1]`. 300 divides 2 exactly
        // (150 each), so both interior gaps read 150 regardless of the remainder rule.
        //
        // The run starts at exactly the base's own start (0); the first character is 300
        // units, so the next site is `0 + 300 + 150 == 450`, and the third is
        // `450 + 300 + 150 == 900`. The third character's own end, `900 + 300 == 1200`, is
        // exactly the base's own end — both ends flush, unlike `jis`'s own fixture above,
        // where every unit consumed the `distribute` call's own two-way exactness rather
        // than a fixed offset.
        let base_items = [item(0, 400), item(3, 400), item(6, 400)];
        let base_scales = [scale(400)];
        let reading_items = ruby_items(3, 300);
        let reading_scales = [scale(300)];
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(3),
            AnnotationIndex::new(0)..AnnotationIndex::new(3),
        )];
        let (text, ruby) = group_ruby_fixture(
            "鬼門方",
            &base_items,
            &base_scales,
            "かかか",
            &reading_items,
            &reading_scales,
            &runs,
        );
        let items = ItemIndex::new(0)..ItemIndex::new(3);
        let placements = [
            InlineOffset::new(0).unwrap(),
            InlineOffset::new(400).unwrap(),
            InlineOffset::new(800).unwrap(),
        ];

        let (offsets, declined) = place_group(text, ruby, items, &placements, flush_policy());
        assert_eq!(declined, 0);
        assert_eq!(offsets.len(), 3);
        assert_eq!(
            offsets[0],
            InlineOffset::new(0).unwrap(),
            "`flush` starts the run exactly at the base's own start"
        );
        assert_eq!(
            offsets[2],
            InlineOffset::new(900).unwrap(),
            "0 + 300 + 150 + 300 + 150 == 900"
        );
        let base_end = InlineOffset::new(1200).unwrap();
        assert_eq!(
            offsets[2].units() + 300,
            base_end.units(),
            "the last character's own end lands exactly on the base's own end: `flush`'s \
             own ends-flush property holds exactly here, and holds under either remainder \
             rule, because every `flush` site (unlike `jis`'s own unconsumed trailing site) \
             is consumed by this run's own walk"
        );
        let gap_one = offsets[1].units() - offsets[0].units() - 300;
        let gap_two = offsets[2].units() - offsets[1].units() - 300;
        assert_eq!(
            gap_one, gap_two,
            "both interior gaps split the surplus equally"
        );
        assert_eq!(
            gap_one, 150,
            "300 units of surplus over two equal interior sites"
        );
    }

    #[test]
    fn group_ruby_jis_and_flush_agree_when_the_base_and_reading_are_equal_length() {
        // §3.3.6 paragraph 1: a 1000-unit base against two 500-unit ruby characters —
        // exactly the fixture the now-deleted `3.3.5/group-ruby-placement/produces-no-
        // attachment-and-is-not-declined` case once used to assert the opposite. Surplus is
        // zero, so every weight's own share is zero under both methods regardless of their
        // different weight shapes, and both place the run flush with the base's own start:
        // paragraph 1's own "set solid... center... aligned" falls out rather than being a
        // third branch (this module's own doc, §4.3).
        let base_items = [item(0, 1000)];
        let base_scales = [scale(1000)];
        let reading_items = ruby_items(2, 500);
        let reading_scales = [scale(500)];
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(2),
        )];
        let items = ItemIndex::new(0)..ItemIndex::new(1);
        let placements = [InlineOffset::new(0).unwrap()];

        for policy in [Policy::JLREQ, flush_policy()] {
            let (text, ruby) = group_ruby_fixture(
                "鬼",
                &base_items,
                &base_scales,
                "きき",
                &reading_items,
                &reading_scales,
                &runs,
            );
            let (offsets, declined) = place_group(text, ruby, items.clone(), &placements, policy);
            assert_eq!(declined, 0);
            assert_eq!(
                offsets,
                [
                    InlineOffset::new(0).unwrap(),
                    InlineOffset::new(500).unwrap()
                ],
                "{policy:?}: both methods place the run flush with the base's own start when \
                 the surplus is zero"
            );
        }
    }

    #[test]
    fn group_ruby_jis_degenerates_to_centered_with_one_ruby_character() {
        // §4.3's own degenerate case: `jis`'s weights for `n == 1` are `[1, 1]` (no interior
        // site at all), which is centering — the identical arithmetic
        // `nakatsuki_and_katatsuki_place_a_one_character_run_at_different_offsets`'s own
        // nakatsuki half already pins for mono-ruby, over the identical 1000-against-500
        // fixture: surplus 500, split into two 250-unit shares.
        let base_items = [item(0, 1000)];
        let base_scales = [scale(1000)];
        let reading_items = ruby_items(1, 500);
        let reading_scales = [scale(500)];
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(1),
        )];
        let (text, ruby) = group_ruby_fixture(
            "鬼",
            &base_items,
            &base_scales,
            "き",
            &reading_items,
            &reading_scales,
            &runs,
        );
        let items = ItemIndex::new(0)..ItemIndex::new(1);
        let placements = [InlineOffset::new(0).unwrap()];

        let (offsets, declined) = place_group(text, ruby, items, &placements, Policy::JLREQ);
        assert_eq!(declined, 0);
        assert_eq!(
            offsets,
            [InlineOffset::new(250).unwrap()],
            "500 of 1000 units centers with a 250-unit leading share, `jis`'s own [1, 1] \
             degenerate weights"
        );
    }

    #[test]
    fn group_ruby_flush_start_aligns_with_one_ruby_character() {
        // `docs/decisions/group-ruby-flush-single-character.md`'s own published reading: at
        // `n == 1`, `flush`'s own interior weights are empty (`count - 1 == 0`), so
        // `distribute` yields no shares at all and the run starts exactly at the base's own
        // start, with the 500-unit surplus applied nowhere — the identical fixture as the
        // `jis` test above, under `flush` instead, to show the two methods genuinely diverge
        // here rather than agreeing the way they do at `n == 2` (paragraph 1's own ratio).
        let base_items = [item(0, 1000)];
        let base_scales = [scale(1000)];
        let reading_items = ruby_items(1, 500);
        let reading_scales = [scale(500)];
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(1),
        )];
        let (text, ruby) = group_ruby_fixture(
            "鬼",
            &base_items,
            &base_scales,
            "き",
            &reading_items,
            &reading_scales,
            &runs,
        );
        let items = ItemIndex::new(0)..ItemIndex::new(1);
        let placements = [InlineOffset::new(0).unwrap()];

        let (offsets, declined) = place_group(text, ruby, items, &placements, flush_policy());
        assert_eq!(declined, 0);
        assert_eq!(
            offsets,
            [InlineOffset::new(0).unwrap()],
            "the leading clause is honored by construction and the surplus is applied \
             nowhere, `docs/decisions/group-ruby-flush-single-character.md`'s own reading"
        );
    }

    #[test]
    fn group_ruby_longer_than_its_base_is_declined_and_produces_no_attachment() {
        // §3.3.6 paragraph 3: a 1000-unit base against two 600-unit ruby characters (1200
        // total) is genuinely longer than its base — `base_extent.max(ruby_extent) !=
        // base_extent` — so this round declines rather than spreading the base apart
        // (this module's own doc, "What this round declines: §3.3.6 paragraph 3").
        let base_items = [item(0, 1000)];
        let base_scales = [scale(1000)];
        let reading_items = ruby_items(2, 600);
        let reading_scales = [scale(600)];
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(2),
        )];
        let (text, ruby) = group_ruby_fixture(
            "鬼",
            &base_items,
            &base_scales,
            "かか",
            &reading_items,
            &reading_scales,
            &runs,
        );
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);
        let items = ItemIndex::new(0)..ItemIndex::new(1);
        let placements = [InlineOffset::new(0).unwrap()];

        let mut lower_scratch = Lowered::new();
        let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut lower_scratch)
            .expect("a well-formed group-ruby lowers");
        let mut place_scratch = Lowered::new();
        let attachments = place(
            &constructs,
            &contribution,
            items,
            &placements,
            Policy::JLREQ,
            &mut place_scratch,
        );
        assert!(
            attachments.attachments().is_empty(),
            "a declined run produces no attachment"
        );
        let run = contribution
            .runs()
            .of(ItemIndex::new(0))
            .expect("the base item joined a run");
        let expected = contribution.construct_of(run.run());
        let declined: Vec<_> = attachments.declined().collect();
        assert_eq!(
            declined,
            [expected],
            "the declined construct is reported exactly once"
        );
    }

    #[test]
    fn a_group_ruby_run_straddling_the_items_range_before_its_own_start_is_skipped_entirely() {
        // Four base items; the group-ruby run's own base is items 1..3, but this call's
        // `items` is only 2..4 — the run's own start (1) lies *before* `items.start` (2).
        // Without the module's own whole-range guard, `first_index` would be computed as
        // `base.start.get().saturating_sub(items.start.get())`, i.e. `1_u32.saturating_sub(2)
        // == 0` — a silent wraparound that reads `placements[0]` (item 2's own placement,
        // 2000) as if it were item 1's own, rather than refusing to read one at all. The
        // guard this module's own "Indexing convention" doc states catches this before that
        // subtraction ever runs, so the run is skipped whole: no attachment, not declined.
        let base_items = [item(0, 400), item(3, 400), item(6, 400), item(9, 400)];
        let base_scales = [scale(400)];
        let text = Text::new("鬼門方角", &base_items, &base_scales).expect("four ideographs");
        let reading_items = ruby_items(2, 300);
        let reading_scales = [scale(300)];
        let annotation =
            Annotation::new("かか", &reading_items, &reading_scales).expect("two kana");
        let runs = [RubyRun::new(
            ItemIndex::new(1)..ItemIndex::new(3),
            AnnotationIndex::new(0)..AnnotationIndex::new(2),
        )];
        let ruby = Ruby::new(
            text,
            ItemIndex::new(1)..ItemIndex::new(3),
            annotation,
            &runs,
            RubyStyle::GroupRuby,
        )
        .expect("one run over items 1..3");
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);

        let mut lower_scratch = Lowered::new();
        let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut lower_scratch)
            .expect("a well-formed group-ruby lowers");

        let items = ItemIndex::new(2)..ItemIndex::new(4);
        let placements = [
            InlineOffset::new(2000).unwrap(),
            InlineOffset::new(2400).unwrap(),
        ];
        let mut place_scratch = Lowered::new();
        let attachments = place(
            &constructs,
            &contribution,
            items,
            &placements,
            Policy::JLREQ,
            &mut place_scratch,
        );
        assert!(
            attachments.attachments().is_empty(),
            "the run straddles `items`' own start and is skipped whole"
        );
        assert!(
            attachments.declined().next().is_none(),
            "a straddling run is skipped, not declined — this call never reasoned about its \
             geometry at all"
        );
    }

    /// One jukugo-ruby compound over two base items, one ruby character on the first and
    /// three on the second — §3.3.7¶2's own trigger, "any... character... which needs three
    /// or more ruby characters." Every §3.3.7 paragraph-2 test below builds its own
    /// `base_items`/`reading_items`/`runs` locals and hands them here, the identical
    /// caller-owns-the-arrays shape `group_ruby_fixture` above already establishes, so each
    /// test states only its own policy and its own expectation rather than the shared
    /// geometry.
    fn jukugo_paragraph_two_fixture<'r>(
        base_items: &'r [Item],
        base_scales: &'r [Scale],
        reading_items: &'r [Item],
        reading_scales: &'r [Scale],
        runs: &'r [RubyRun],
    ) -> (Text<'r>, Ruby<'r>) {
        let text = Text::new("鬼門", base_items, base_scales).expect("two ideographs");
        let annotation =
            Annotation::new("かかかか", reading_items, reading_scales).expect("four kana");
        let ruby = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(2),
            annotation,
            runs,
            RubyStyle::JukugoRuby,
        )
        .expect("one run per base item, the second carrying three ruby characters");
        (text, ruby)
    }

    /// The two-base-item, one-then-three-ruby-character `runs` [`jukugo_paragraph_two_fixture`]
    /// needs, shared by every call site the identical way `base_items`/`reading_items` are
    /// declared fresh at each one.
    fn jukugo_paragraph_two_runs() -> [RubyRun; 2] {
        [
            RubyRun::new(
                ItemIndex::new(0)..ItemIndex::new(1),
                AnnotationIndex::new(0)..AnnotationIndex::new(1),
            ),
            RubyRun::new(
                ItemIndex::new(1)..ItemIndex::new(2),
                AnnotationIndex::new(1)..AnnotationIndex::new(4),
            ),
        ]
    }

    #[test]
    fn jukugo_paragraph_one_delegates_each_base_run_to_mono_placement_under_both_alignments() {
        // Two base items, one ruby character each — §3.3.7¶1's own "two or less" — so this
        // is delegated per base to §3.3.5's method through `place_mono_run`, unmodified.
        // Each run's own 500-unit reading is not longer than its own 1000-unit base, so
        // katatsuki start-aligns rather than declining (the trap a run genuinely longer than
        // its base would spring instead).
        let base_items = [item(0, 1000), item(3, 1000)];
        let base_scales = [scale(1000)];
        let text = Text::new("鬼門", &base_items, &base_scales).expect("two ideographs");
        let reading_items = ruby_items(2, 500);
        let reading_scales = [scale(500)];
        let annotation = Annotation::new("きき", &reading_items, &reading_scales)
            .expect("two kana, one per base item");
        let runs = [
            RubyRun::new(
                ItemIndex::new(0)..ItemIndex::new(1),
                AnnotationIndex::new(0)..AnnotationIndex::new(1),
            ),
            RubyRun::new(
                ItemIndex::new(1)..ItemIndex::new(2),
                AnnotationIndex::new(1)..AnnotationIndex::new(2),
            ),
        ];
        let base_ruby = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(2),
            annotation,
            &runs,
            RubyStyle::JukugoRuby,
        )
        .expect("one run per base item, both two or fewer ruby characters");

        let items = ItemIndex::new(0)..ItemIndex::new(2);
        let placements = [
            InlineOffset::new(0).unwrap(),
            InlineOffset::new(1000).unwrap(),
        ];

        let mut offsets = Vec::new();
        for alignment in [RubyAlignment::Nakatsuki, RubyAlignment::Katatsuki] {
            let ruby = base_ruby.with_alignment(alignment);
            let declared = [ruby];
            let constructs = Constructs::over(text).with_ruby(&declared);
            let mut lower_scratch = Lowered::new();
            let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut lower_scratch)
                .expect("a well-formed jukugo-ruby lowers");
            let mut place_scratch = Lowered::new();
            let attachments = place(
                &constructs,
                &contribution,
                items.clone(),
                &placements,
                Policy::JLREQ,
                &mut place_scratch,
            );
            let placed = attachments.attachments();
            assert_eq!(placed.len(), 2, "one attachment per base character");
            offsets.push((placed[0].inline(), placed[1].inline()));
            assert!(
                attachments.declined().next().is_none(),
                "{alignment:?}: neither run is longer than its own base"
            );
        }

        assert_eq!(
            offsets[0],
            (
                InlineOffset::new(250).unwrap(),
                InlineOffset::new(1250).unwrap()
            ),
            "nakatsuki: each 500-of-1000-unit run centers with its own 250-unit leading share"
        );
        assert_eq!(
            offsets[1],
            (
                InlineOffset::new(0).unwrap(),
                InlineOffset::new(1000).unwrap()
            ),
            "katatsuki: each run start-aligns with its own base, no distribution at all"
        );
        assert_ne!(
            offsets[0], offsets[1],
            "the two alignments resolve to different offsets for the identical compound"
        );
    }

    #[test]
    fn jukugo_paragraph_two_group_answer_forces_jis_regardless_of_group_ruby_distribution() {
        // §3.3.7¶2's own `group` answer reuses §3.3.6's own `jis` geometry over the whole
        // compound (base 2000, reading 1600, surplus 400; `jis`'s own five-site weights
        // `[1,2,2,2,1]` sum to 8, and 400 divides 8 exactly, so the shares are exactly
        // [50,100,100,100,50] regardless of the remainder rule) — and forces it even under a
        // policy that answers `flush` for `Question::GROUP_RUBY_DISTRIBUTION`
        // (`decision:jukugo-group-layout-distribution`). A pass-through implementation that
        // read that question for a jukugo compound would start the run at 0 with three equal
        // ~133-unit interior gaps under `flush_policy()` instead — visibly different from the
        // offsets asserted below, which is what makes the forcing observable rather than
        // merely stated.
        let base_items = [item(0, 1000), item(3, 1000)];
        let base_scales = [scale(1000)];
        let reading_items = ruby_items(4, 400);
        let reading_scales = [scale(400)];
        let runs = jukugo_paragraph_two_runs();
        let (text, ruby) = jukugo_paragraph_two_fixture(
            &base_items,
            &base_scales,
            &reading_items,
            &reading_scales,
            &runs,
        );
        let items = ItemIndex::new(0)..ItemIndex::new(2);
        let placements = [
            InlineOffset::new(0).unwrap(),
            InlineOffset::new(1000).unwrap(),
        ];

        for policy in [Policy::JLREQ, flush_policy()] {
            let declared = [ruby];
            let constructs = Constructs::over(text).with_ruby(&declared);
            let mut lower_scratch = Lowered::new();
            let contribution = lower(&constructs, policy, horizontal(), &mut lower_scratch)
                .expect("a well-formed jukugo-ruby lowers");
            let mut place_scratch = Lowered::new();
            let attachments = place(
                &constructs,
                &contribution,
                items.clone(),
                &placements,
                policy,
                &mut place_scratch,
            );
            let offsets: Vec<_> = attachments
                .attachments()
                .iter()
                .map(|attachment| attachment.inline())
                .collect();
            assert!(
                attachments.declined().next().is_none(),
                "{policy:?}: not declined"
            );
            assert_eq!(
                offsets,
                [
                    InlineOffset::new(50).unwrap(),
                    InlineOffset::new(550).unwrap(),
                    InlineOffset::new(1050).unwrap(),
                    InlineOffset::new(1550).unwrap(),
                ],
                "{policy:?}: `jis`'s own geometry, forced regardless of \
                 `Question::GROUP_RUBY_DISTRIBUTION`'s own answer"
            );
        }
    }

    #[test]
    fn jukugo_paragraph_two_declines_under_the_phonetic_answer() {
        // The identical paragraph-2 compound as the `group`-answer test above, under
        // `Question::JUKUGO_RUBY_LAYOUT`'s own `phonetic` answer instead: §F's own
        // phonetic-structure distribution is not implemented this round, so this declines
        // rather than guessing at a geometry no code here computes.
        let base_items = [item(0, 1000), item(3, 1000)];
        let base_scales = [scale(1000)];
        let reading_items = ruby_items(4, 400);
        let reading_scales = [scale(400)];
        let runs = jukugo_paragraph_two_runs();
        let (text, ruby) = jukugo_paragraph_two_fixture(
            &base_items,
            &base_scales,
            &reading_items,
            &reading_scales,
            &runs,
        );
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);
        let items = ItemIndex::new(0)..ItemIndex::new(2);
        let placements = [
            InlineOffset::new(0).unwrap(),
            InlineOffset::new(1000).unwrap(),
        ];

        let mut lower_scratch = Lowered::new();
        let contribution = lower(
            &constructs,
            phonetic_policy(),
            horizontal(),
            &mut lower_scratch,
        )
        .expect("a well-formed jukugo-ruby lowers");
        let mut place_scratch = Lowered::new();
        let attachments = place(
            &constructs,
            &contribution,
            items,
            &placements,
            phonetic_policy(),
            &mut place_scratch,
        );
        assert!(
            attachments.attachments().is_empty(),
            "a declined compound produces no attachment"
        );
        let run = contribution
            .runs()
            .of(ItemIndex::new(0))
            .expect("the first base item joined the compound's own run");
        let expected = contribution.construct_of(run.run());
        let declined: Vec<_> = attachments.declined().collect();
        assert_eq!(
            declined,
            [expected],
            "the declined construct is reported exactly once"
        );
    }

    #[test]
    fn a_jukugo_compound_straddling_the_items_range_is_declined_not_skipped() {
        // The identical paragraph-2 compound again, but this call's own `items` covers only
        // the first base item (0..1), not the second (item 1) the compound's own reading
        // also needs — a split §C.2#8 permits (`docs/decisions/jukugo-ruby-unset-group.md`)
        // and `RubyStyle::GroupRuby`'s own base range structurally cannot reach (this
        // module's own doc, "Indexing convention"). §3.3.7¶2's own "as a whole" instruction
        // has no whole left to attach once the line has split the compound, so this declines
        // rather than silently skipping the way an ordinary out-of-range group-ruby run does.
        let base_items = [item(0, 1000), item(3, 1000)];
        let base_scales = [scale(1000)];
        let reading_items = ruby_items(4, 400);
        let reading_scales = [scale(400)];
        let runs = jukugo_paragraph_two_runs();
        let (text, ruby) = jukugo_paragraph_two_fixture(
            &base_items,
            &base_scales,
            &reading_items,
            &reading_scales,
            &runs,
        );
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);

        let mut lower_scratch = Lowered::new();
        let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut lower_scratch)
            .expect("a well-formed jukugo-ruby lowers");

        let items = ItemIndex::new(0)..ItemIndex::new(1);
        let placements = [InlineOffset::new(0).unwrap()];
        let mut place_scratch = Lowered::new();
        let attachments = place(
            &constructs,
            &contribution,
            items,
            &placements,
            Policy::JLREQ,
            &mut place_scratch,
        );
        assert!(
            attachments.attachments().is_empty(),
            "a straddled compound produces no attachment"
        );
        let run = contribution
            .runs()
            .of(ItemIndex::new(0))
            .expect("the first base item joined the compound's own run");
        let expected = contribution.construct_of(run.run());
        let declined: Vec<_> = attachments.declined().collect();
        assert_eq!(
            declined,
            [expected],
            "the straddled compound is declined, not silently skipped"
        );
    }
}
