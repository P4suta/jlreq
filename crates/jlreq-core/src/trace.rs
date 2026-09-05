// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Why the composer reached the layout it reached.
//!
//! A [`Layout`](crate::Layout) states the answer and a [`Diagnostic`](crate::Diagnostic)
//! states the few conditions a program is expected to branch on. Neither states the work:
//! the break candidates that were weighed and rejected, the mojikumi (文字組み) table cell
//! that produced an amount, the rung of the adjustment ladder that absorbed a line's
//! surplus. That intermediate state is discarded when the layout is built, and it is
//! exactly what a reader needs when the output is legal but unexpected.
//!
//! A [`Trace`] records it. Recording is opted into per call — [`compose`](crate::compose)
//! and [`Composer::compose`](crate::Composer::compose) run the same code with a sink that
//! is off — so there is one implementation rather than a traced one and a plain one.
//!
//! The trace is not part of the compatibility contract. Event shapes, their order, and
//! their rendering may change in any release; `docs/adr/0028` states why that is the right
//! trade for this channel and why an event is not a second carrier of a diagnostic's fact.

use alloc::vec::Vec;
use core::ops::Range;

mod render;

use crate::model::WritingMode;
use crate::paragraph::Alignment;

/// Which families of decision a [`Trace`] records.
///
/// The two quadratic families — every considered break pair, and every placed cluster —
/// are outside [`Categories::DEFAULT`], because a paragraph large enough to be worth
/// investigating produces more of them than anyone can read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Categories(u32);

impl Categories {
    /// Record nothing.
    pub const NONE: Self = Self(0x0000);
    /// Paragraph-wide preparation.
    pub const PREPARE: Self = Self(0x0001);
    /// The outcome of the line-breaking search.
    pub const SEARCH: Self = Self(0x0002);
    /// Every break pair the search weighed. Quadratic in the candidate count.
    pub const SEARCH_CANDIDATES: Self = Self(0x0004);
    /// Kinsoku (禁則) legality at a boundary.
    pub const KINSOKU: Self = Self(0x0008);
    /// Mojikumi (文字組み) spacing lookups.
    pub const SPACING: Self = Self(0x0010);
    /// The reduction half of the adjustment ladder.
    pub const REDUCE: Self = Self(0x0020);
    /// The expansion half of the adjustment ladder.
    pub const EXPAND: Self = Self(0x0040);
    /// Hanging punctuation (ぶら下げ, burasage).
    pub const HANGING: Self = Self(0x0080);
    /// Ruby (ルビ) distribution and overhang.
    pub const RUBY: Self = Self(0x0100);
    /// Warichu (割注), furawake (振分け), jidori (字取り), and tate-chu-yoko (縦中横).
    pub const STRUCTURE: Self = Self(0x0200);
    /// Line geometry as it is finished.
    pub const PLACE: Self = Self(0x0400);
    /// Every placed cluster. Linear in the cluster count.
    pub const PLACE_CLUSTERS: Self = Self(0x0800);

    /// Every family whose event count stays proportional to lines times sites.
    pub const DEFAULT: Self = Self(0x07fb);
    /// Every family, including the two quadratic ones.
    pub const ALL: Self = Self(0x0fff);

    /// The union of two sets.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// This set without the members of `other`.
    #[must_use]
    pub const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// Whether every member of `other` is a member of this set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether this set has no members.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The set as its bit pattern, for rendering and for round-tripping a selection.
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }
}

/// Which stage of composition is speaking.
///
/// The ladder and ruby helpers are reached from three places: preparation builds
/// paragraph-wide prefix sums and calls them with a boundary of zero for every cluster,
/// the search calls them to measure a candidate line, and placement calls them for the
/// line it is actually setting. Only the last has the frame of reference a site event
/// claims, so the phase decides which families may speak.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Phase {
    #[default]
    Prepare,
    Search,
    Placement,
}

/// Where in the caller's input a decision belongs.
///
/// Cluster ordinals index the shaped-text slice; bytes are UTF-8 offsets into the
/// paragraph's own source, as every core position is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Site {
    pub(crate) line: Option<u32>,
    pub(crate) clusters: Range<usize>,
    pub(crate) bytes: Range<usize>,
}

impl Site {
    /// The line ordinal this decision belongs to, or None outside line placement.
    #[must_use]
    pub const fn line(&self) -> Option<u32> {
        self.line
    }

