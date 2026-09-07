// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Why a document came out as it did.
//!
//! [`jlreq_core::trace`] answers that question for the composition of one paragraph. This
//! module answers it for everything the facade does around that composition — how the text
//! was cut into paragraphs, how graphemes were itemized into shaping runs, **which face was
//! asked to cover a grapheme and how many were tried before it**, and how many break
//! opportunities reached the composer — and it carries every core event through, tagged
//! with the paragraph it came from, so one document has one trace.
//!
//! `docs/adr/0028-the-trace-is-not-a-diagnostic.md` records why this is a channel of its
//! own rather than an extension of [`TextLayout`](crate::TextLayout) or
//! [`Diagnostic`](crate::Diagnostic), and `docs/design/tracing.md` reads the format.
//!
//! ```no_run
//! use jlreq::trace::DocumentTrace;
//! use jlreq::LayoutOptions;
//! # fn example(engine: &mut jlreq::LayoutEngine, fonts: &jlreq::FontLibrary)
//! #     -> Result<(), jlreq::LayoutError> {
//! let options = LayoutOptions::try_new(240.0, 16.0)?;
//! let mut trace = DocumentTrace::new();
//! let layout = engine.layout_traced("一行目\n二行目", fonts, options, &mut trace)?;
//!
//! // One decision per line, already formatted.
//! print!("{trace}");
//!
//! // Or walk it: every event names its kind, its family, and where in the document it belongs.
//! for event in trace.events() {
//!     let _ = (event.kind(), event.site().paragraph(), event.site().bytes());
//! }
//! # let _ = layout;
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::ops::Range;

use jlreq_core::trace::{Event as CoreEvent, Trace as CoreTrace};

use crate::{Alignment, BaseDirection, Widow, WritingMode};

/// The width the kind column is padded to.
///
/// It matches [`jlreq_core::trace`]'s own column exactly, so an absorbed core line and a
/// facade line put their fields in the same place and a diff of the two channels aligns.
const KIND_WIDTH: usize = 24;

/// The identifier a reader can use to tell one rendering generation from another.
const FORMAT: &str = "jlreq.trace/1 document";

/// The families of facade decision a [`DocumentTrace`] may record.
///
/// A set, not an enumeration: build one by combining constants with [`Categories::with`].
/// Unlike the core's, no family here grows faster than the input — the facade records one
/// event per paragraph, per shaping run, and per *distinct* face decision — so
/// [`Categories::ALL`] is also the default. The core categories are chosen separately,
/// because two of those genuinely are superlinear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Categories(u32);

impl Categories {
    /// Record nothing.
    pub const NONE: Self = Self(0x0000);
    /// How the document was cut into paragraphs, and the style each one resolved to.
    pub const PARAGRAPHS: Self = Self(0x0001);
    /// How each paragraph's text was itemized before shaping.
    pub const ITEMIZE: Self = Self(0x0002);
    /// Each shaping run: its script, direction, resolved level, face, and glyph count.
    pub const RUNS: Self = Self(0x0004);
    /// Which face covered a grapheme, and how many candidates were tried first.
    pub const FACES: Self = Self(0x0008);
    /// How many break opportunities of each strength reached the composer.
    pub const BREAKS: Self = Self(0x0010);
    /// The core's own composition events, tagged with the paragraph they came from.
    pub const CORE: Self = Self(0x0020);
    /// How the composer's logical placements became physical cells.
    ///
    /// The core says where it put each cluster; this says what the facade then
    /// did with that, which is a separate arithmetic and had its own defects.
    pub const PLACEMENT: Self = Self(0x0040);
    /// Every family this release names.
    pub const ALL: Self = Self(0x007f);

    /// The union of two sets.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// This set without the named families.
    #[must_use]
    pub const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// Whether every family in `other` is in this set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether this set names no family at all.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The raw bits, as the rendered header prints them.
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }
}

/// Where in the document a facade decision belongs.
///
/// Byte offsets are always into the document's own text, never into a paragraph's slice —
/// including for an absorbed core event, whose paragraph-local offsets are shifted here as
/// it is taken in. The rendered core line keeps the core's own frame; `para.segment` states
/// each paragraph's document range so the two are always reconcilable.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Site {
    paragraph: Option<usize>,
    bytes: Range<usize>,
}

