// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! §3.6 tab setting: [`TabKind`], [`TabStop`], [`TabLine`], [`tab_line`].
//!
//! §3.6.1's own words state what tab setting is for: "Tab setting is useful for alignment
//! of table data, itemized lists, etc. where a series of characters need to be set at
//! specific alignment positions within a line." Its own second paragraph states the
//! inputs, precisely enough to build a vocabulary from:
//!
//! > For tab setting, it is necessary to identify tab positions, tab types (how to align
//! > the characters in the tab position), and the characters to be set. For this purpose,
//! > it is necessary to insert a tab sign before the tabbed character. The series of
//! > characters just after the tab sign are the target characters... If there is more than
//! > one tab sign, it is necessary to set the same numbers of tab positions and tab types
//! > as the number of tab signs.
//!
//! Three declarations, one target string apiece: a tab position ([`TabStop::position`]), a
//! tab type ([`TabStop::kind`], [`TabKind`]), and where the target string itself begins
//! ([`tab_line`]'s own `starts`). §3.6.1's Note also records that "Tab Setting is described
//! in 'JIS X 4051 4.21 Tab Setting'" — a cross-reference this crate does not chase, the
//! same discipline every other §3 section's own JIS X 4051 pointer already gets.
//!
//! `crates/jlreq-line/src/lib.rs`'s own `# Status`, before this round, named this gap
//! honestly rather than fabricating coverage: "aligning a run against a caller-declared tab
//! position needs a vocabulary for stating that position, and this milestone has no type
//! for one, so there is nothing yet for a slot to hold or a wire to reach." This module is
//! that vocabulary and that placement logic, landing in the same milestone `crate::align`
//! did — a second line-level function, not a mode of [`crate::compose::compose`] (see this
//! module's own `# What this is not`, below).
//!
//! # §3.6.2's four kinds, named direction-neutral
//!
//! > There are the following types of tab setting to align texts.
//!
//! - **Start alignment** ([`TabKind::Start`]): "the start position of the text is aligned
//!   to the tab position." §3.6.2's own Japanese names this twice, once per axis — 左そろえ
//!   タブ (start-alignment tab) "is the tab type for horizontal writing, and" 上そろえタブ
//!   (also start-alignment) "is the tab type for vertical writing" — which is exactly the
//!   pattern [`jlreq_unit::Direction`]'s own doc names for the three rules that genuinely
//!   read the direction (ADR-0004): one rule, stated twice for two axes. [`TabKind`]
//!   collapses the pair the way [`crate::Alignment`] already collapses `LineHead`/`LineEnd`
//!   into direction-neutral names, for the identical reason.
//! - **End alignment** ([`TabKind::End`]): "the end position of the text is aligned to the
//!   tab position" (右そろえタブ / 下そろえタブ, the same horizontal/vertical pair).
//! - **Center alignment** ([`TabKind::Centered`]): "the center of the text is aligned to
//!   the tab position" (中央そろえタブ, stated once — §3.6.2 does not pair this one by
//!   axis, because a center has no direction-dependent name to collapse).
//! - **Alignment with a specified character** ([`TabKind::Character`]): "the start position
//!   of a specified character or sign (for example, a period) in the text is aligned to the
//!   tab position." §3.6.2's own text leaves open what "specified" means as a declaration
//!   and what happens when the named character occurs zero or several times — a genuine
//!   silence this module reads rather than guesses at, published in full at
//!   `docs/decisions/specified-character-tab-alignment.md` (see `# The specified-character
//!   kind`, below, for the reading in brief).
//!
//! `spec/derived/rules.tsv` marks 3.6.1, 3.6.2 and 3.6.3 `direction_conditional = false`,
//! so the `direction` gate's own equality check (the union of rules `docs/direction-sites.
//! toml` names must equal the set the inventory marks direction-conditional) is exactly why
//! [`TabKind`] carries no `Direction`-branching logic anywhere in this module: the four
//! kinds above are the specification's own collapse of a direction-doubled rule into one
//! name, not a fifth rule this module invents to read the direction itself.
//!
//! # §3.6.3: the placement algorithm
//!
//! > Set the text from the line head to the position before the tab sign in the first tab
//! > position, set the text from the first tab sign to the next tab sign in the second tab
//! > position, and so on. The behavior of opening brackets (cl-01) and closing brackets
//! > (cl-02), etc. is same as for the main text.
//!
//! Then four numbered behaviors, with its own warning that trial and error, not a closed
//! formula, is how a real document's tab positions get designed:
//!
//! > Following are some examples. The behavior of text before and after the tab positions
//! > are very difficult to anticipate, so it is necessary to design using trial and error.
//!
//! - **(a)** "If the target string is the first series of the line, the characters should
//!   be set in the first tab position from the start of the line, and so on, one after
//!   another." [`tab_line`]'s own forward cursor over `stops`, consuming one declared stop
//!   per run absent any overflow, is exactly this: run *k* nominally takes the next
//!   unconsumed stop, which is stop *k* when nothing before it ran long.
//! - **(b)** "If the target string of text is too long to be set before the next tab
//!   position and overflows, the next string of text is aligned to the tab position after
//!   the end of the preceding string." A run whose own placed extent reaches past a later
//!   stop's own position makes that stop unusable for the run that would nominally have
//!   taken it; [`tab_line`]'s cursor skips it and keeps searching `stops`, in declaration
//!   order, for the first one whose position does not fall short of where the previous run
//!   ended (`crate::compose::fits`, reused rather than re-derived).
//! - **(c)** "If the beginning of the string overlaps with the end of the preceding string
//!   as the result of the tab setting indication, the following string is set just after
//!   the preceding string." A stop the search finds satisfies "not before the previous run
//!   ends" by construction for [`TabKind::Start`] (its own anchor *is* the found stop), but
//!   [`TabKind::End`], [`TabKind::Centered`] and [`TabKind::Character`] pull their own
//!   anchor *back* from the found stop by the run's own natural width (or half of it, or
//!   the distance to the named item) — and that pulled-back anchor can still fall short of
//!   where the previous run ended even though the stop it was pulled back from did not.
//!   [`tab_line`]'s own clamp (`crate::compose::overflow` against the running floor, zero
//!   exactly when nothing is owed) is this sentence: fall back to "just after the preceding
//!   string" whenever the kind's own arithmetic would have overlapped it.
//! - **(d)** "If there is no tab position corresponding to the target string, the string
//!   should be set from the tab position of the next line, and so forth." The forward
//!   cursor over `stops` can run out — every remaining declared stop already consumed or
//!   skipped past by an earlier run's own overflow — and when it does, this run and every
//!   run named after it ("and so forth") get no home on this line at all. See `# Declared
//!   shortage is an input error; a search that comes up empty is not`, below, for why this
//!   is reported through [`TabLine::deferred`] rather than an [`Err`].
//!
//! Figures 160 through 169 are images only, and none of the four behaviors above needed
//! one to become a decidable rule: where this module's own algorithm had to choose between
//! two readings a figure alone would have settled, that choice is written down as a
//! decision below rather than guessed from a picture this crate does not have.
//!
//! # Declared shortage is an input error; a search that comes up empty is not
//!
//! §3.6.1 states the caller's own precondition in so many words: "the same numbers of tab
//! positions and tab types as the number of tab signs." [`tab_line`] checks it once, at the
//! call itself — `stops.len() < starts.len()` is `Err(ComposeError::InsufficientTabStops)`
//! — because a caller who declares fewer stops than target strings has not stated a
//! well-formed tab-setting problem at all; there is nothing for §3.6.3's own algorithm to
//! read a placement out of. `docs/api-frozen.toml` names no closed answer set for
//! [`ComposeError`] (unlike a `[[closed_choices]]` question), so ADR-0012 permits a new
//! variant on the already-`#[non_exhaustive]` enum, and this is exactly the shape
//! [`crate::compose::ComposeError::OutOfRange`] and
//! [`crate::compose::ComposeError::CandidateOutOfRange`] already take: an input the caller
//! could have gotten right and did not.
//!
//! §3.6.3(d) is a different state, reached only *during* placement rather than *before* it:
//! `stops.len() >= starts.len()` can still be true at the call and yet leave a later run
//! with nothing, because (b)'s own overflow can consume more than the one stop a run's own
//! nominal position would have used, silently narrowing the supply for every run after it.
//! That is not a caller mistake — it is §3.6.3's own stated, ordinary outcome, worded to
//! expect it ("and so forth" is not the language of an error) — so it is reported the same
//! way `crate::Feasible::rejected` and `crate::Composition::violations` already report a
//! consequence rather than hide one: as data in the success value, [`TabLine::deferred`],
//! not as an [`Err`] a caller has to unwind a whole call to observe. A caller retries a
//! deferred run as the first target of its own next `tab_line` call, over the next line's
//! own declared stops, which is exactly what "set... from the tab position of the next
//! line" states.
//!
//! # Tab-stop order is read, not required
//!
//! JLReq never states that a document's own declared tab positions are sorted, distinct, or
//! inside any measure — §3.6.3(b) and (c)'s own "the tab position after the end of the
//! preceding string" presupposes an ascending sequence to search forward through, but never
//! says the caller's own declaration must be one. [`tab_line`] neither refuses nor
//! normalizes an out-of-order `stops` slice: its own forward search reads `stops` by
//! declaration-order index, one candidate at a time, and asks only whether *that*
//! candidate's own position clears the running floor — a question well-defined for any
//! order, since nothing about it assumes the *next* index carries a larger position than
//! the one before it. A caller who declares stops out of the ascending order §3.6.3's own
//! prose assumes gets whatever placement that search finds — visually surprising, perhaps,
//! but never a panic, a silent guess, or a refused call — which is the accept reading among
//! the three this decision could have taken (refuse an unsorted slice; silently re-sort it;
//! accept it and let the same search primitive answer for any order). Refusing would add a
//! validation pass this module's own algorithm has no other use for; re-sorting would
//! silently discard the caller's own declared order, which §3.6.1 never states is
//! insignificant. Accepting costs nothing beyond stating, here, that the specification's own
//! ascending assumption is the intended use, not an enforced one.
//!
//! # The specified-character kind
//!
//! [`TabKind::Character`] names the occurrence by [`jlreq_unit::ItemIndex`] — `{ at:
//! ItemIndex }`, an ordinal the caller states directly — rather than by a bare `char`
//! kumihan would search the run's own text for. The full argument, and the two questions
//! this reading answers without inventing an answer to either (what a zero-occurrence or a
//! several-occurrence search would do) rather than by construction, is published at
//! `docs/decisions/specified-character-tab-alignment.md`, per this project's own convention
//! that a reading of a specification silence is written there rather than only in a doc
//! comment (`docs/decisions/README.md`). In brief: the caller already knows exactly which
//! occurrence they mean — the same knowledge that let them place a tab sign before the
//! target string in the first place (§3.6.1) — so asking them to name it directly removes
//! the zero/many question rather than answering it by search order, and is the only one of
//! the two readings ADR-0018's occurrence model and Appendix A's 25 code-point-*sequence*
//! keys both survive: a `char` cannot name a key that is more than one code point.
//!
//! # Interior runs are neither a line head nor a line end
//!
//! Each target run gets its own [`crate::compose::geometry_of`] call — that function
//! already takes an item sub-range, which is the primitive that makes per-run tab placement
//! one call each rather than a new geometry engine — and the naive shape of that call would
//! give **every** run the same line-head and line-end treatment §3.1.2 and §B.2 reserve for
//! a paragraph's own two edges, because [`crate::compose::geometry_of`] used to assume both
//! of its own edges were always genuine ones. §3.6.3's own text is explicit that neither
//! reservation belongs to an interior tab run: only the run this module places *first* on a
//! line opens it (行頭), and only the run it places *last* closes it (行末) — every run
//! between sits at a tab position, which JLReq never calls either. A `crate::compose::
//! geometry_of` line-head miscalculation was already found and fixed once in this crate,
//! for `compose`'s own per-line calls (the regression test beside it, in `compose.rs`, is
//! its own record); this module is a second site the identical mistake reaches, and
//! `crate::compose::Edges` — a `head`/`end` pair this round added to `geometry_of`'s own
//! signature, replacing its former unconditional assumption — is what closes it here.
//! `an_interior_tabbed_run_is_not_treated_as_a_line_end`, below, proves the trailing edge
//! behaviorally: §B.2's own rules (most visibly, the trailing half em after a line-ending
//! full stop) must not apply where more text follows across a tab, and the test fails
//! without `Edges::new(.., false)` there to suppress them. The leading edge cannot be
//! proven the identical way: Table 1's own line-head row (`before: 0`) carries `terms: &[]`
//! in every one of its 29 cells, so "this position is a genuine line head" and "this
//! position consults no adjacency at all" are numerically indistinguishable outcomes for
//! *any* following class, and no fixture built only on that comparison could ever fail
//! either way — the correctness this module wants there (§3.6.3's own second sentence: no
//! mojikumi rule crosses a tab sign) is structurally encoded by `range_head_adjacency`
//! reading `edges.head` (`crate::compose`'s own doc) rather than independently observable
//! from outside it. `an_interior_tabbed_runs_leading_edge_does_not_cross_the_tab_gap`,
//! below, instead catches a real, different, third failure mode on that same edge: an
//! interior-boundary lookup that reads the *actual* item closing the previous run across
//! the tab gap (a genuine, non-blank Table 1 cell, cl-02×cl-19 in that fixture) rather than
//! either of the two edge treatments above — a mistake `edges.head` alone would not have
//! caught, since it lives in the `else` branch `geometry_of`'s own per-item loop takes for
//! every non-head position, not the branch `edges.head` guards.
//!
//! [`tab_line`] passes `Edges::new(true, false)` to its own first placed run, `Edges::new(
//! false, false)` to every interior run, and re-derives the true last placed run's own
//! *anchor* and geometry a second time with `Edges::new(head, true)` once the whole line's
//! own layout is known (`crate::compose::Edges`'s own doc states why both booleans are
//! independent; [`place_targets`]'s own doc states why the anchor, not only the geometry,
//! needs the redo — [`TabKind::End`] and [`TabKind::Centered`] read the run's own natural
//! *extent*, which a genuine line end's own trailing space changes).
//!
//! §3.6.3's own second sentence — "the behavior of opening brackets (cl-01) and closing
//! brackets (cl-02), etc. is same as for the main text" — is the positive half of the same
//! point: a run's own *interior* spacing (between two of its own items) is ordinary
//! main-text mojikumi, unaffected by anything this module does, because each run's own
//! `geometry_of` call reads Table 1 and Table 2 across its own interior boundaries exactly
//! as `compose` does across a line's. What the sentence does not extend to is the *tab gap
//! between two runs*: `crate::compose::Edges`'s own `false` case is never routed through
//! [`jlreq_spacing::boundary`] against whatever item precedes the run — an interior run's
//! own leading edge consults no adjacency at all, rather than one read across the gap,
//! because §3.6.3 states no mojikumi rule crossing a tab sign and inventing one (even one
//! that happens to answer the same as a genuine line head, since Table 1's own line-head
//! row is blank in every cell) would be reading a rule into a boundary the specification
//! never describes as a class-pair adjacency in the first place.
//!
//! # What this is not
//!
//! [`tab_line`] is a second line-level function beside [`crate::align::align`], not a mode
//! of [`crate::compose::compose`]: no call in this module ever builds a
//! [`crate::Ladder`], drains a reduction or expansion stage, or offers hanging punctuation,
//! the same absence `crate::align`'s own `# What this is not` states for its own four
//! methods and for the identical reason (§3.6 names no adjustment process of its own for a
//! tab-placed run to undergo). [`crate::compose::compose`] does not call into this module
//! either, and this module does not call into it: composing a paragraph that itself
//! contains tab-set material — deciding *where* a line breaks around one — is outside this
//! round's own scope, an explicitly undeclared boundary rather than a silent one.
//!
//! §3.6.1, §3.6.2 and §3.6.3 are now each exercised by a published conformance case
//! (`crates/jlreq-conform/cases/3.6.1.json`, `3.6.2.json`, `3.6.3.json`), authored the way
//! ADR-0006 requires: as a phase of its own, independent of this module, against
//! `crates/jlreq-conform/cases.schema.json`'s own `tab` reading of `input.kind` — a fifth
//! question beside `classify`, `boundary`, `compose` and `align`, added for exactly this
//! purpose the same way an earlier round added `align` before the align cases existed. Two
//! shapes this module can reach are deliberately left unasserted rather than silently
//! counted as covered: `Err(ComposeError::InsufficientTabStops)` (§3.6.1's own
//! declared-count precondition) has no case demanding it, because the published case format
//! still has no field for stating that a call is expected to be refused
//! (`docs/design/conformance.md`'s own unmet `expect.refused` bullet); and
//! [`TabLine::deferred`] (§3.6.3(d)) has no case asserting it either, because `CaseOutput`
//! carries no channel for it, a decision made on purpose rather than by oversight
//! (`docs/conformance-deferrals.toml`'s own `3.6.3` entry states the exclusion in full).
//! `docs/conformance-deferrals.toml`'s own entries for 3.6.1, 3.6.2 and 3.6.3 record all
//! three rules as owned, with both exclusions stated by name, rather than deferred for lack
//! of any case at all.

