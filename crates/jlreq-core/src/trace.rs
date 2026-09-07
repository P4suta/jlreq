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

use crate::layout::CoordinateTransform;
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
    /// How much a line has to give or take, and how it was allowed to.
    LineFit {
        /// Whether this line ends the paragraph, which decides whether it justifies.
        is_last: bool,
        /// What the line's content occupies before any adjustment.
        content_width: i64,
        /// The measure.
        available: i64,
        /// Measure minus content; negative when the line must be reduced.
        remaining: i64,
        /// Base clusters on the line.
        cluster_count: usize,
        /// Whether the line is justified rather than set at its natural width.
        justify: bool,
        /// What the ladder is asked to absorb: negative to reduce, positive to expand.
        need: i64,
        /// The offset the alignment applies before anything is placed.
        alignment_offset: i64,
    },
    /// The mojikumi (文字組み) spacing at one boundary, and where it came from.
    ///
    /// `applied` is the amount composition used. `before_term` and `after_term` are the two
    /// halves of the Table 1 cell that the class pair selects; when their sum differs from
    /// `applied`, a construct — a tate-chu-yoko (縦中横) or a formula — stated the spacing
    /// instead, and the difference is the whole explanation.
    BoundarySpace {
        /// The class of the cluster before the boundary.
        before_class: u8,
        /// The class of the cluster after it.
        after_class: u8,
        /// The inline size the leading term is scaled against.
        before_size: i32,
        /// The inline size the trailing term is scaled against.
        after_size: i32,
        /// Whether the leading occurrence is set solid, suppressing its term.
        before_solid: bool,
        /// Whether the trailing occurrence is set solid, suppressing its term.
        after_solid: bool,
        /// The cell's leading term, already scaled.
        before_term: i32,
        /// The cell's trailing term, already scaled.
        after_term: i32,
        /// The spacing composition actually used at this boundary.
        applied: i32,
    },
    /// One boundary the reduction half of the ladder may take from.
    ReductionSite {
        /// The boundary's ordinal within the line.
        boundary: usize,
        /// The proportional weight this boundary carries.
        weight: i32,
        /// The most that may be taken here.
        capacity: i32,
        /// The rung of the ladder this site belongs to.
        stage: u8,
        /// Whether the site is taken whole rather than shared proportionally.
        discrete: bool,
    },
    /// One rung of the reduction ladder, and what it absorbed.
    ReductionStage {
        /// The rung.
        stage: u8,
        /// What still had to be absorbed when the rung was reached.
        need: i64,
        /// What the rung's whole-site takes absorbed before anything was shared.
        discrete_taken: i64,
        /// What the rung's proportional sites could hold between them.
        capacity: i64,
        /// What the rung actually took.
        taken: i64,
        /// What was left for the next rung.
        remaining: i64,
    },
    /// One boundary the expansion half of the ladder may give to.
    ExpansionSite {
        /// The boundary's ordinal within the line.
        boundary: usize,
        /// The proportional weight this boundary carries.
        weight: i32,
        /// The ceiling on this boundary, where the tables state one.
        cap: Option<i32>,
        /// The rung the ceiling belongs to, where there is one.
        stage: Option<u8>,
        /// Whether the boundary may take a share of what no ceiling absorbed.
        residual: bool,
    },
    /// One rung of the expansion ladder, and what it absorbed.
    ExpansionStage {
        /// The rung.
        stage: u8,
        /// Boundaries the rung distributed across.
        sites: usize,
        /// What those boundaries could hold between them.
        capacity: i64,
        /// What the rung actually gave.
        taken: i64,
        /// What was left for the next rung.
        remaining: i64,
    },
    /// What no ceiling could absorb, spread across the boundaries that accept a residue.
    ExpansionResidual {
        /// Boundaries the residue was spread across.
        sites: usize,
        /// The residue.
        amount: i64,
    },
    /// Punctuation hung past the measure rather than forcing another rung.
    Hanging {
        /// What the line occupies before hanging.
        occupied: i64,
        /// The measure.
        available: i64,
        /// What was hung.
        amount: i64,
    },
    /// A warichu (割注) block, and how its two sublines were cut.
    Warichu {
        /// The inline extent of the first subline.
        first_width: i32,
        /// The inline extent of the second.
        second_width: i32,
        /// What the block occupies on the line.
        advance: i32,
    },
    /// A furawake (振分け) block, and the lanes its text was split across.
    Furawake {
        /// The declared column count.
        columns: u16,
        /// The lanes the text actually filled.
        lanes: usize,
        /// The gap set between lanes.
        line_gap: i32,
        /// What the block occupies on the line.
        advance: i32,
        /// The block-axis demand the block makes of the line.
        block_extent: i32,
    },
    /// A tate-chu-yoko (縦中横) group set upright inside vertical writing.
    TateChuYoko {
        /// Clusters in the group.
        members: usize,
        /// What the group would occupy set horizontally.
        horizontal_width: i64,
        /// The block-axis demand it makes of the line.
        block_extent: i32,
    },
    /// One placed cluster.
    ClusterPlaced {
        /// The shaped-text ordinal.
        ordinal: usize,
        /// The logical inline coordinate.
        inline: i32,
        /// The logical block coordinate.
        block: i32,
        /// The placed inline advance.
        advance: i32,
        /// The local transform needed before drawing.
        transform: CoordinateTransform,
    },
    /// One line's finished geometry.
    LineFinished {
        /// The line's logical inline origin.
        inline_origin: i32,
        /// The line's logical block origin.
        block_origin: i32,
        /// The occupied inline extent, excluding hanging punctuation.
        inline_extent: i32,
        /// The line's block-axis demand.
        block_extent: i32,
        /// Base cluster placements emitted.
        clusters: usize,
        /// Ruby, emphasis, reference-mark, and script attachments emitted.
        attachments: usize,
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
            Self::LineFit { .. } => "line.fit",
            Self::BoundarySpace { .. } => "space.boundary",
            Self::ReductionSite { .. } => "reduce.site",
            Self::ReductionStage { .. } => "reduce.stage",
            Self::ExpansionSite { .. } => "expand.site",
            Self::ExpansionStage { .. } => "expand.stage",
            Self::ExpansionResidual { .. } => "expand.residual",
            Self::Hanging { .. } => "hang.line-end",
            Self::Warichu { .. } => "warichu.block",
            Self::Furawake { .. } => "furawake.block",
            Self::TateChuYoko { .. } => "tcy.group",
            Self::ClusterPlaced { .. } => "place.cluster",
            Self::LineFinished { .. } => "line.finished",
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
            Self::LineFit { .. } | Self::LineFinished { .. } => Categories::PLACE,
            Self::BoundarySpace { .. } => Categories::SPACING,
            Self::ReductionSite { .. } | Self::ReductionStage { .. } => Categories::REDUCE,
            Self::ExpansionSite { .. }
            | Self::ExpansionStage { .. }
            | Self::ExpansionResidual { .. } => Categories::EXPAND,
            Self::Hanging { .. } => Categories::HANGING,
            Self::Warichu { .. } | Self::Furawake { .. } | Self::TateChuYoko { .. } => {
                Categories::STRUCTURE
            },
            Self::ClusterPlaced { .. } => Categories::PLACE_CLUSTERS,
        }
    }

    /// The rule that states this decision, where one does.
    ///
    /// Bookkeeping about the engine's own work has no address, and says so rather than
    /// borrowing a nearby one.
    #[must_use]
    pub const fn jlreq(&self) -> Option<RuleAddress> {
        match *self {
            Self::ParagraphPrepared { .. }
            | Self::SearchRefused { .. }
            | Self::ClusterPlaced { .. } => None,
            Self::SearchCandidate { .. } | Self::SearchBoundStop { .. } => {
                Some(RuleAddress::Section("3.8.1"))
            },
            Self::SearchCandidateRefused { .. } => Some(RuleAddress::Section("3.1.9")),
            Self::LineChosen { .. } => Some(RuleAddress::Section("3.1.1")),
            Self::LineFit { .. } | Self::LineFinished { .. } => Some(RuleAddress::Section("3.8.1")),
            Self::BoundarySpace {
                before_class,
                after_class,
                ..
            } => Some(RuleAddress::Cell("B.1", before_class, after_class)),
            Self::ReductionSite { .. }
            | Self::ReductionStage { .. }
            | Self::ExpansionSite { .. }
            | Self::ExpansionStage { .. }
            | Self::ExpansionResidual { .. } => Some(RuleAddress::Section("3.8.3")),
            Self::Hanging { .. } => Some(RuleAddress::Section("2.5.1")),
            // Both stack their text off the line onto sublines beside it, which is one
            // paragraph of the specification rather than two.
            Self::Warichu { .. } | Self::Furawake { .. } => Some(RuleAddress::Section("3.3.2")),
            Self::TateChuYoko { .. } => Some(RuleAddress::Section("3.2.4")),
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
    pub(super) fn every_fact() -> [Fact; 19] {
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
            Fact::LineFit {
                is_last: true,
                content_width: 3_900,
                available: 4_000,
                remaining: 100,
                cluster_count: 4,
                justify: false,
                need: 0,
                alignment_offset: 0,
            },
            Fact::BoundarySpace {
                before_class: 1,
                after_class: 27,
                before_size: 1_000,
                after_size: 1_000,
                before_solid: false,
                after_solid: true,
                before_term: 0,
                after_term: 250,
                applied: 250,
            },
            Fact::ReductionSite {
                boundary: 2,
                weight: 1_000,
                capacity: 250,
                stage: 3,
                discrete: false,
            },
            Fact::ReductionStage {
                stage: 3,
                need: 300,
                discrete_taken: 50,
                capacity: 250,
                taken: 250,
                remaining: 0,
            },
            Fact::ExpansionSite {
                boundary: 1,
                weight: 1_000,
                cap: Some(250),
                stage: Some(2),
                residual: false,
            },
            Fact::ExpansionStage {
                stage: 2,
                sites: 1,
                capacity: 250,
                taken: 250,
                remaining: 0,
            },
            Fact::ExpansionResidual {
                sites: 2,
                amount: 40,
            },
            Fact::Hanging {
                occupied: 4_250,
                available: 4_000,
                amount: 250,
            },
            Fact::Warichu {
                first_width: 1_500,
                second_width: 1_500,
                advance: 1_500,
            },
            Fact::Furawake {
                columns: 3,
                lanes: 3,
                line_gap: 100,
                advance: 2_000,
                block_extent: 3_200,
            },
            Fact::TateChuYoko {
                members: 2,
                horizontal_width: 1_400,
                block_extent: 1_400,
            },
            Fact::ClusterPlaced {
                ordinal: 5,
                inline: 4_000,
                block: 1_000,
                advance: 1_000,
                transform: crate::layout::CoordinateTransform::RotateClockwise,
            },
            Fact::LineFinished {
                inline_origin: 0,
                block_origin: 1_000,
                inline_extent: 4_000,
                block_extent: 1_000,
                clusters: 4,
                attachments: 1,
            },
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

    /// The fixture must hold one of every variant, and the compiler must say so.
    ///
    /// The match below has no wildcard arm, so a new [`Fact`] does not compile until it is
    /// given an index here; the assertion then fails until the fixture actually holds one.
    /// Without this the fixture could silently stop covering a variant, and the rendering
    /// and namespace tests that read it would quietly narrow.
    #[test]
    fn the_fixture_holds_one_of_every_variant() {
        let mut seen = [false; 19];
        for fact in every_fact() {
            let index = match fact {
                Fact::ParagraphPrepared { .. } => 0,
                Fact::SearchCandidate { .. } => 1,
                Fact::SearchCandidateRefused { .. } => 2,
                Fact::SearchBoundStop { .. } => 3,
                Fact::SearchRefused { .. } => 4,
                Fact::LineChosen { .. } => 5,
                Fact::LineFit { .. } => 6,
                Fact::BoundarySpace { .. } => 7,
                Fact::ReductionSite { .. } => 8,
                Fact::ReductionStage { .. } => 9,
                Fact::ExpansionSite { .. } => 10,
                Fact::ExpansionStage { .. } => 11,
                Fact::ExpansionResidual { .. } => 12,
                Fact::Hanging { .. } => 13,
                Fact::Warichu { .. } => 14,
                Fact::Furawake { .. } => 15,
                Fact::TateChuYoko { .. } => 16,
                Fact::ClusterPlaced { .. } => 17,
                Fact::LineFinished { .. } => 18,
            };
            seen[index] = true;
        }
        assert!(
            seen.iter().all(|covered| *covered),
            "the fixture is missing a variant"
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

    /// The ceiling reads back as whatever it was last set to.
    ///
    /// It survived being replaced by a constant, because the corpus sets it
    /// once and every fixture is well under it.
    #[test]
    fn the_event_ceiling_is_the_one_last_set() {
        let mut trace = Trace::new();
        trace.set_max_events(7);
        assert_eq!(trace.max_events(), 7);
        trace.set_max_events(0);
        assert_eq!(trace.max_events(), 0);
    }
}