impl Site {
    /// The paragraph ordinal this decision belongs to, or None for the whole document.
    #[must_use]
    pub const fn paragraph(&self) -> Option<usize> {
        self.paragraph
    }

    /// The document UTF-8 byte range the decision covers.
    #[must_use]
    pub fn bytes(&self) -> Range<usize> {
        self.bytes.clone()
    }

    pub(crate) const fn document(bytes: Range<usize>) -> Self {
        Self {
            paragraph: None,
            bytes,
        }
    }

    pub(crate) const fn in_paragraph(paragraph: usize, bytes: Range<usize>) -> Self {
        Self {
            paragraph: Some(paragraph),
            bytes,
        }
    }
}

/// Which script class the facade assigned a run, as a stable token.
///
/// The classification is the facade's own coarse partition — it selects the shaping
/// direction and nothing else — so it is reported as what it is rather than as a Unicode
/// script property the library does not claim to resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Script {
    /// Kana, Han, and the CJK-compatible ranges the vertical rules apply to.
    Japanese,
    /// Latin letters and digits.
    Latin,
    /// A right-to-left script.
    Rtl,
    /// An emoji range.
    Emoji,
    /// Anything the partition does not name.
    Other,
}

/// Which way a run was handed to the shaper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RunDirection {
    /// Inline progression left to right.
    LeftToRight,
    /// Inline progression right to left.
    RightToLeft,
    /// Inline progression top to bottom, as an upright vertical run.
    TopToBottom,
    /// Anything else the shaper reported.
    Other,
}

/// One recorded decision.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Fact {
    /// How the document was cut up, and the options the whole call ran under.
    TextSegmented {
        /// Paragraphs the separator scan produced.
        paragraphs: usize,
        /// Source bytes the document holds.
        bytes: usize,
        /// The writing mode every paragraph is set in.
        writing_mode: WritingMode,
        /// The base direction handed to UAX #9.
        base_direction: BaseDirection,
        /// The document's default measure, in caller units.
        line_extent: i32,
        /// The document's default em size, in caller units.
        font_size: i32,
    },
    /// A paragraph of the document, and the style that resolved for it.
    ParagraphSegment {
        /// The paragraph ordinal, matching [`TextLine::paragraph_index`](crate::TextLine).
        index: usize,
        /// Whether the paragraph holds no text at all.
        blank: bool,
        /// The measure this paragraph was composed against, in caller units.
        line_extent: i32,
        /// The alignment that resolved for it.
        alignment: Alignment,
        /// The first-line indent that resolved for it, in caller units.
        first_line_indent: i32,
        /// The final-line policy that resolved for it.
        widow: Widow,
    },
    /// How one stretch of text was itemized before shaping.
    TextItemized {
        /// Grapheme clusters the segmenter produced.
        graphemes: usize,
        /// Shaping runs those graphemes were grouped into.
        runs: usize,
        /// Shaped clusters handed to the core.
        clusters: usize,
        /// The bidi paragraph level UAX #9 resolved.
        base_level: u8,
        /// Whether any grapheme resolved to a level other than the paragraph level.
        mixed_levels: bool,
        /// Whether this text is a construct's annotation rather than the base text.
        annotation: bool,
    },
    /// One run handed to the shaper as a unit.
    ShapingRun {
        /// The script class that selected the direction.
        script: Script,
        /// The direction the run was shaped in.
        direction: RunDirection,
        /// The bidi level the run's first grapheme resolved to.
        level: u8,
        /// The face the run was shaped with.
        face: u32,
        /// Raw glyphs the shaper returned.
        glyphs: usize,
        /// Whether this run belongs to a construct's annotation.
        annotation: bool,
    },
    /// A face was found that covers a grapheme completely.
    FaceChosen {
        /// The chosen face.
        face: u32,
        /// The family that face declares.
        family: String,
        /// Its one-based position in the candidate order.
        position: usize,
        /// How many candidates the order held.
        candidates: usize,
    },
    /// No candidate covered a grapheme, so the primary face's `.notdef` was kept.
    FaceFallback {
        /// The primary face, which was retained.
        face: u32,
        /// The family that face declares.
        family: String,
        /// How many candidates were tried and rejected.
        candidates: usize,
    },
    /// The break opportunities a paragraph handed to the composer.
    BreaksCollected {
        /// Opportunities UAX #14 and the construct rules left available.
        allowed: usize,
        /// Author-declared opportunities that cost the search a surcharge.
        discretionary: usize,
        /// Author-declared breaks the search must take.
        mandatory: usize,
    },
    /// One line, after the facade turned the composer's placements into cells.
    LinePlaced {
        /// The line ordinal within its paragraph.
        line: usize,
        /// Cells the line's placements produced, after bidi reordering.
        cells: usize,
        /// Where the visual cursor started, in caller units.
        cursor: i32,
        /// The inline extent the composer reported, in caller units.
        inline_extent: i32,
        /// Where the last cell ended, in caller units.
        content_end: i32,
    },
    /// One cell's step, in the order the cells are visited.
    ///
    /// `advance` is what the composer charged the cell and `step` is how far the
    /// cursor actually moved. The two differ on purpose — a conditional space at
    /// a class boundary is billed to both sides, and a tate-chu-yoko run's halves
    /// share one coordinate — and both defects this channel was added for were a
    /// disagreement between them that nothing recorded.
    CellStepped {
        /// The cell's position in the visual order.
        ordinal: usize,
        /// The inline coordinate the composer placed it at, in caller units.
        inline: i32,
        /// The advance the composer charged it, in caller units.
        advance: i32,
        /// How far the cursor moved past it, in caller units.
        step: i32,
    },
    /// One decision the core recorded while composing a paragraph.
    Core {
        /// The core's own event, in the core's own frame of reference.
        event: CoreEvent,
    },
}