use alloc::vec::Vec;
use core::ops::Range;

use jlreq_class::Text;
use jlreq_spec::Policy;
use jlreq_unit::{Direction, InlineExtent, InlineOffset, ItemIndex, Runs, distribute};

use crate::align::one;
use crate::compose::{
    ComposeError, Edges, Geometry, Line, byte_range, fits, geometry_of, overflow,
};
use crate::ladder::Adjustment;
use crate::objective::Demerits;

/// One of §3.6.2's four tab types, named direction-neutral the way [`crate::Alignment`]'s
/// own methods are — see this module's own `# §3.6.2's four kinds, named direction-neutral`
/// for why the specification's own horizontal/vertical pairs collapse to one name apiece.
///
/// JLReq: §3.6.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TabKind {
    /// The run's own start is aligned to the tab position (左（上）そろえタブ).
    ///
    /// JLReq: §3.6.2
    Start,
    /// The run's own end is aligned to the tab position (右（下）そろえタブ).
    ///
    /// JLReq: §3.6.2
    End,
    /// The run's own center is aligned to the tab position (中央そろえタブ), split by
    /// [`jlreq_unit::distribute`] the identical way [`crate::Alignment::Centered`] splits a
    /// residual into two shares.
    ///
    /// JLReq: §3.6.2
    Centered,
    /// The start of the named item is aligned to the tab position (指定文字そろえタブ).
    /// `at` is an ordinal into the same running-text stream `tab_line`'s own `text`
    /// argument indexes, not a byte offset and not a bare `char` — see this module's own
    /// `# The specified-character kind` and `docs/decisions/specified-character-tab-
    /// alignment.md` for why.
    ///
    /// JLReq: §3.6.2
    Character {
        /// Which occurrence, by the caller's own naming.
        at: ItemIndex,
    },
}