    /// The shaped-text cluster ordinals the decision covers.
    #[must_use]
    pub fn clusters(&self) -> Range<usize> {
        self.clusters.clone()
    }

    /// The source UTF-8 byte range the decision covers.
    #[must_use]
    pub fn bytes(&self) -> Range<usize> {
        self.bytes.clone()
    }

    pub(crate) const fn paragraph(clusters: Range<usize>, bytes: Range<usize>) -> Self {
        Self {
            line: None,
            clusters,
            bytes,
        }
    }

    pub(crate) const fn on_line(line: u32, clusters: Range<usize>, bytes: Range<usize>) -> Self {
        Self {
            line: Some(line),
            clusters,
            bytes,
        }
    }
}

/// A JLReq 2020 rule address in the grammar `docs/adr/0013` fixes.
///
/// A section is written `3.8.3`, an appendix note `C.2#5`, and a table cell
/// `B.1@cl-05,cl-05`. This is the same inventory [`Diagnostic::jlreq`](crate::Diagnostic)
/// names; the enum exists because a trace addresses cells, which no shipped diagnostic
/// does, and a reader should not have to parse a string to tell the three apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RuleAddress {
    /// A numbered section of the specification body.
    Section(&'static str),
    /// A numbered note under an appendix heading.
    Note(&'static str, u16),
    /// One cell of an appendix matrix, keyed by the class pair that selects it.
    Cell(&'static str, u8, u8),
}

/// One recorded decision.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Fact {
    /// The paragraph's shape, as composition begins.
    ParagraphPrepared {
        /// Shaped clusters in the paragraph.
        clusters: usize,
        /// Break candidates the search may use.
        candidates: usize,
        /// Typed inline structures lowered into the paragraph.
        constructs: usize,
        /// Whether every cluster is ordinary enough for the indexed fast measurement.
        fast_measure: bool,
        /// The measure, in caller units.
        line_extent: i32,
        /// The paragraph's writing mode.
        writing_mode: WritingMode,
        /// The paragraph's alignment.
        alignment: Alignment,
    },
    /// One break pair the search weighed, and what it cost.
    ///
    /// The natural and the reduced width are both here, so the reduction capacity the
    /// search assumed for this line is their difference. That is why the ladder stays
    /// quiet while the search runs: it would be restating a subtraction.
    SearchCandidate {
        /// The break candidate the trial line starts at.
        start_candidate: usize,
        /// The break candidate the trial line ends at.
        end_candidate: usize,
        /// The line's width before any reduction.
        natural_width: i64,
        /// The line's width once the available reduction is spent.
        reduced_width: i64,
        /// The measure.
        available: i64,
        /// Measure minus reduced width; negative when the line does not fit.
        delta: i64,
        /// The badness of that difference.
        badness: i64,
        /// The surcharge for breaking at an author's discretionary opportunity.
        discretionary: i64,
        /// The surcharge for breaking inside a warichu (割注).
        warichu: i64,
        /// The surcharge for breaking inside a formula.
        formula: i64,
        /// The surcharge for leaving too little on the final line.
        widow: i64,
        /// The total charged for this line alone.
        edge_cost: u128,
        /// The best known cost of reaching the end of this trial line.
        total_cost: u128,
        /// Whether this trial line would end the paragraph.
        is_last: bool,
        /// Whether this pair became the best known way to reach its end.
        accepted: bool,
    },
    /// A candidate the search skipped without weighing, because kinsoku (禁則) refuses it.
    SearchCandidateRefused {
        /// The break candidate.
        candidate: usize,
    },
    /// The search stopped extending a line leftward because no earlier start can win.
    SearchBoundStop {
        /// The break candidate the abandoned trial line starts at.
        start_candidate: usize,
        /// The break candidate the trial line ends at.
        end_candidate: usize,
        /// The narrowest this line could be made.
        minimum_width: i64,
        /// The measure.
        available: i64,
        /// The best known cost of reaching this end.
        best_cost: u128,
    },
    /// The search was refused before it finished, having charged its whole budget.
    SearchRefused {
        /// What the search had charged when it was refused.
        charged: usize,
        /// The budget it was held to.
        limit: usize,
    },
    /// One line the search settled on.
    LineChosen {
        /// The line ordinal, counting from zero.
        line: u32,
        /// The break candidate the line starts at.
        start_candidate: usize,
        /// The break candidate the line ends at.
        end_candidate: usize,
        /// The cost the search charged for this line alone.
        edge_cost: u128,
    },
}

impl Fact {
    /// The stable dotted name of this kind of decision.
    ///
    /// The namespace is deliberately disjoint from the product error-code namespaces in
    /// `docs/error-codes.md`, so a trace name can never be read as a compatibility key.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match *self {
            Self::ParagraphPrepared { .. } => "prepare.paragraph",
            Self::SearchCandidate { .. } => "search.candidate",
            Self::SearchCandidateRefused { .. } => "search.refused-candidate",
            Self::SearchBoundStop { .. } => "search.bound-stop",
            Self::SearchRefused { .. } => "search.refused",
            Self::LineChosen { .. } => "search.chosen",
        }
    }

    /// The family this decision belongs to.
    #[must_use]
    pub const fn category(&self) -> Categories {
        match *self {
            Self::ParagraphPrepared { .. } => Categories::PREPARE,
            Self::SearchCandidate { .. } | Self::SearchBoundStop { .. } => {
                Categories::SEARCH_CANDIDATES
            },
            Self::SearchCandidateRefused { .. } => Categories::KINSOKU,
            Self::SearchRefused { .. } | Self::LineChosen { .. } => Categories::SEARCH,
        }
    }

    /// The rule that states this decision, where one does.
    ///
    /// Bookkeeping about the engine's own work has no address, and says so rather than
    /// borrowing a nearby one.
    #[must_use]
    pub const fn jlreq(&self) -> Option<RuleAddress> {
        match *self {
            Self::ParagraphPrepared { .. } | Self::SearchRefused { .. } => None,
            Self::SearchCandidate { .. } | Self::SearchBoundStop { .. } => {
                Some(RuleAddress::Section("3.8.1"))
            },
            Self::SearchCandidateRefused { .. } => Some(RuleAddress::Section("3.1.9")),
            Self::LineChosen { .. } => Some(RuleAddress::Section("3.1.1")),
        }
    }
}

