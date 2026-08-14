// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::construct::{Construct, ConstructKind, is_math_token};
use crate::model::{InputError, ShapedText, WritingMode};

/// The semantic kind of a caller-supplied line-break opportunity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BreakKind {
    Allowed,
    Mandatory,
    Discretionary,
}

/// One break opportunity at a UTF-8 byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Break {
    offset: usize,
    kind: BreakKind,
}

impl Break {
    /// Declare an ordinary UAX #14 break opportunity.
    #[must_use]
    pub const fn allowed(offset: usize) -> Self {
        Self {
            offset,
            kind: BreakKind::Allowed,
        }
    }

    /// Declare a mandatory break.
    #[must_use]
    pub const fn mandatory(offset: usize) -> Self {
        Self {
            offset,
            kind: BreakKind::Mandatory,
        }
    }

    /// Declare a discretionary break that is penalized when taken.
    #[must_use]
    pub const fn discretionary(offset: usize) -> Self {
        Self {
            offset,
            kind: BreakKind::Discretionary,
        }
    }

    /// The UTF-8 byte offset immediately after the preceding cluster.
    #[must_use]
    pub const fn offset(self) -> usize {
        self.offset
    }

    /// Whether the paragraph must break here.
    #[must_use]
    pub const fn is_mandatory(self) -> bool {
        matches!(self.kind, BreakKind::Mandatory)
    }

    /// Whether choosing this opportunity carries a discretionary penalty.
    #[must_use]
    pub const fn is_discretionary(self) -> bool {
        matches!(self.kind, BreakKind::Discretionary)
    }
}

/// Inline alignment inside each line measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Alignment {
    /// Align at inline start.
    Start,
    /// Center the line.
    Center,
    /// Align at inline end.
    End,
    /// Expand legal internal spacing to fill non-final lines.
    #[default]
    Justify,
}

/// Widow control for the final line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Widow {
    /// Do not impose a final-line cluster minimum.
    #[default]
    Allow,
    /// Prefer at least this many base clusters on the final line.
    MinimumClusters(u16),
}

/// Direction-independent alignment at a tab stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TabAlignment {
    /// Put the following segment's inline start at the stop.
    Start,
    /// Center the following segment on the stop.
    Center,
    /// Put the following segment's inline end at the stop.
    End,
    /// Align the first occurrence of this character with the stop.
    Character(char),
}

/// One tab position measured from the line's inline start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TabStop {
    position: i32,
    alignment: TabAlignment,
}

impl TabStop {
    /// Build a positive tab position in caller units.
    pub fn new(position: i32, alignment: TabAlignment) -> Result<Self, InputError> {
        if position <= 0 {
            return Err(InputError::new(
                "input.invalid-tab-stop",
                None,
                "tab stops must have positive positions",
            ));
        }
        Ok(Self {
            position,
            alignment,
        })
    }

    /// The position from inline start in caller units.
    #[must_use]
    pub const fn position(self) -> i32 {
        self.position
    }

    /// The direction-independent alignment.
    #[must_use]
    pub const fn alignment(self) -> TabAlignment {
        self.alignment
    }
}

/// A completely validated paragraph ready for infallible composition.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Paragraph {
    pub(crate) text: ShapedText,
    pub(crate) line_extent: i32,
    pub(crate) breaks: Vec<Break>,
    pub(crate) constructs: Vec<Construct>,
    pub(crate) tab_stops: Vec<TabStop>,
    pub(crate) first_line_indent: i32,
    pub(crate) alignment: Alignment,
    pub(crate) widow: Widow,
    pub(crate) writing_mode: WritingMode,
}

impl Paragraph {
    /// Start a builder around already-validated shaped text and a line measure.
    #[must_use]
    pub fn builder(text: ShapedText, line_extent: i32) -> ParagraphBuilder {
        ParagraphBuilder {
            text,
            line_extent,
            breaks: Vec::new(),
            constructs: Vec::new(),
            tab_stops: Vec::new(),
            first_line_indent: 0,
            alignment: Alignment::default(),
            widow: Widow::default(),
            writing_mode: WritingMode::default(),
        }
    }

    /// The shaped source.
    #[must_use]
    pub const fn text(&self) -> &ShapedText {
        &self.text
    }