/// One declared tab position and the kind of alignment the run that follows its own tab
/// sign uses there (§3.6.1's own "it is necessary to identify tab positions, tab types...
/// and the characters to be set").
///
/// JLReq: §3.6.1, §3.6.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TabStop {
    position: InlineExtent,
    kind: TabKind,
}

impl TabStop {
    /// Declare one tab position and its own alignment kind.
    ///
    /// JLReq: §3.6.1, §3.6.2
    #[must_use]
    pub const fn new(position: InlineExtent, kind: TabKind) -> Self {
        Self { position, kind }
    }

    /// Where this stop sits.
    ///
    /// JLReq: §3.6.1
    #[must_use]
    pub const fn position(self) -> InlineExtent {
        self.position
    }

    /// How a run aligns to it.
    ///
    /// JLReq: §3.6.2
    #[must_use]
    pub const fn kind(self) -> TabKind {
        self.kind
    }
}

/// The result of placing as many of the caller's own target runs on one line as its
/// declared tab stops allow (§3.6.3).
///
/// JLReq: §3.6.3
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TabLine {
    placed: Vec<Line>,
    deferred: Vec<ItemIndex>,
}

impl TabLine {
    /// The runs this line placed, in the caller's own `starts` order.
    ///
    /// JLReq: §3.6.3
    #[must_use]
    pub fn placed(&self) -> &[Line] {
        &self.placed
    }