/// One decision and where it belongs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Event {
    pub(crate) site: Site,
    pub(crate) fact: Fact,
}

impl Event {
    /// Where in the caller's input this decision belongs.
    #[must_use]
    pub const fn site(&self) -> &Site {
        &self.site
    }

    /// The decision itself.
    #[must_use]
    pub const fn fact(&self) -> &Fact {
        &self.fact
    }

    /// The stable dotted name of this kind of decision.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        self.fact.kind()
    }

    /// The family this decision belongs to.
    #[must_use]
    pub const fn category(&self) -> Categories {
        self.fact.category()
    }

    /// The rule that states this decision, where one does.
    #[must_use]
    pub const fn jlreq(&self) -> Option<RuleAddress> {
        self.fact.jlreq()
    }
}

/// A recorded sequence of composition decisions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Trace {
    events: Vec<Event>,
    categories: Categories,
    limit: usize,
    phase: Phase,
    truncated: bool,
}

impl Trace {
    /// The default event ceiling.
    ///
    /// Large enough for a chapter, small enough that a pathological paragraph cannot turn
    /// a debugging aid into an allocation failure.
    pub const DEFAULT_MAX_EVENTS: usize = 100_000;

    /// A trace recording [`Categories::DEFAULT`].
    #[must_use]
    pub const fn new() -> Self {
        Self::with_categories(Categories::DEFAULT)
    }

    /// A trace recording exactly the named families.
    #[must_use]
    pub const fn with_categories(categories: Categories) -> Self {
        Self {
            events: Vec::new(),
            categories,
            limit: Self::DEFAULT_MAX_EVENTS,
            phase: Phase::Prepare,
            truncated: false,
        }
    }

    /// The families this trace records.
    #[must_use]
    pub const fn categories(&self) -> Categories {
        self.categories
    }

    /// Record a different set of families from here on.
    pub const fn set_categories(&mut self, categories: Categories) {
        self.categories = categories;
    }

    /// The event ceiling.
    #[must_use]
    pub const fn max_events(&self) -> usize {
        self.limit
    }

    /// Set the event ceiling.
    pub const fn set_max_events(&mut self, limit: usize) {
        self.limit = limit;
    }