impl Fact {
    /// The stable name of this kind of decision.
    ///
    /// The namespace is deliberately disjoint from the product error-code namespaces in
    /// `docs/error-codes.md`; a trace name is a reading aid, not a compatibility key. A
    /// core fact reports the core's own name unchanged, because the line is the core's.
    ///
    /// There is no wildcard arm: a new [`Fact`] does not compile until it is named.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match *self {
            Self::TextSegmented { .. } => "text.segmented",
            Self::ParagraphSegment { .. } => "para.segment",
            Self::TextItemized { .. } => "text.itemized",
            Self::ShapingRun { .. } => "text.run",
            Self::FaceChosen { .. } => "face.chosen",
            Self::FaceFallback { .. } => "face.fallback",
            Self::BreaksCollected { .. } => "text.breaks",
            Self::LinePlaced { .. } => "draw.line",
            Self::CellStepped { .. } => "draw.cell",
            Self::Core { ref event } => event.kind(),
        }
    }

    /// The family this decision belongs to.
    ///
    /// There is no wildcard arm: a new [`Fact`] does not compile until it is filed.
    #[must_use]
    pub const fn category(&self) -> Categories {
        match *self {
            Self::TextSegmented { .. } | Self::ParagraphSegment { .. } => Categories::PARAGRAPHS,
            Self::TextItemized { .. } => Categories::ITEMIZE,
            Self::ShapingRun { .. } => Categories::RUNS,
            Self::FaceChosen { .. } | Self::FaceFallback { .. } => Categories::FACES,
            Self::BreaksCollected { .. } => Categories::BREAKS,
            Self::LinePlaced { .. } | Self::CellStepped { .. } => Categories::PLACEMENT,
            Self::Core { .. } => Categories::CORE,
        }
    }
}

/// One recorded decision, and where it belongs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Event {
    site: Site,
    fact: Fact,
}

impl Event {
    /// Where in the document this decision belongs.
    #[must_use]
    pub const fn site(&self) -> &Site {
        &self.site
    }

    /// What was decided.
    #[must_use]
    pub const fn fact(&self) -> &Fact {
        &self.fact
    }

    /// The stable name of this kind of decision.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        self.fact.kind()
    }

    /// The family this decision belongs to.
    #[must_use]
    pub const fn category(&self) -> Categories {
        self.fact.category()
    }
}