    /// §3.6.3(d): every run this line found no tab position for, named by its own
    /// `starts` entry — "the string should be set from the tab position of the next
    /// line", which a caller reads as its own next `tab_line` call's first target.
    ///
    /// JLReq: §3.6.3
    #[must_use]
    pub fn deferred(&self) -> &[ItemIndex] {
        &self.deferred
    }
}

/// Place §3.6's target runs on one line: `starts[k]` is where the run after the `k`-th tab
/// sign begins (§3.6.1's own "the series of characters just after the tab sign are the
/// target characters"), and the run itself extends to `starts[k + 1]`, or to the end of
/// `text` for the last one. `stops` is the caller's own declared pool, read in declaration
/// order (see this module's own `# Tab-stop order is read, not required`).
///
/// `starts` itself is assumed ascending rather than merely read in whatever order it comes,
/// the way `stops` is: §3.6.1's own tab signs are inserted one after another along the same
/// running text (`starts[k] < starts[k + 1]` is what "the series of characters just after
/// the tab sign" means for a well-formed declaration), so unlike `stops` there is no reading
/// under which a caller's own out-of-order `starts` is a deliberate choice this function
/// accepts. This function checks `starts` only for range (`ComposeError::OutOfRange`), not
/// for order: a descending pair leaves `range = starts[k]..starts[k + 1]` inverted for that
/// run, which `geometry_of`'s own `range.start.get()..range.end.get()` reads as the empty
/// range Rust's own convention gives a backwards bound, and `byte_range` likewise returns
/// an inverted `Range<ByteOffset>` rather than an error — a caller mistake this function does
/// not yet catch, named here rather than silently assumed away.
///
/// `Err(ComposeError::InsufficientTabStops)` when `stops` is shorter than `starts` —
/// §3.6.1's own declared-count precondition, checked once before any placement is
/// attempted. `Err(ComposeError::OutOfRange)` when a `starts` entry, or a
/// [`TabKind::Character`]'s own `at`, names no item of `text`. Neither error is §3.6.3(d):
/// see this module's own `# Declared shortage is an input error; a search that comes up
/// empty is not`.
///
/// JLReq: §3.6.1, §3.6.2, §3.6.3
pub fn tab_line(
    text: Text<'_>,
    runs: Runs<'_>,
    starts: &[ItemIndex],
    stops: &[TabStop],
    direction: Direction,
    policy: Policy,
) -> Result<TabLine, ComposeError> {
    if stops.len() < starts.len() {
        return Err(ComposeError::InsufficientTabStops {
            targets: u32::try_from(starts.len()).unwrap_or(u32::MAX),
            stops: u32::try_from(stops.len()).unwrap_or(u32::MAX),
        });
    }
    let item_count = u32::try_from(text.items().len()).unwrap_or(u32::MAX);
    for &start in starts {
        if start.get() > item_count {
            return Err(ComposeError::OutOfRange { at: start });
        }
    }

    let (seated, deferred) = place_targets(text, runs, starts, stops, direction, policy)?;
    let placed = finalize_lines(text, seated);
    Ok(TabLine { placed, deferred })
}

/// One run this line has decided to place: its own item span, the declared stop it seated
/// at, whether it is this line's own first placed run, and the indent and geometry that
/// resulted. Every run but the line's own last placed run is final as soon as it is pushed;
/// the last one is provisional here (built under `Edges::new(head, false)`, because whether
/// this run is also the line's own *last* placed run — deserving `edges.end = true`, and an
/// anchor recomputed under it — is not known until every run has been walked) and
/// [`place_targets`] overwrites it in place once the walk finishes, rather than leaving that
/// correction to a later pass: see this module's own `# Interior runs are neither a line
/// head nor a line end` for why an anchor computed under the wrong edge would misplace
/// [`TabKind::End`] and [`TabKind::Centered`] specifically.
struct Placed {
    range: Range<ItemIndex>,
    stop: TabStop,
    indent: InlineExtent,
    head: bool,
    geometry: Geometry,
}

/// Walk `starts` in order, seating each run at the first remaining stop whose own position
/// does not fall short of the running floor (§3.6.3(a)/(b)), until either every run is
/// seated or the search comes up empty (§3.6.3(d)) — split out of [`tab_line`] to keep both
/// functions under `clippy::too_many_lines`.
///
/// The loop itself anchors and measures every run under `Edges::new(head, false)`: whether
/// a run is this line's own *last* placed run is not decided until the walk either exhausts
/// `starts` or runs out of stops, so no run can know its own true `edges.end` while the loop
/// is still seating the ones after it. Once the walk ends, the run at `seated`'s own last
/// index — unambiguously this line's own last placed run, `starts` order preserved and a
/// mid-walk deferral changing only how many runs got that far, never which one is last among
/// them — has both its anchor and its geometry redone under `Edges::new(head, true)`, against
/// the same floor its provisional placement above already used (`floor_before_last`,
/// captured immediately before that run's own `floor` update, never touched again): this is
/// the same §3.6.3(c) overlap clamp the loop already applies, just re-run against the
/// anchor §B.2's own genuine-line-end rules actually produce rather than the one a
/// not-yet-known `edges.end` guessed. [`TabKind::Start`]'s own anchor does not depend on
/// `edges` at all (`candidate_indent`), so a redo changes nothing for it; [`TabKind::
/// Character`]'s own anchor reads an item *placement*, which `geometry_of` computes before
/// its own `edges.end` branch ever runs, so a redo cannot change that either — only
/// [`TabKind::End`] and [`TabKind::Centered`], whose own anchor is pulled back by the run's
/// own *extent*, are the ones a genuine line end's trailing space moves.
fn place_targets(
    text: Text<'_>,
    runs: Runs<'_>,
    starts: &[ItemIndex],
    stops: &[TabStop],
    direction: Direction,
    policy: Policy,
) -> Result<(Vec<Placed>, Vec<ItemIndex>), ComposeError> {
    let item_count = u32::try_from(text.items().len()).unwrap_or(u32::MAX);
    let mut floor = InlineExtent::ZERO;
    let mut floor_before_last = InlineExtent::ZERO;
    let mut cursor = 0usize;
    let mut seated: Vec<Placed> = Vec::with_capacity(starts.len());
    let mut deferred = Vec::new();

    for (run_index, &start) in starts.iter().enumerate() {
        let end = starts
            .get(run_index.saturating_add(1))
            .copied()
            .unwrap_or(ItemIndex::new(item_count));
        let range = start..end;

        let Some(offset) = stops[cursor..]
            .iter()
            .position(|stop| fits(floor, stop.position()))
        else {
            // §3.6.3(d): the declared pool is exhausted. "And so forth" — every run named
            // after this one shares the same fate, because the cursor never rewinds and a
            // later run's own nominal stop is already behind it in `stops`.
            deferred.push(start);
            deferred.extend(starts[run_index.saturating_add(1)..].iter().copied());
            break;
        };
        cursor = cursor.saturating_add(offset).saturating_add(1);
        let stop = stops[cursor.saturating_sub(1)];

        let head = run_index == 0;
        let edges = Edges::new(head, false);
        let candidate =
            candidate_indent(text, runs, direction, policy, range.clone(), edges, stop)?;
        // §3.6.3(c): fall back to the running floor whenever the kind's own arithmetic
        // pulled the anchor back far enough to overlap the preceding run.
        let indent = if overflow(floor, candidate) == InlineExtent::ZERO {
            candidate
        } else {
            floor
        };
        let geometry = geometry_of(text, runs, range.clone(), indent, direction, policy, edges);
        floor_before_last = floor;
        floor = geometry.extent;
        seated.push(Placed {
            range,
            stop,
            indent,
            head,
            geometry,
        });
    }

    // The run at `seated`'s own last index — this line's own true last placed run, however
    // many runs after it were deferred — was anchored and measured above under
    // `edges.end = false`, before it was knowable whether this run would turn out to be the
    // one closing the line. Redo both now that it is, against the floor that run's own
    // original placement used (`floor_before_last`, frozen before the loop's own final
    // `floor` update, so this redo answers exactly the §3.6.3(c) question the loop already
    // asked for this run, not a new one against a floor a later — deferred — run never
    // reached).
    if let Some(last) = seated.last_mut() {
        let edges = Edges::new(last.head, true);
        let candidate = candidate_indent(
            text,
            runs,
            direction,
            policy,
            last.range.clone(),
            edges,
            last.stop,
        )?;
        let indent = if overflow(floor_before_last, candidate) == InlineExtent::ZERO {
            candidate
        } else {
            floor_before_last
        };
        last.indent = indent;
        last.geometry = geometry_of(
            text,
            runs,
            last.range.clone(),
            indent,
            direction,
            policy,
            edges,
        );
    }

    Ok((seated, deferred))
}