    /// The recorded decisions, in the order the composer reached them.
    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Whether the ceiling was reached and later decisions were dropped.
    #[must_use]
    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }

    /// Empty the buffer, keeping the configuration and the allocation.
    pub fn clear(&mut self) {
        self.events.clear();
        self.truncated = false;
        self.phase = Phase::Prepare;
    }

    /// Take the recorded decisions, leaving an empty configured trace behind.
    #[must_use]
    pub fn take_events(&mut self) -> Vec<Event> {
        self.truncated = false;
        core::mem::take(&mut self.events)
    }

    /// A sink that records nothing and allocates nothing.
    pub(crate) const fn off() -> Self {
        Self {
            events: Vec::new(),
            categories: Categories::NONE,
            limit: 0,
            phase: Phase::Prepare,
            truncated: false,
        }
    }

    /// The one branch an instrumentation site pays when tracing is off.
    pub(crate) const fn wants(&self, category: Categories) -> bool {
        self.categories.contains(category) && self.phase_admits(category)
    }

    pub(crate) fn push(&mut self, site: Site, fact: Fact) {
        if self.events.len() >= self.limit {
            self.truncated = true;
            return;
        }
        self.events.push(Event { site, fact });
    }

    pub(crate) const fn enter(&mut self, phase: Phase) {
        self.phase = phase;
    }

    /// Outside placement the ladder helpers measure rather than set, so a site event from
    /// them would name a boundary that is not a line's. The search's own candidate events
    /// carry both the natural and the reduced width, so the capacity the search assumed
    /// remains readable as their difference.
    const fn phase_admits(&self, category: Categories) -> bool {
        match self.phase {
            // Kinsoku legality is settled per boundary while the indexes are built, and a
            // boundary is what it names, so preparation may speak for it. The ladder may
            // not: preparation reaches those helpers with a boundary of zero for every
            // cluster, because it is summing a paragraph rather than setting a line.
            Phase::Prepare => {
                category.contains(Categories::PREPARE) || category.contains(Categories::KINSOKU)
            },
            Phase::Search => {
                category.contains(Categories::SEARCH)
                    || category.contains(Categories::SEARCH_CANDIDATES)
                    || category.contains(Categories::KINSOKU)
            },
            Phase::Placement => true,
        }
    }
}