/// A recorded sequence of document layout decisions.
///
/// Two category sets, because the two channels have different cost profiles: the facade's
/// families are all linear in the input, and two of the core's are not.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DocumentTrace {
    events: Vec<Event>,
    categories: Categories,
    core_categories: jlreq_core::trace::Categories,
    limit: usize,
    truncated: bool,
}

impl DocumentTrace {
    /// The default event ceiling.
    ///
    /// A document trace absorbs a core trace per paragraph, so the ceiling is a whole
    /// document's budget rather than a paragraph's, and the core is handed whatever
    /// remains of it before each paragraph.
    pub const DEFAULT_MAX_EVENTS: usize = 1_000_000;

    /// A trace recording every facade family and [`jlreq_core::trace::Categories::DEFAULT`].
    #[must_use]
    pub const fn new() -> Self {
        Self::with_categories(Categories::ALL, jlreq_core::trace::Categories::DEFAULT)
    }

    /// A trace recording exactly the named facade and core families.
    #[must_use]
    pub const fn with_categories(
        categories: Categories,
        core_categories: jlreq_core::trace::Categories,
    ) -> Self {
        Self {
            events: Vec::new(),
            categories,
            core_categories,
            limit: Self::DEFAULT_MAX_EVENTS,
            truncated: false,
        }
    }

    /// The facade families this trace records.
    #[must_use]
    pub const fn categories(&self) -> Categories {
        self.categories
    }

    /// Record a different set of facade families from here on.
    pub const fn set_categories(&mut self, categories: Categories) {
        self.categories = categories;
    }

    /// The core families this trace asks each paragraph's composition for.
    #[must_use]
    pub const fn core_categories(&self) -> jlreq_core::trace::Categories {
        self.core_categories
    }

    /// Ask each paragraph's composition for a different set of core families.
    pub const fn set_core_categories(&mut self, categories: jlreq_core::trace::Categories) {
        self.core_categories = categories;
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

    /// The recorded decisions, in the order the engine reached them.
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
    }

    /// Take the recorded decisions, leaving an empty configured trace behind.
    #[must_use]
    pub fn take_events(&mut self) -> Vec<Event> {
        self.truncated = false;
        std::mem::take(&mut self.events)
    }

    /// A sink that records nothing and allocates nothing.
    ///
    /// This is what [`LayoutEngine::layout`](crate::LayoutEngine::layout) passes, so the
    /// traced and untraced entry points run one body rather than two.
    pub(crate) const fn off() -> Self {
        Self {
            events: Vec::new(),
            categories: Categories::NONE,
            core_categories: jlreq_core::trace::Categories::NONE,
            limit: 0,
            truncated: false,
        }
    }

    /// The one branch an instrumentation site pays when tracing is off.
    pub(crate) const fn wants(&self, category: Categories) -> bool {
        self.categories.contains(category)
    }

    pub(crate) fn push(&mut self, site: Site, fact: Fact) {
        if self.events.len() >= self.limit {
            self.truncated = true;
            return;
        }
        self.events.push(Event { site, fact });
    }

    pub(crate) fn record(&mut self, site: Site, fact: Fact) {
        if self.wants(fact.category()) {
            self.push(site, fact);
        }
    }

    /// How many more events this trace will hold.
    pub(crate) fn remaining(&self) -> usize {
        self.limit.saturating_sub(self.events.len())
    }

    /// Take one paragraph's core trace in, shifting every site into document coordinates.
    ///
    /// Called before the composer's result is unwrapped, so a paragraph that refused still
    /// leaves behind the reasoning that led to the refusal — which is the case a reader
    /// most needs it for.
    pub(crate) fn absorb(&mut self, paragraph: usize, offset: usize, core: &mut CoreTrace) {
        if core.is_truncated() {
            self.truncated = true;
        }
        for event in core.take_events() {
            let bytes = event.site().bytes();
            let site = Site::in_paragraph(
                paragraph,
                bytes.start.saturating_add(offset)..bytes.end.saturating_add(offset),
            );
            self.push(site, Fact::Core { event });
        }
    }