    /// The available inline extent per line.
    #[must_use]
    pub const fn line_extent(&self) -> i32 {
        self.line_extent
    }

    /// Validated break opportunities, including the implicit paragraph end.
    #[must_use]
    pub fn breaks(&self) -> &[Break] {
        &self.breaks
    }

    /// Validated inline structures in declaration order.
    #[must_use]
    pub fn constructs(&self) -> &[Construct] {
        &self.constructs
    }

    /// Validated, increasing tab stops.
    #[must_use]
    pub fn tab_stops(&self) -> &[TabStop] {
        &self.tab_stops
    }

    /// The first-line indent in caller units.
    #[must_use]
    pub const fn first_line_indent(&self) -> i32 {
        self.first_line_indent
    }

    /// The line alignment.
    #[must_use]
    pub const fn alignment(&self) -> Alignment {
        self.alignment
    }

    /// The final-line widow preference.
    #[must_use]
    pub const fn widow(&self) -> Widow {
        self.widow
    }

    /// The paragraph writing mode.
    #[must_use]
    pub const fn writing_mode(&self) -> WritingMode {
        self.writing_mode
    }
}

/// Collects and jointly validates paragraph-level inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParagraphBuilder {
    text: ShapedText,
    line_extent: i32,
    breaks: Vec<Break>,
    constructs: Vec<Construct>,
    tab_stops: Vec<TabStop>,
    first_line_indent: i32,
    alignment: Alignment,
    widow: Widow,
    writing_mode: WritingMode,
}

impl ParagraphBuilder {
    /// Add break opportunities, replacing none already added.
    #[must_use]
    pub fn breaks<I>(mut self, breaks: I) -> Self
    where
        I: IntoIterator<Item = Break>,
    {
        self.breaks.extend(breaks);
        self
    }

    /// Add inline structures.
    #[must_use]
    pub fn constructs<I>(mut self, constructs: I) -> Self
    where
        I: IntoIterator<Item = Construct>,
    {
        self.constructs.extend(constructs);
        self
    }

    /// Add paragraph tab stops.
    #[must_use]
    pub fn tab_stops<I>(mut self, stops: I) -> Self
    where
        I: IntoIterator<Item = TabStop>,
    {
        self.tab_stops.extend(stops);
        self
    }

    /// Set a non-negative first-line indent.
    #[must_use]
    pub const fn first_line_indent(mut self, indent: i32) -> Self {
        self.first_line_indent = indent;
        self
    }

    /// Set line alignment.
    #[must_use]
    pub const fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set final-line widow control.
    #[must_use]
    pub const fn widow(mut self, widow: Widow) -> Self {
        self.widow = widow;
        self
    }

    /// Set logical-to-physical writing mode.
    #[must_use]
    pub const fn writing_mode(mut self, writing_mode: WritingMode) -> Self {
        self.writing_mode = writing_mode;
        self
    }

    /// Validate all cross-input invariants.
    pub fn build(mut self) -> Result<Paragraph, InputError> {
        if self.line_extent <= 0 {
            return Err(InputError::new(
                "input.invalid-line-extent",
                None,
                "line extent must be positive",
            ));
        }
        if self.first_line_indent < 0 || self.first_line_indent >= self.line_extent {
            return Err(InputError::new(
                "input.invalid-indent",
                None,
                "first-line indent must be non-negative and smaller than the line extent",
            ));
        }

        validate_constructs(&self.text, &self.constructs)?;
        validate_breaks(&self.text, &self.constructs, &mut self.breaks)?;
        validate_construct_breaks(&self.text, &self.constructs, &self.breaks)?;
        validate_tabs(
            &self.text,
            &self.breaks,
            self.line_extent,
            &mut self.tab_stops,
        )?;

        Ok(Paragraph {
            text: self.text,
            line_extent: self.line_extent,
            breaks: self.breaks,
            constructs: self.constructs,
            tab_stops: self.tab_stops,
            first_line_indent: self.first_line_indent,
            alignment: self.alignment,
            widow: self.widow,
            writing_mode: self.writing_mode,
        })
    }
}