/// Where [`candidate_indent`]'s own per-kind arithmetic would place `stop` against `range`,
/// before §3.6.3(c)'s overlap clamp: `stop.position()` itself for [`TabKind::Start`], and
/// `stop.position()` pulled back by the run's own natural width, half of it, or the
/// distance to a named item's own natural placement for the other three — the identical
/// natural-then-final shape [`crate::align::align`]'s own `LineEnd` and `Centered` methods
/// take, over the same [`geometry_of`] primitive. `edges` is this function's own caller's
/// responsibility to get right, not this function's: [`TabKind::End`] and [`TabKind::
/// Centered`] read `natural.extent`, which `edges.end` changes by a genuine line end's own
/// trailing space, so an anchor computed under the wrong `edges.end` is a wrong anchor, not
/// a merely provisional one — [`place_targets`]'s own second pass exists because of exactly
/// this (this module's own `# Interior runs are neither a line head nor a line end`).
/// [`TabKind::Character`] reads a *placement*, not the extent, so it alone is invariant
/// under `edges.end` regardless of which value its own caller passes.
///
/// `Err(ComposeError::OutOfRange)` when [`TabKind::Character`]'s own `at` names no item of
/// `range` — the one placement question this function's own caller cannot answer for it,
/// because only the kind itself knows which item it means.
fn candidate_indent(
    text: Text<'_>,
    runs: Runs<'_>,
    direction: Direction,
    policy: Policy,
    range: Range<ItemIndex>,
    edges: Edges,
    stop: TabStop,
) -> Result<InlineExtent, ComposeError> {
    match stop.kind() {
        TabKind::Start => Ok(stop.position()),
        TabKind::End => {
            let natural = geometry_of(
                text,
                runs,
                range,
                InlineExtent::ZERO,
                direction,
                policy,
                edges,
            );
            Ok(stop.position().sub_sat(natural.extent))
        },
        TabKind::Centered => {
            let natural = geometry_of(
                text,
                runs,
                range,
                InlineExtent::ZERO,
                direction,
                policy,
                edges,
            );
            let two_equal_sites = [one(), one()];
            let mut shares = distribute(natural.extent, &two_equal_sites, policy.remainder());
            let half = shares.next().unwrap_or(InlineExtent::ZERO);
            Ok(stop.position().sub_sat(half))
        },
        TabKind::Character { at } => {
            if at.get() < range.start.get() || at.get() >= range.end.get() {
                return Err(ComposeError::OutOfRange { at });
            }
            let local = at.get().saturating_sub(range.start.get()) as usize;
            let natural = geometry_of(
                text,
                runs,
                range,
                InlineExtent::ZERO,
                direction,
                policy,
                edges,
            );
            // No entry at `local` (defensive only: the bounds check above already confirms
            // `local < range.end.get() - range.start.get()`, and `geometry_of`'s own
            // per-item loop pushes exactly one placement per item it does not break out of
            // early over — the same silent-truncation behavior every other caller of
            // `geometry_of`, including `compose`'s own, already accepts, not a new
            // possibility this function invents) leaves `placement` at zero, which
            // `distance_to` and the `sub_sat` below already treat the same as "no shift" —
            // the answer a caller reading zero back out cannot distinguish from a genuine
            // zero-offset item, but that ambiguity is `geometry_of`'s own, not manufactured
            // here.
            let placement = natural
                .placements
                .get(local)
                .copied()
                .unwrap_or(InlineOffset::ZERO);
            Ok(stop.position().sub_sat(distance_to(placement)))
        },
    }
}

/// An item's own natural placement, read back as an extent so it can be subtracted from a
/// caller-declared tab position (§3.6.2's specified-character kind): [`InlineOffset`] and
/// [`InlineExtent`] share no arithmetic (ADR-0011), so this crosses the untyped channel
/// once — the reviewed entry `docs/scalar-sites.toml` names for this item — the same shape
/// `crate::compose::shift_by` takes in the other direction.
fn distance_to(offset: InlineOffset) -> InlineExtent {
    InlineExtent::new(offset.units()).unwrap_or(InlineExtent::ZERO)
}