    pub(crate) fn core_trace(&self) -> CoreTrace {
        let mut core = CoreTrace::with_categories(self.core_categories);
        core.set_max_events(self.remaining());
        core
    }
}

impl Default for DocumentTrace {
    fn default() -> Self {
        Self::new()
    }
}

/// The paragraph-scope column that every line starts with.
impl fmt::Display for Site {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.paragraph {
            Some(paragraph) => write!(formatter, "P{paragraph:02}"),
            None => write!(formatter, "doc"),
        }
    }
}

impl fmt::Display for Event {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{site} ", site = self.site)?;
        write_body(&self.site, &self.fact, formatter)
    }
}

impl fmt::Display for DocumentTrace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "{FORMAT} events={count} facade=0x{facade:04x} core=0x{core:04x} truncated={truncated}",
            count = self.events.len(),
            facade = self.categories.bits(),
            core = self.core_categories.bits(),
            truncated = flag(self.truncated),
        )?;
        for (ordinal, event) in self.events.iter().enumerate() {
            writeln!(formatter, "{ordinal:04} {event}")?;
        }
        Ok(())
    }
}

/// A boolean as one column, because `true` and `false` do not align.
const fn flag(value: bool) -> u8 {
    if value { 1 } else { 0 }
}

/// The alignment as a stable token.
const fn alignment(value: Alignment) -> &'static str {
    match value {
        Alignment::Start => "start",
        Alignment::Center => "center",
        Alignment::End => "end",
        Alignment::Justify => "justify",
    }
}

/// The writing mode as a stable token, spelled as the core spells it.
const fn writing_mode(value: WritingMode) -> &'static str {
    match value {
        WritingMode::HorizontalTb => "horizontal-tb",
        WritingMode::VerticalRl => "vertical-rl",
    }
}

/// The requested base direction as a stable token.
const fn base_direction(value: BaseDirection) -> &'static str {
    match value {
        BaseDirection::Auto => "auto",
        BaseDirection::LeftToRight => "ltr",
        BaseDirection::RightToLeft => "rtl",
    }
}

/// The script class as a stable token.
const fn script(value: Script) -> &'static str {
    match value {
        Script::Japanese => "japanese",
        Script::Latin => "latin",
        Script::Rtl => "rtl",
        Script::Emoji => "emoji",
        Script::Other => "other",
    }
}

/// The shaping direction as a stable token.
const fn direction(value: RunDirection) -> &'static str {
    match value {
        RunDirection::LeftToRight => "ltr",
        RunDirection::RightToLeft => "rtl",
        RunDirection::TopToBottom => "ttb",
        RunDirection::Other => "other",
    }
}

/// The final-line policy as a stable token pair.
fn widow(value: Widow) -> String {
    match value {
        Widow::Allow => "allow".to_owned(),
        Widow::MinimumClusters(minimum) => format!("min-{minimum}"),
    }
}

/// The kind column, padded to the shared width, then the document byte range.
fn write_head(site: &Site, fact: &Fact, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
        formatter,
        "{kind:KIND_WIDTH$} b{start}..{end} ",
        kind = fact.kind(),
        start = site.bytes.start,
        end = site.bytes.end,
    )
}