fn validate_constructs(text: &ShapedText, constructs: &[Construct]) -> Result<(), InputError> {
    for construct in constructs {
        let range = construct.range();
        if range.start >= range.end || range.end > text.source().len() {
            return Err(InputError::new(
                "input.construct-out-of-range",
                Some(range),
                "a construct is empty or outside the paragraph source",
            ));
        }
        if !text.cluster_boundary(range.start) || !text.cluster_boundary(range.end) {
            return Err(InputError::new(
                "input.construct-splits-cluster",
                Some(range),
                "construct endpoints must be shaped-cluster boundaries",
            ));
        }
        if let ConstructKind::Ruby(ruby) = construct.kind() {
            for run in ruby.runs() {
                if !text.cluster_boundary(run.base().start)
                    || !text.cluster_boundary(run.base().end)
                {
                    return Err(InputError::new(
                        "input.ruby-run-splits-cluster",
                        Some(run.base()),
                        "ruby base runs must end at shaped-cluster boundaries",
                    ));
                }
                if ruby.kind() == crate::RubyKind::Mono
                    && text
                        .clusters()
                        .iter()
                        .filter(|cluster| {
                            let cluster = cluster.range();
                            run.base().start <= cluster.start && cluster.end <= run.base().end
                        })
                        .count()
                        != 1
                {
                    return Err(InputError::new(
                        "input.mono-ruby-run-shape",
                        Some(run.base()),
                        "each mono-ruby run must cover exactly one shaped base cluster",
                    ));
                }
            }
        }
        match construct.kind() {
            ConstructKind::Furawake { columns, .. } if *columns == 0 => {
                return Err(InputError::new(
                    "input.invalid-furawake-columns",
                    Some(range),
                    "furawake needs at least one column",
                ));
            },
            ConstructKind::Furawake { line_gap, .. } if *line_gap < 0 => {
                return Err(InputError::new(
                    "input.invalid-furawake-line-gap",
                    Some(range),
                    "furawake line gap must not be negative",
                ));
            },
            ConstructKind::Jidori { cells, .. } if *cells == 0 => {
                return Err(InputError::new(
                    "input.invalid-jidori-cells",
                    Some(range),
                    "jidori needs at least one cell",
                ));
            },
            _ => {},
        }
    }

    for (index, left) in constructs.iter().enumerate() {
        let left = left.range();
        let following = index.saturating_add(1);
        let Some(rest) = constructs.get(following..) else {
            continue;
        };
        for right in rest {
            let right = right.range();
            let crosses =
                (left.start < right.start && right.start < left.end && left.end < right.end)
                    || (right.start < left.start && left.start < right.end && right.end < left.end);
            if crosses {
                return Err(InputError::new(
                    "input.crossing-constructs",
                    Some(left.start.min(right.start)..left.end.max(right.end)),
                    "inline construct ranges may nest or be disjoint, but may not cross",
                ));
            }
        }
    }
    Ok(())
}

fn validate_breaks(
    text: &ShapedText,
    constructs: &[Construct],
    breaks: &mut Vec<Break>,
) -> Result<(), InputError> {
    let end = text.source().len();
    for opportunity in &*breaks {
        if opportunity.offset > end || !text.cluster_boundary(opportunity.offset) {
            return Err(InputError::new(
                "input.break-splits-cluster",
                Some(opportunity.offset..opportunity.offset),
                "break offsets must be shaped-cluster boundaries",
            ));
        }
        for construct in constructs {
            let range = construct.range();
            if range.start < opportunity.offset
                && opportunity.offset < range.end
                && !construct_allows_break(text, construct, opportunity.offset)
            {
                return Err(InputError::new(
                    "input.break-inside-construct",
                    Some(opportunity.offset..opportunity.offset),
                    "this inline structure is indivisible at the requested break",
                ));
            }
        }
    }
    for cluster in text.clusters() {
        if &text.source()[cluster.range()] != "\t" {
            continue;
        }
        let offset = cluster.range().start;
        let inside_construct = constructs.iter().any(|construct| {
            let range = construct.range();
            range.start < offset && offset < range.end
        });
        if offset != 0
            && !inside_construct
            && !breaks
                .iter()
                .any(|opportunity| opportunity.offset == offset)
        {
            breaks.push(Break::allowed(offset));
        }
    }
    breaks.retain(|opportunity| opportunity.offset != 0);
    breaks.sort_by_key(|opportunity| opportunity.offset);
    if breaks
        .windows(2)
        .any(|pair| pair[0].offset == pair[1].offset)
    {
        return Err(InputError::new(
            "input.duplicate-break",
            None,
            "each byte offset may carry only one break kind",
        ));
    }
    if breaks.last().is_none_or(|last| last.offset != end) {
        breaks.push(Break::mandatory(end));
    } else if let Some(last) = breaks.last_mut() {
        last.kind = BreakKind::Mandatory;
    }
    Ok(())
}