impl Default for Trace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Categories, Fact, Phase, Site, Trace};
    use crate::model::WritingMode;
    use crate::paragraph::Alignment;

    fn prepared() -> Fact {
        Fact::ParagraphPrepared {
            clusters: 3,
            candidates: 2,
            constructs: 0,
            fast_measure: true,
            line_extent: 4_000,
            writing_mode: WritingMode::HorizontalTb,
            alignment: Alignment::Justify,
        }
    }

    fn chosen() -> Fact {
        Fact::LineChosen {
            line: 0,
            start_candidate: 0,
            end_candidate: 2,
            edge_cost: 17,
        }
    }

    /// One instance of every variant, so a new one cannot be added without being named
    /// here, in `kind`, in `category`, and in `jlreq`.
    fn every_fact() -> [Fact; 6] {
        [
            prepared(),
            Fact::SearchCandidate {
                start_candidate: 0,
                end_candidate: 2,
                natural_width: 4_100,
                reduced_width: 3_900,
                available: 4_000,
                delta: 100,
                badness: 3,
                discretionary: 100_000,
                warichu: 5,
                formula: 7,
                widow: 11,
                edge_cost: 100_026,
                total_cost: 100_026,
                is_last: false,
                accepted: true,
            },
            Fact::SearchCandidateRefused { candidate: 3 },
            Fact::SearchBoundStop {
                start_candidate: 1,
                end_candidate: 4,
                minimum_width: 4_200,
                available: 4_000,
                best_cost: 19,
            },
            Fact::SearchRefused {
                charged: 64,
                limit: 64,
            },
            chosen(),
        ]
    }

    #[test]
    fn categories_form_a_set() {
        let both = Categories::PREPARE.with(Categories::SEARCH);
        assert!(both.contains(Categories::PREPARE));
        assert!(both.contains(Categories::SEARCH));
        assert!(!both.contains(Categories::KINSOKU));
        assert!(
            both.without(Categories::PREPARE)
                .contains(Categories::SEARCH)
        );
        assert!(
            !both
                .without(Categories::PREPARE)
                .contains(Categories::PREPARE)
        );
        assert!(Categories::NONE.is_empty());
        assert!(!Categories::DEFAULT.is_empty());
        assert_eq!(Categories::NONE.bits(), 0);
    }

    #[test]
    fn the_default_set_omits_exactly_the_two_quadratic_families() {
        assert!(!Categories::DEFAULT.contains(Categories::SEARCH_CANDIDATES));
        assert!(!Categories::DEFAULT.contains(Categories::PLACE_CLUSTERS));
        assert_eq!(
            Categories::DEFAULT,
            Categories::ALL
                .without(Categories::SEARCH_CANDIDATES)
                .without(Categories::PLACE_CLUSTERS)
        );
    }

    #[test]
    fn every_fact_states_a_kind_and_a_family() {
        for fact in every_fact() {
            assert!(!fact.kind().is_empty());
            assert!(!fact.category().is_empty());
            assert!(Categories::ALL.contains(fact.category()));
        }
    }

    #[test]
    fn kinds_are_unique() {
        let facts = every_fact();
        for (index, fact) in facts.iter().enumerate() {
            for other in facts.iter().skip(index.saturating_add(1)) {
                assert_ne!(fact.kind(), other.kind());
            }
        }
    }

    #[test]
    fn kinds_never_look_like_a_product_error_code() {
        // `xtask repository` holds `docs/error-codes.md` and the product literals to the
        // same set. A trace name colliding with one of those namespaces would enter that
        // gate as a compatibility key, which it is not.
        for fact in every_fact() {
            for prefix in [
                "input.",
                "style.",
                "compose.",
                "layout.",
                "font.",
                "document.",
                "limit.",
            ] {
                assert!(
                    !fact.kind().starts_with(prefix),
                    "a trace kind collides with a product error-code namespace"
                );
            }
        }
    }

    #[test]
    fn a_disabled_trace_wants_nothing_and_records_nothing() {
        let mut trace = Trace::off();
        assert!(!trace.wants(Categories::PREPARE));
        assert!(!trace.wants(Categories::SEARCH));
        trace.push(Site::paragraph(0..1, 0..3), prepared());
        assert!(trace.events().is_empty());
        assert!(trace.is_truncated());
    }

    #[test]
    fn the_phase_admits_only_what_it_can_speak_for() {
        let mut trace = Trace::new();
        assert!(trace.wants(Categories::PREPARE));
        assert!(!trace.wants(Categories::REDUCE));
        trace.enter(Phase::Search);
        assert!(trace.wants(Categories::SEARCH));
        assert!(trace.wants(Categories::KINSOKU));
        assert!(!trace.wants(Categories::REDUCE));
        assert!(!trace.wants(Categories::PREPARE));
        trace.enter(Phase::Placement);
        assert!(trace.wants(Categories::REDUCE));
        assert!(trace.wants(Categories::PLACE));
    }

    #[test]
    fn the_ceiling_truncates_and_says_so() {
        let mut trace = Trace::new();
        trace.set_max_events(1);
        assert_eq!(trace.max_events(), 1);
        trace.push(Site::paragraph(0..1, 0..3), prepared());
        trace.push(Site::on_line(0, 1..2, 3..6), chosen());
        assert_eq!(trace.events().len(), 1);
        assert!(trace.is_truncated());
        trace.clear();
        assert!(!trace.is_truncated());
        assert!(trace.events().is_empty());
    }

    #[test]
    fn accessors_and_takeback_preserve_every_field() {
        let mut trace = Trace::default();
        assert_eq!(trace.categories(), Categories::DEFAULT);
        trace.set_categories(Categories::ALL);
        assert_eq!(trace.categories(), Categories::ALL);
        trace.push(Site::on_line(2, 0..3, 0..9), chosen());
        let event = &trace.events()[0];
        assert_eq!(event.site().line(), Some(2));
        assert_eq!(event.site().clusters(), 0..3);
        assert_eq!(event.site().bytes(), 0..9);
        assert_eq!(event.fact(), &chosen());
        assert_eq!(event.kind(), "search.chosen");
        assert_eq!(event.category(), Categories::SEARCH);
        assert!(event.jlreq().is_some());
        assert!(prepared().jlreq().is_none());
        let taken = trace.take_events();
        assert_eq!(taken.len(), 1);
        assert!(trace.events().is_empty());
    }
}