/// Turn every seated run into a [`Line`]: [`place_targets`] has already given every run —
/// including the one that turns out to be this line's own last placed run, redone there
/// under `edges.end = true` before returning — its own final indent and geometry, so §B.2's
/// own line-end rules (most visibly, the trailing half em after a line-ending full stop or
/// comma) already apply to the run that genuinely closes this printed line and to no other
/// by the time this function ever sees it (`crate::compose::Edges`'s own doc; this module's
/// own `# Interior runs are neither a line head nor a line end`); this function only
/// packages that already-final geometry into a [`Line`], reading no field but `range`,
/// `geometry` and the byte span the two of them address together.
///
/// Every returned [`Line`] passes `is_last: true` to [`Line::from_geometry`], the same
/// blanket choice [`crate::align::align`] already makes for the identical reason: no line
/// this module ever produces is drained by [`crate::Ladder`] (`Adjustment::empty()`,
/// `hanging: None`), so [`Line::is_last`]'s own specific meaning — §3.8.1's Note that a
/// paragraph's own last line is exempt from expansion — never applies to a value this
/// function returns, and reusing the field to also mean "the run that closes this tab
/// line" would give it a second, unrelated meaning a caller reading its own doc would not
/// find there. Whether a run closes the line is instead exactly what [`place_targets`]'s
/// own `edges.end` already decided before this function ever runs.
fn finalize_lines(text: Text<'_>, placed: Vec<Placed>) -> Vec<Line> {
    let mut lines = Vec::with_capacity(placed.len());

    for run in placed {
        let bytes = byte_range(text, run.range.start, run.range.end);
        lines.push(Line::from_geometry(
            run.range,
            bytes,
            run.geometry,
            Demerits::ZERO,
            true,
            Adjustment::empty(),
            None,
        ));
    }

    lines
}

#[cfg(test)]
mod tests {
    use jlreq_class::Text;
    use jlreq_spec::Policy;
    use jlreq_unit::{
        Advance, ByteOffset, Direction, Frame, InlineExtent, InlineOffset, Item, ItemIndex, Runs,
        Scale, ScaleId,
    };

    use super::{TabKind, TabStop, tab_line};
    use crate::ComposeError;

    fn scale(units: i32) -> Scale {
        let em = Advance::new(units).expect("a positive advance");
        Scale::new(em, em).expect("a positive scale")
    }

    fn offset(units: i32) -> InlineOffset {
        InlineOffset::new(units).expect("a valid offset")
    }

    fn extent(units: i32) -> InlineExtent {
        InlineExtent::new(units).expect("a valid extent")
    }