fn construct_allows_break(text: &ShapedText, construct: &Construct, at: usize) -> bool {
    match construct.kind() {
        ConstructKind::Ruby(ruby) => {
            ruby.kind() != crate::RubyKind::Group
                && ruby.runs().iter().any(|run| run.base().end == at)
        },
        ConstructKind::Emphasis { .. }
        | ConstructKind::Warichu(_)
        | ConstructKind::Furawake { .. } => true,
        ConstructKind::Formula(range) => {
            let before = text
                .clusters()
                .iter()
                .find(|cluster| cluster.range().end == at)
                .and_then(|cluster| single_cluster_character(text, cluster));
            let after = text
                .clusters()
                .iter()
                .find(|cluster| cluster.range().start == at)
                .and_then(|cluster| single_cluster_character(text, cluster));
            range.start < at
                && at < range.end
                && (before.is_some_and(is_math_token) || after.is_some_and(is_math_token))
        },
        _ => false,
    }
}

fn validate_construct_breaks(
    text: &ShapedText,
    constructs: &[Construct],
    breaks: &[Break],
) -> Result<(), InputError> {
    for construct in constructs {
        let ConstructKind::Furawake { range, columns, .. } = construct.kind() else {
            continue;
        };
        let split_count = breaks
            .iter()
            .filter(|opportunity| {
                range.start < opportunity.offset && opportunity.offset < range.end
            })
            .count();
        if split_count != usize::from(columns.saturating_sub(1)) {
            return Err(InputError::new(
                "input.furawake-split-count",
                Some(range.clone()),
                "furawake needs exactly one declared split between adjacent sublines",
            ));
        }
        if usize::from(*columns)
            > text
                .clusters()
                .iter()
                .filter(|cluster| {
                    range.start <= cluster.range().start && cluster.range().end <= range.end
                })
                .count()
        {
            return Err(InputError::new(
                "input.furawake-empty-subline",
                Some(range.clone()),
                "every furawake subline must contain at least one shaped cluster",
            ));
        }
    }
    Ok(())
}

fn single_cluster_character(text: &ShapedText, cluster: &crate::Cluster) -> Option<char> {
    let mut characters = text.source()[cluster.range()].chars();
    let character = characters.next()?;
    characters.next().is_none().then_some(character)
}

fn validate_tabs(
    text: &ShapedText,
    breaks: &[Break],
    line_extent: i32,
    stops: &mut Vec<TabStop>,
) -> Result<(), InputError> {
    stops.sort_by(|left, right| {
        left.position
            .partial_cmp(&right.position)
            .unwrap_or(Ordering::Equal)
    });
    for stop in &*stops {
        if stop.position >= line_extent {
            return Err(InputError::new(
                "input.tab-stop-outside-line",
                None,
                "tab stops must be inside the line extent",
            ));
        }
    }
    if stops
        .windows(2)
        .any(|pair| pair[0].position == pair[1].position)
    {
        return Err(InputError::new(
            "input.duplicate-tab-stop",
            None,
            "tab stop positions must be unique",
        ));
    }
    let mut tab_count = 0_usize;
    for cluster in text.clusters() {
        if &text.source()[cluster.range()] == "\t" {
            tab_count = tab_count.saturating_add(1);
            if tab_count > stops.len() {
                return Err(InputError::new(
                    "input.insufficient-tab-stops",
                    Some(cluster.range()),
                    "each tab character needs a declared stop on its mandatory-delimited line",
                ));
            }
        }
        if breaks.iter().any(|opportunity| {
            opportunity.offset() == cluster.range().end && opportunity.is_mandatory()
        }) {
            tab_count = 0;
        }
    }
    Ok(())
}