/// One decision, after the scope column.
///
/// There is no wildcard arm: a new [`Fact`] does not compile until it is rendered.
///
/// A core fact renders as the core's own line, unchanged and un-prefixed. The core pads its
/// kind to the same width, so the two channels put their fields in the same column, and a
/// line that appears in both a core golden and a document golden reads identically in each.
fn write_body(site: &Site, fact: &Fact, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match *fact {
        Fact::Core { ref event } => write!(formatter, "{event}"),
        Fact::TextSegmented {
            paragraphs,
            bytes,
            writing_mode: mode,
            base_direction: base,
            line_extent,
            font_size,
        } => {
            write_head(site, fact, formatter)?;
            write!(
                formatter,
                "paragraphs={paragraphs} bytes={bytes} mode={mode} base={base} \
                 extent={line_extent} size={font_size}",
                mode = writing_mode(mode),
                base = base_direction(base),
            )
        },
        Fact::ParagraphSegment {
            index,
            blank,
            line_extent,
            alignment: align,
            first_line_indent,
            widow: policy,
        } => {
            write_head(site, fact, formatter)?;
            write!(
                formatter,
                "index={index} blank={blank} extent={line_extent} \
                 align={align} indent={first_line_indent} widow={policy}",
                blank = flag(blank),
                align = alignment(align),
                policy = widow(policy),
            )
        },
        Fact::TextItemized {
            graphemes,
            runs,
            clusters,
            base_level,
            mixed_levels,
            annotation,
        } => {
            write_head(site, fact, formatter)?;
            write!(
                formatter,
                "graphemes={graphemes} runs={runs} clusters={clusters} \
                 base-level={base_level} mixed={mixed_levels} annotation={annotation}",
                mixed_levels = flag(mixed_levels),
                annotation = flag(annotation),
            )
        },
        Fact::ShapingRun {
            script: class,
            direction: way,
            level,
            face,
            glyphs,
            annotation,
        } => {
            write_head(site, fact, formatter)?;
            write!(
                formatter,
                "script={class} direction={way} level={level} face={face} \
                 glyphs={glyphs} annotation={annotation}",
                class = script(class),
                way = direction(way),
                annotation = flag(annotation),
            )
        },
        Fact::FaceChosen {
            face,
            ref family,
            position,
            candidates,
        } => {
            write_head(site, fact, formatter)?;
            write!(
                formatter,
                "face={face} family={family} position={position} \
                 candidates={candidates}"
            )
        },
        Fact::FaceFallback {
            face,
            ref family,
            candidates,
        } => {
            write_head(site, fact, formatter)?;
            write!(
                formatter,
                "face={face} family={family} candidates={candidates}"
            )
        },
        Fact::BreaksCollected {
            allowed,
            discretionary,
            mandatory,
        } => {
            write_head(site, fact, formatter)?;
            write!(
                formatter,
                "allowed={allowed} discretionary={discretionary} \
                 mandatory={mandatory}"
            )
        },
        Fact::LinePlaced {
            line,
            cells,
            cursor,
            inline_extent,
            content_end,
        } => {
            write_head(site, fact, formatter)?;
            write!(
                formatter,
                "line={line} cells={cells} cursor={cursor} \
                 extent={inline_extent} content={content_end}"
            )
        },
        Fact::CellStepped {
            ordinal,
            inline,
            advance,
            step,
        } => {
            write_head(site, fact, formatter)?;
            write!(
                formatter,
                "ordinal={ordinal} inline={inline} advance={advance} step={step}"
            )
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site() -> Site {
        Site::in_paragraph(0, 3..9)
    }

    /// One of every variant, so the tables below have something to be complete about.
    pub(super) fn every_fact() -> Vec<Fact> {
        vec![
            Fact::TextSegmented {
                paragraphs: 2,
                bytes: 24,
                writing_mode: WritingMode::VerticalRl,
                base_direction: BaseDirection::Auto,
                line_extent: 20_000,
                font_size: 1_000,
            },
            Fact::ParagraphSegment {
                index: 0,
                blank: false,
                line_extent: 20_000,
                alignment: Alignment::Justify,
                first_line_indent: 1_000,
                widow: Widow::MinimumClusters(2),
            },
            Fact::TextItemized {
                graphemes: 8,
                runs: 2,
                clusters: 8,
                base_level: 0,
                mixed_levels: true,
                annotation: false,
            },
            Fact::ShapingRun {
                script: Script::Japanese,
                direction: RunDirection::LeftToRight,
                level: 0,
                face: 0,
                glyphs: 6,
                annotation: false,
            },
            Fact::FaceChosen {
                face: 1,
                family: "Secondary".to_owned(),
                position: 2,
                candidates: 3,
            },
            Fact::FaceFallback {
                face: 0,
                family: "Primary".to_owned(),
                candidates: 3,
            },
            Fact::BreaksCollected {
                allowed: 7,
                discretionary: 1,
                mandatory: 0,
            },
            Fact::LinePlaced {
                line: 0,
                cells: 9,
                cursor: 0,
                inline_extent: 11_520,
                content_end: 11_520,
            },
            Fact::CellStepped {
                ordinal: 2,
                inline: 2_048,
                advance: 1_280,
                step: 1_152,
            },
            Fact::Core {
                event: core_event(),
            },
        ]
    }

    /// A real core event, taken from a real composition rather than fabricated, because
    /// the core's constructors are its own and this channel must render what it is given.
    fn core_event() -> CoreEvent {
        let mut trace = CoreTrace::new();
        compose_fixture(&mut trace);
        trace
            .events()
            .first()
            .cloned()
            .expect("core records a prepare event for any paragraph")
    }

    /// A real two-cluster composition, so the absorbed events are what the core actually
    /// emits rather than values this module invented to render.
    fn compose_fixture(trace: &mut CoreTrace) {
        let source = "日本";
        let clusters = source.char_indices().map(|(start, character)| {
            jlreq_core::Cluster::new(start..start.saturating_add(character.len_utf8()), 1_000)
        });
        let text = jlreq_core::ShapedText::new(
            source,
            jlreq_core::Size::square(1_000).expect("a square em is a valid size"),
            jlreq_core::Frame::FullEm,
            clusters,
        )
        .expect("two ordinary clusters shape");
        let paragraph = jlreq_core::Paragraph::builder(text, 2_000)
            .build()
            .expect("a two-cluster paragraph builds");
        let composed = jlreq_core::compose_traced(&paragraph, &jlreq_core::Style::default(), trace);
        assert!(composed.is_ok(), "the fixture paragraph composes");
    }

    #[test]
    fn the_fixture_holds_one_of_every_variant() {
        let mut seen = [false; 10];
        for fact in every_fact() {
            let index = match fact {
                Fact::TextSegmented { .. } => 0,
                Fact::ParagraphSegment { .. } => 1,
                Fact::TextItemized { .. } => 2,
                Fact::ShapingRun { .. } => 3,
                Fact::FaceChosen { .. } => 4,
                Fact::FaceFallback { .. } => 5,
                Fact::BreaksCollected { .. } => 6,
                Fact::LinePlaced { .. } => 8,
                Fact::CellStepped { .. } => 9,
                Fact::Core { .. } => 7,
            };
            seen[index] = true;
        }
        assert!(seen.iter().all(|hit| *hit), "a variant is unrepresented");
    }

    #[test]
    fn no_kind_collides_with_a_product_error_code_namespace() {
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
    fn every_fact_renders_its_kind_and_scope() {
        for fact in every_fact() {
            let kind = fact.kind();
            let event = Event { site: site(), fact };
            let rendered = event.to_string();
            assert!(rendered.starts_with("P00 "), "{rendered}");
            assert!(rendered.contains(kind), "{rendered}");
        }
    }

    #[test]
    fn a_disabled_trace_records_nothing() {
        let mut trace = DocumentTrace::off();
        for fact in every_fact() {
            trace.record(site(), fact);
        }
        assert!(trace.events().is_empty());
        assert!(!trace.is_truncated());
    }

    #[test]
    fn the_ceiling_truncates_rather_than_grows() {
        let mut trace = DocumentTrace::new();
        trace.set_max_events(2);
        for fact in every_fact() {
            trace.record(site(), fact);
        }
        assert_eq!(trace.events().len(), 2);
        assert!(trace.is_truncated());
        assert_eq!(trace.remaining(), 0);
    }

    #[test]
    fn categories_select_one_family_at_a_time() {
        let mut trace =
            DocumentTrace::with_categories(Categories::FACES, jlreq_core::trace::Categories::NONE);
        for fact in every_fact() {
            trace.record(site(), fact);
        }
        let kinds: Vec<_> = trace.events().iter().map(Event::kind).collect();
        assert_eq!(kinds, ["face.chosen", "face.fallback"]);
    }

    #[test]
    fn a_set_is_a_set() {
        let both = Categories::RUNS.with(Categories::FACES);
        assert!(both.contains(Categories::RUNS));
        assert!(both.contains(Categories::FACES));
        assert!(!both.without(Categories::FACES).contains(Categories::FACES));
        assert!(Categories::NONE.is_empty());
        assert!(!Categories::ALL.is_empty());
        assert_eq!(Categories::ALL.bits(), 0x007f);
    }

    #[test]
    fn taking_events_leaves_a_configured_trace_behind() {
        let mut trace = DocumentTrace::new();
        trace.record(site(), every_fact().swap_remove(0));
        let taken = trace.take_events();
        assert_eq!(taken.len(), 1);
        assert!(trace.events().is_empty());
        assert_eq!(trace.categories(), Categories::ALL);
        assert_eq!(
            trace.core_categories(),
            jlreq_core::trace::Categories::DEFAULT
        );
        trace.record(site(), every_fact().swap_remove(1));
        assert_eq!(trace.events().len(), 1);
        trace.clear();
        assert!(trace.events().is_empty());
    }

    #[test]
    fn absorbing_shifts_every_core_site_into_document_coordinates() {
        let mut document = DocumentTrace::new();
        let mut core = document.core_trace();
        compose_fixture(&mut core);
        let local: Vec<_> = core
            .events()
            .iter()
            .map(|event| event.site().bytes())
            .collect();
        assert!(!local.is_empty());
        document.absorb(3, 100, &mut core);
        assert!(core.events().is_empty());
        for (event, expected) in document.events().iter().zip(local) {
            assert_eq!(event.site().paragraph(), Some(3));
            assert_eq!(
                event.site().bytes(),
                expected.start.saturating_add(100)..expected.end.saturating_add(100)
            );
        }
    }

    #[test]
    fn the_document_scope_prints_without_a_paragraph() {
        let event = Event {
            site: Site::document(0..12),
            fact: Fact::BreaksCollected {
                allowed: 1,
                discretionary: 0,
                mandatory: 0,
            },
        };
        assert!(event.to_string().starts_with("doc "), "{event}");
        assert_eq!(event.site().paragraph(), None);
        assert_eq!(event.site().bytes(), 0..12);
    }

    #[test]
    fn the_header_names_both_channels() {
        let mut trace = DocumentTrace::new();
        trace.record(site(), every_fact().swap_remove(6));
        let rendered = trace.to_string();
        let header = rendered.lines().next().unwrap_or_default();
        assert_eq!(
            header,
            "jlreq.trace/1 document events=1 facade=0x007f core=0x07fb truncated=0"
        );
        assert!(rendered.contains("0000 P00 text.breaks"), "{rendered}");
    }

    /// The set algebra and the three settings a caller configures a trace with.
    ///
    /// Every one of these survived mutation: `with` still passed with `|` moved
    /// to `^`, which is the same answer for disjoint sets and the wrong one the
    /// moment a family is in both; and the setters still passed when replaced
    /// by nothing at all, because the corpus configures a trace once and never
    /// reconfigures one.
    #[test]
    fn a_family_already_in_a_set_survives_being_added_again() {
        let breaks = Categories::NONE.with(Categories::BREAKS);
        assert_eq!(breaks.with(Categories::BREAKS), breaks);
        assert_eq!(
            Categories::BREAKS.with(Categories::RUNS),
            Categories::NONE
                .with(Categories::RUNS)
                .with(Categories::BREAKS)
        );
        assert_eq!(Categories::ALL.with(Categories::ALL), Categories::ALL);
    }

    #[test]
    fn a_trace_reports_the_settings_it_was_last_given() {
        let mut trace = DocumentTrace::new();
        trace.set_categories(Categories::BREAKS);
        assert_eq!(trace.categories(), Categories::BREAKS);
        trace.set_categories(Categories::RUNS);
        assert_eq!(trace.categories(), Categories::RUNS);

        trace.set_core_categories(jlreq_core::trace::Categories::NONE);
        assert_eq!(trace.core_categories(), jlreq_core::trace::Categories::NONE);
        trace.set_core_categories(jlreq_core::trace::Categories::ALL);
        assert_eq!(trace.core_categories(), jlreq_core::trace::Categories::ALL);

        trace.set_max_events(7);
        assert_eq!(trace.max_events(), 7);
        trace.set_max_events(0);
        assert_eq!(trace.max_events(), 0);
    }
}