    /// Regression for this round's own correctness trap: an interior tabbed run's own
    /// *trailing* edge must not pick up §B.2's line-end rules, the failure mode this
    /// round's own brief names as demonstrated rather than hypothetical. The fixture pairs
    /// two runs on one line — "亜。" (an ideograph then a full stop) first, "亜" second —
    /// with a start-alignment stop for each. §B.2 note 6 adds a half-em (500-unit) trailing
    /// space after a line-ending full stop; the first run's own two items sum to 1000 + 500
    /// = 1500 units before any such space, so a wrongly line-end-treated interior run would
    /// report 2000 instead — exactly the difference
    /// `crate::compose::tests::edges_end_false_suppresses_the_line_end_trailing_space`
    /// isolates at the `geometry_of` level; this test is the same fact observed through
    /// `tab_line`'s own public surface.
    #[test]
    fn an_interior_tabbed_run_is_not_treated_as_a_line_end() {
        let items = [
            Item::new(ByteOffset::new(0), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
            Item::new(ByteOffset::new(3), extent(500), ScaleId::new(0)).with_frame(Frame::HalfEm),
            Item::new(ByteOffset::new(6), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("亜。亜", &items, &scales).expect("a well formed stream");
        let starts = [ItemIndex::new(0), ItemIndex::new(2)];
        let stops = [
            TabStop::new(InlineExtent::ZERO, TabKind::Start),
            TabStop::new(extent(2500), TabKind::Start),
        ];

        let line = tab_line(
            text,
            Runs::none(),
            &starts,
            &stops,
            Direction::Horizontal,
            Policy::JLREQ,
        )
        .expect("a well formed declaration");

        assert!(
            line.deferred().is_empty(),
            "both stops are declared, in order, past the floor"
        );
        let placed = line.placed();
        assert_eq!(placed.len(), 2, "one line per declared stop, both seated");
        assert_eq!(
            placed[0].extent(),
            extent(1500),
            "the buggy reading applies §B.2 note 6's own trailing half em to the run before \
             the tab and reports 2000 — a line-end space this run never earned, because \
             `亜` follows it across the second stop rather than a genuine line end"
        );
    }

    /// The complementary half of this round's own correctness trap, on the *leading* edge:
    /// an interior tabbed run's own first item must not be read against whatever item
    /// closed the *previous* run, across the tab gap, the same "boundary lookup across a
    /// tab" §3.6.3's own second sentence forbids. Unlike the trailing-edge case above, a
    /// naive `Adjacency::at_line_head` substitute cannot fail this test on its own — Table
    /// 1's own line-head row is blank in every cell, so treating this item as a genuine
    /// line head and treating it as "no adjacency at all" are numerically identical — so
    /// this fixture instead reuses the discriminating cl-02×cl-19 pattern
    /// `compose::tests::geometry_of_reads_a_lines_own_head_as_the_line_head_not_the_previous_lines_close`
    /// already established: a closing bracket (」, cl-02, half-em frame) closes the first
    /// run, and an ideograph (cl-19) opens the second, across the tab. Table 1's own
    /// interior cl-02×cl-19 cell carries a real half-em gap; a `tab_line` that wrongly
    /// consulted it across the tab would push the second run's own placement to 1500
    /// instead of the declared stop, 1000.
    #[test]
    fn an_interior_tabbed_runs_leading_edge_does_not_cross_the_tab_gap() {
        let items = [
            Item::new(ByteOffset::new(0), extent(500), ScaleId::new(0)).with_frame(Frame::HalfEm),
            Item::new(ByteOffset::new(3), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("」亜", &items, &scales).expect("a well formed stream");
        let starts = [ItemIndex::new(0), ItemIndex::new(1)];
        let stops = [
            TabStop::new(InlineExtent::ZERO, TabKind::Start),
            TabStop::new(extent(1000), TabKind::Start),
        ];

        let line = tab_line(
            text,
            Runs::none(),
            &starts,
            &stops,
            Direction::Horizontal,
            Policy::JLREQ,
        )
        .expect("a well formed declaration");

        assert_eq!(
            line.placed()[1].placements(),
            &[offset(1000)],
            "the buggy reading, consulting the interior cl-02×cl-19 cell across the tab \
             instead of reading no adjacency at all, pushes this item's own placement to \
             1500 by the closing bracket's own half-em space — a boundary this run does \
             not sit beside, because a tab, not an interior adjacency, separates the two"
        );
    }

    /// §3.6.3(a): the ordinary case, one run per declared stop, in order.
    #[test]
    fn each_run_seats_at_its_own_nominal_stop_absent_overflow() {
        let items = [
            Item::new(ByteOffset::new(0), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
            Item::new(ByteOffset::new(3), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("亜亜", &items, &scales).expect("a well formed stream");
        let starts = [ItemIndex::new(0), ItemIndex::new(1)];
        let stops = [
            TabStop::new(InlineExtent::ZERO, TabKind::Start),
            TabStop::new(extent(3000), TabKind::Start),
        ];

        let line = tab_line(
            text,
            Runs::none(),
            &starts,
            &stops,
            Direction::Horizontal,
            Policy::JLREQ,
        )
        .expect("a well formed declaration");

        let placed = line.placed();
        assert_eq!(placed[0].placements(), &[offset(0)]);
        assert_eq!(placed[1].placements(), &[offset(3000)]);
    }

    /// §3.6.3(b): the first run's own extent overtakes the second declared stop, so the
    /// search that stop's own run would nominally have used skips past it — wasted — and
    /// seats at the third declared stop instead, the first one "after the end of the
    /// preceding string" in the search's own declaration order.
    #[test]
    fn an_overflowing_run_makes_the_search_skip_a_stop_too_early_to_use() {
        let items = [
            Item::new(ByteOffset::new(0), extent(2000), ScaleId::new(0)).with_frame(Frame::FullEm),
            Item::new(ByteOffset::new(3), extent(500), ScaleId::new(0)).with_frame(Frame::FullEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("亜亜", &items, &scales).expect("a well formed stream");
        let starts = [ItemIndex::new(0), ItemIndex::new(1)];
        // The first run alone (2000 units) already overtakes the second stop's own
        // position (1500); the third (2500) is the first remaining stop that does not.
        let stops = [
            TabStop::new(InlineExtent::ZERO, TabKind::Start),
            TabStop::new(extent(1500), TabKind::Start),
            TabStop::new(extent(2500), TabKind::Start),
        ];

        let line = tab_line(
            text,
            Runs::none(),
            &starts,
            &stops,
            Direction::Horizontal,
            Policy::JLREQ,
        )
        .expect("a well formed declaration");

        let placed = line.placed();
        assert_eq!(
            placed[1].placements(),
            &[offset(2500)],
            "seated at the third declared stop (2500), the first one the search finds \
             past the first run's own end (2000) — the second stop (1500) is skipped \
             entirely, wasted by the first run's own overflow past it"
        );
    }

    /// §3.6.3(c): the search finds a stop whose own position clears the running floor, but
    /// `TabKind::End` pulls the run's own anchor *back* from that stop by the run's own
    /// natural width, and the pulled-back anchor overlaps the preceding run — so the
    /// overlap clamp sets the run just after the preceding string instead, solid, rather
    /// than the overlapping position the kind's own arithmetic alone would have chosen.
    #[test]
    fn an_end_aligned_runs_pulled_back_anchor_is_clamped_to_the_preceding_runs_end() {
        let items = [
            Item::new(ByteOffset::new(0), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
            Item::new(ByteOffset::new(3), extent(500), ScaleId::new(0)).with_frame(Frame::FullEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("亜亜", &items, &scales).expect("a well formed stream");
        let starts = [ItemIndex::new(0), ItemIndex::new(1)];
        // The second stop (1200) itself clears the floor the first run leaves (1000), so
        // the search finds it directly — but `End` alignment there would pull the second
        // run's own 500-unit-wide anchor back to 700, short of that floor.
        let stops = [
            TabStop::new(InlineExtent::ZERO, TabKind::Start),
            TabStop::new(extent(1200), TabKind::End),
        ];

        let line = tab_line(
            text,
            Runs::none(),
            &starts,
            &stops,
            Direction::Horizontal,
            Policy::JLREQ,
        )
        .expect("a well formed declaration");

        let placed = line.placed();
        assert_eq!(
            placed[1].placements(),
            &[offset(1000)],
            "clamped to the first run's own end (1000) — 'just after the preceding \
             string' — rather than the overlapping 700 `End` alignment's own unclamped \
             arithmetic (1200 minus the run's own 500-unit width) would have chosen"
        );
    }

    /// §3.6.3(d): the search comes up empty for a later run once an earlier one has
    /// consumed every remaining declared stop, and that run — and every one named after it
    /// — is deferred rather than silently dropped or placed off the declared list.
    #[test]
    fn a_run_with_no_remaining_stop_is_deferred_and_so_forth() {
        let items = [
            Item::new(ByteOffset::new(0), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
            Item::new(ByteOffset::new(3), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
            Item::new(ByteOffset::new(6), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("亜亜亜", &items, &scales).expect("a well formed stream");
        let starts = [ItemIndex::new(0), ItemIndex::new(1), ItemIndex::new(2)];
        // Three stops for three target runs is a well formed declaration
        // (`stops.len() >= starts.len()`), so §3.6.1's own precondition is not what makes
        // the third run homeless — its own arithmetic is: run one seats at stop 0 (0),
        // ending at 1000 (the floor run two's own search reads). Run two's own search
        // finds stop 1 (1000, which clears that floor exactly) and seats there, ending at
        // 2000. Run three's own search then has only stop 2 (1200) left to try, and 1200
        // does not clear a floor of 2000 — no stop remains that does.
        let stops = [
            TabStop::new(InlineExtent::ZERO, TabKind::Start),
            TabStop::new(extent(1000), TabKind::Start),
            TabStop::new(extent(1200), TabKind::Start),
        ];

        let line = tab_line(
            text,
            Runs::none(),
            &starts,
            &stops,
            Direction::Horizontal,
            Policy::JLREQ,
        )
        .expect("a well formed declaration");

        assert_eq!(line.placed().len(), 2, "the first two runs seat normally");
        assert_eq!(
            line.deferred(),
            &[ItemIndex::new(2)],
            "the third run's own start is deferred: no remaining declared stop clears \
             the floor the first two runs left behind"
        );
    }

    /// §3.6.1's own declared-count precondition: fewer stops than target runs is refused
    /// at the call itself, distinct from §3.6.3(d)'s own runtime shortage above.
    #[test]
    fn fewer_stops_than_targets_is_refused_before_any_placement() {
        let items =
            [Item::new(ByteOffset::new(0), extent(1000), ScaleId::new(0))
                .with_frame(Frame::FullEm)];
        let scales = [scale(1000)];
        let text = Text::new("亜", &items, &scales).expect("a well formed stream");
        let starts = [ItemIndex::new(0), ItemIndex::new(1)];
        let stops = [TabStop::new(InlineExtent::ZERO, TabKind::Start)];

        let error = tab_line(
            text,
            Runs::none(),
            &starts,
            &stops,
            Direction::Horizontal,
            Policy::JLREQ,
        )
        .expect_err("one stop for two target runs violates §3.6.1's own precondition");

        assert_eq!(
            error,
            ComposeError::InsufficientTabStops {
                targets: 2,
                stops: 1
            },
            "the caller declared fewer tab stops than target runs"
        );
    }

    /// §3.6.2's specified-character kind: the run's own end-alignment counterpart against
    /// an item named directly, proving `TabKind::Character` actually shifts the run so the
    /// *named* item — not the run's own start — lands on the stop.
    #[test]
    fn character_alignment_seats_the_named_item_at_the_stop() {
        let items = [
            Item::new(ByteOffset::new(0), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
            Item::new(ByteOffset::new(3), extent(500), ScaleId::new(0)).with_frame(Frame::HalfEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("亜。", &items, &scales).expect("a well formed stream");
        let starts = [ItemIndex::new(0)];
        let stops = [TabStop::new(
            extent(2000),
            TabKind::Character {
                at: ItemIndex::new(1),
            },
        )];

        let line = tab_line(
            text,
            Runs::none(),
            &starts,
            &stops,
            Direction::Horizontal,
            Policy::JLREQ,
        )
        .expect("a well formed declaration");

        assert_eq!(
            line.placed()[0].placements(),
            &[offset(1000), offset(2000)],
            "the named item (the full stop, item 1) sits exactly at the declared stop \
             (2000); the run's own start is pushed back by the same amount (1000)"
        );
    }

    /// Regression for this round's own second correctness trap, found in review: an
    /// `End`-aligned run's own anchor must be pulled back by its *true* natural width — the
    /// one §B.2's own trailing half em after a line-ending full stop or comma is already
    /// part of, once this run turns out to be the tab line's own genuine last placed run —
    /// not by the narrower, provisional width [`place_targets`] computes before that is
    /// known. Getting this wrong does not lose the trailing space; it *keeps* it but anchors
    /// the run as though the space did not exist, so the run's own true end (`Line::extent`,
    /// which includes trailing per `docs/adr/0017-normalized-line-geometry.md`) overshoots
    /// the declared stop by exactly the trailing amount — 3500 against a declared 3000 was
    /// the failure this fixture demonstrated before the fix, one run alone on the line
    /// ("亜。": an ideograph then a line-ending full stop), `End`-aligned to a stop
    /// (3000) chosen deliberately past where the run's own narrower, pre-trailing natural
    /// width (1500) would have placed it without clamping — the one shape that lets the two
    /// candidate anchors (0, from the narrower width; 1000, from the true one) actually
    /// differ instead of both saturating to the same clamped floor.
    #[test]
    fn an_end_aligned_last_runs_anchor_accounts_for_its_own_trailing_space() {
        let items = [
            Item::new(ByteOffset::new(0), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
            Item::new(ByteOffset::new(3), extent(500), ScaleId::new(0)).with_frame(Frame::HalfEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("亜。", &items, &scales).expect("a well formed stream");
        let starts = [ItemIndex::new(0)];
        let stops = [TabStop::new(extent(3000), TabKind::End)];

        let line = tab_line(
            text,
            Runs::none(),
            &starts,
            &stops,
            Direction::Horizontal,
            Policy::JLREQ,
        )
        .expect("a well formed declaration");

        let placed = line.placed();
        assert_eq!(
            placed[0].placements(),
            &[offset(1000), offset(2000)],
            "the anchor (1000) pulls the run back by its own true natural width (2000, \
             trailing half em included) from the declared stop (3000) — not by the \
             narrower, pre-trailing width (1500) the provisional pass alone would have used"
        );
        assert_eq!(
            placed[0].extent(),
            extent(3000),
            "the run's own true end, trailing space included, lands exactly on the \
             declared stop — the buggy anchor left this at 3500, overshooting by the \
             trailing half em a wrongly-narrow anchor never accounted for"
        );
    }

    /// The redo above is not the only place a wrong floor could leak in: once the last
    /// run's own anchor and geometry are redone, the §3.6.3(c) clamp guarding that redo
    /// must fall back to `floor_before_last` — the floor the *preceding* run actually left
    /// (1000, "亜" alone, `Start`-aligned to 0) — not to zero or to any value this run's own
    /// provisional pass computed. Two runs, second (last) `End`-aligned to 2200: its true
    /// natural width (2000, trailing half em on the line-ending full stop included) pulls
    /// the candidate anchor back to 200, short of the floor either way, so both the
    /// pre-redo and post-redo candidates clamp — this fixture is not about which one wins,
    /// it is about which floor the clamp reads, and 1000 (not 0) is the only floor that
    /// produces the placements asserted below.
    #[test]
    fn the_last_runs_redone_clamp_reads_the_preceding_runs_own_floor() {
        let items = [
            Item::new(ByteOffset::new(0), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
            Item::new(ByteOffset::new(3), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
            Item::new(ByteOffset::new(6), extent(500), ScaleId::new(0)).with_frame(Frame::HalfEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("亜亜。", &items, &scales).expect("a well formed stream");
        let starts = [ItemIndex::new(0), ItemIndex::new(1)];
        let stops = [
            TabStop::new(InlineExtent::ZERO, TabKind::Start),
            TabStop::new(extent(2200), TabKind::End),
        ];

        let line = tab_line(
            text,
            Runs::none(),
            &starts,
            &stops,
            Direction::Horizontal,
            Policy::JLREQ,
        )
        .expect("a well formed declaration");

        let placed = line.placed();
        assert_eq!(
            placed[0].placements(),
            &[offset(0)],
            "the first run, unaffected"
        );
        assert_eq!(
            placed[1].placements(),
            &[offset(1000), offset(2000)],
            "clamped to 1000 — the first run's own end, the floor this redo must read — \
             not to 0, which a redo consulting the wrong floor would have produced instead"
        );
    }

    /// `TabKind::Character` naming an item outside the run it governs is refused rather
    /// than silently clamped or guessed at.
    #[test]
    fn character_alignment_naming_an_item_outside_the_run_is_refused() {
        let items = [
            Item::new(ByteOffset::new(0), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
            Item::new(ByteOffset::new(3), extent(1000), ScaleId::new(0)).with_frame(Frame::FullEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("亜亜", &items, &scales).expect("a well formed stream");
        // Two declared starts, so the first stop's own run is item 0 alone (bounded by the
        // second run's own start at item 1) — naming item 1, the *other* run's own item,
        // is outside it.
        let starts = [ItemIndex::new(0), ItemIndex::new(1)];
        let stops = [
            TabStop::new(
                extent(1000),
                TabKind::Character {
                    at: ItemIndex::new(1),
                },
            ),
            TabStop::new(extent(2000), TabKind::Start),
        ];

        let error = tab_line(
            text,
            Runs::none(),
            &starts,
            &stops,
            Direction::Horizontal,
            Policy::JLREQ,
        )
        .expect_err("the named item belongs to the run after this one, not this one");

        assert_eq!(
            error,
            ComposeError::OutOfRange {
                at: ItemIndex::new(1)
            }
        );
    }
}
