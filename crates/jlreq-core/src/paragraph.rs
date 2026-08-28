// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::{vec, vec::Vec};

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

/// A completely validated paragraph ready for exact, resource-bounded composition.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Paragraph {
    pub(crate) text: ShapedText,
    pub(crate) line_extent: i32,
    pub(crate) breaks: Vec<Break>,
    pub(crate) constructs: Vec<Construct>,
    pub(crate) tab_stops: Vec<TabStop>,
    pub(crate) line_tabs: Vec<bool>,
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
        let line_tabs = line_tab_mask(&self.text, &self.constructs, self.writing_mode);
        validate_breaks(&self.text, &self.constructs, &line_tabs, &mut self.breaks)?;
        validate_construct_breaks(&self.text, &self.constructs, &self.breaks)?;
        validate_tabs(
            &self.text,
            &self.breaks,
            &line_tabs,
            self.line_extent,
            &mut self.tab_stops,
        )?;

        Ok(Paragraph {
            text: self.text,
            line_extent: self.line_extent,
            breaks: self.breaks,
            constructs: self.constructs,
            tab_stops: self.tab_stops,
            line_tabs,
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
                        .cluster_ordinal(run.base().end)
                        .unwrap_or(usize::MAX)
                        .saturating_sub(
                            text.cluster_ordinal(run.base().start).unwrap_or(usize::MAX),
                        )
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

    let mut ordered: Vec<_> = constructs.iter().map(Construct::range).collect();
    ordered.sort_by(|left, right| {
        left.start
            .cmp(&right.start)
            .then_with(|| right.end.cmp(&left.end))
    });
    let mut stack = Vec::new();
    for current in ordered {
        while stack
            .last()
            .is_some_and(|open: &core::ops::Range<usize>| open.end <= current.start)
        {
            stack.pop();
        }
        if let Some(open) = stack.last() {
            if current.end > open.end {
                return Err(InputError::new(
                    "input.crossing-constructs",
                    Some(open.start.min(current.start)..open.end.max(current.end)),
                    "inline construct ranges may nest or be disjoint, but may not cross",
                ));
            }
        }
        stack.push(current);
    }
    Ok(())
}

/// Whether `construct` sets its text somewhere other than along the line, so that the
/// line holds it at one position however many characters it holds.
///
/// A tate-chu-yoko run runs across the line and a warichu's and a furawake's sublines
/// run beside it; a tate-chu-yoko construct in horizontal composition sets its text
/// along the line like any other text, which is what the rest of this engine reads it
/// as (`docs/decisions/tab-line-correspondence.md`).
fn stacks_text_off_the_line(construct: &Construct, writing_mode: WritingMode) -> bool {
    match construct.kind() {
        ConstructKind::TateChuYoko(_) => writing_mode == WritingMode::VerticalRl,
        ConstructKind::Warichu(_) | ConstructKind::Furawake { .. } => true,
        _ => false,
    }
}

fn validate_breaks(
    text: &ShapedText,
    constructs: &[Construct],
    line_tabs: &[bool],
    breaks: &mut Vec<Break>,
) -> Result<(), InputError> {
    let end = text.source().len();
    let blocked = blocked_break_boundaries(text, constructs);
    for opportunity in &*breaks {
        if opportunity.offset > end || !text.cluster_boundary(opportunity.offset) {
            return Err(InputError::new(
                "input.break-splits-cluster",
                Some(opportunity.offset..opportunity.offset),
                "break offsets must be shaped-cluster boundaries",
            ));
        }
        let ordinal = text
            .cluster_ordinal(opportunity.offset)
            .unwrap_or(usize::MAX);
        if blocked.get(ordinal).copied().unwrap_or(false) {
            return Err(InputError::new(
                "input.break-inside-construct",
                Some(opportunity.offset..opportunity.offset),
                "this inline structure is indivisible at the requested break",
            ));
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
    let mut generated_tab_breaks = Vec::new();
    for (ordinal, cluster) in text.clusters().iter().enumerate() {
        if !line_tabs.get(ordinal).copied().unwrap_or(false) {
            continue;
        }
        let offset = cluster.range().start;
        if offset != 0
            && !blocked.get(ordinal).copied().unwrap_or(false)
            && breaks
                .binary_search_by_key(&offset, |opportunity| opportunity.offset)
                .is_err()
        {
            generated_tab_breaks.push(Break::allowed(offset));
        }
    }
    breaks.extend(generated_tab_breaks);
    breaks.sort_by_key(|opportunity| opportunity.offset);
    if breaks.last().is_none_or(|last| last.offset != end) {
        breaks.push(Break::mandatory(end));
    } else if let Some(last) = breaks.last_mut() {
        last.kind = BreakKind::Mandatory;
    }
    Ok(())
}

fn blocked_break_boundaries(text: &ShapedText, constructs: &[Construct]) -> Vec<bool> {
    let boundary_count = text.clusters().len().saturating_add(1);
    let mut ordinary = vec![0_i32; boundary_count];
    let mut formula = vec![0_i32; boundary_count];
    let mut allowed = vec![0_i32; boundary_count];

    for construct in constructs {
        let range = construct.range();
        let (Some(start), Some(end)) = (
            text.cluster_ordinal(range.start),
            text.cluster_ordinal(range.end),
        ) else {
            continue;
        };
        match construct.kind() {
            ConstructKind::Emphasis { .. }
            | ConstructKind::Warichu(_)
            | ConstructKind::Furawake { .. } => {},
            ConstructKind::Formula(_) => add_interior_range(&mut formula, start, end),
            ConstructKind::Ruby(ruby) => {
                add_interior_range(&mut ordinary, start, end);
                if ruby.kind() != crate::RubyKind::Group {
                    for run in ruby.runs() {
                        if let Some(boundary) = text.cluster_ordinal(run.base().end) {
                            if let Some(value) = allowed.get_mut(boundary) {
                                *value = value.saturating_add(1);
                            }
                        }
                    }
                }
            },
            _ => add_interior_range(&mut ordinary, start, end),
        }
    }

    let mut ordinary_depth = 0_i32;
    let mut formula_depth = 0_i32;
    (0..boundary_count)
        .map(|boundary| {
            ordinary_depth = ordinary_depth.saturating_add(ordinary[boundary]);
            formula_depth = formula_depth.saturating_add(formula[boundary]);
            let ordinary_blocked = ordinary_depth.saturating_sub(allowed[boundary]) > 0;
            ordinary_blocked || (formula_depth > 0 && !boundary_touches_math_token(text, boundary))
        })
        .collect()
}

fn add_interior_range(difference: &mut [i32], start: usize, end: usize) {
    let interior_start = start.saturating_add(1);
    if interior_start >= end {
        return;
    }
    if let Some(value) = difference.get_mut(interior_start) {
        *value = value.saturating_add(1);
    }
    if let Some(value) = difference.get_mut(end) {
        *value = value.saturating_sub(1);
    }
}

fn boundary_touches_math_token(text: &ShapedText, boundary: usize) -> bool {
    boundary
        .checked_sub(1)
        .and_then(|ordinal| text.clusters().get(ordinal))
        .and_then(|cluster| single_cluster_character(text, cluster))
        .is_some_and(is_math_token)
        || text
            .clusters()
            .get(boundary)
            .and_then(|cluster| single_cluster_character(text, cluster))
            .is_some_and(is_math_token)
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
        let first_split = breaks.partition_point(|opportunity| opportunity.offset <= range.start);
        let after_last_split = breaks.partition_point(|opportunity| opportunity.offset < range.end);
        let split_count = after_last_split.saturating_sub(first_split);
        if split_count != usize::from(columns.saturating_sub(1)) {
            return Err(InputError::new(
                "input.furawake-split-count",
                Some(range.clone()),
                "furawake needs exactly one declared split between adjacent sublines",
            ));
        }
        let cluster_count = text
            .cluster_ordinal(range.end)
            .unwrap_or(0)
            .saturating_sub(text.cluster_ordinal(range.start).unwrap_or(0));
        if usize::from(*columns) > cluster_count {
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
    line_tabs: &[bool],
    line_extent: i32,
    stops: &mut Vec<TabStop>,
) -> Result<(), InputError> {
    stops.sort_by_key(|stop| stop.position);
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
    let mut mandatory = breaks
        .iter()
        .filter(|opportunity| opportunity.is_mandatory())
        .peekable();
    for (ordinal, cluster) in text.clusters().iter().enumerate() {
        if line_tabs.get(ordinal).copied().unwrap_or(false) {
            tab_count = tab_count.saturating_add(1);
            if tab_count > stops.len() {
                return Err(InputError::new(
                    "input.insufficient-tab-stops",
                    Some(cluster.range()),
                    "each tab character needs a declared stop on its mandatory-delimited line",
                ));
            }
        }
        if mandatory
            .next_if(|opportunity| opportunity.offset() == cluster.range().end)
            .is_some()
        {
            tab_count = 0;
        }
    }
    Ok(())
}

fn line_tab_mask(
    text: &ShapedText,
    constructs: &[Construct],
    writing_mode: WritingMode,
) -> Vec<bool> {
    let cluster_count = text.clusters().len();
    let mut difference = vec![0_i32; cluster_count.saturating_add(1)];
    for construct in constructs {
        if !stacks_text_off_the_line(construct, writing_mode) {
            continue;
        }
        let range = construct.range();
        let (Some(start), Some(end)) = (
            text.cluster_ordinal(range.start),
            text.cluster_ordinal(range.end),
        ) else {
            continue;
        };
        if let Some(value) = difference.get_mut(start) {
            *value = value.saturating_add(1);
        }
        if let Some(value) = difference.get_mut(end) {
            *value = value.saturating_sub(1);
        }
    }

    let mut depth = 0_i32;
    text.clusters()
        .iter()
        .enumerate()
        .map(|(ordinal, cluster)| {
            depth = depth.saturating_add(difference.get(ordinal).copied().unwrap_or(0));
            depth == 0 && &text.source()[cluster.range()] == "\t"
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::construct::{Ruby, RubyKind, RubyRun};
    use crate::model::{Cluster, Frame, Size};

    fn text(source: &str) -> ShapedText {
        let clusters = source.char_indices().map(|(start, character)| {
            Cluster::new(start..start.saturating_add(character.len_utf8()), 500)
        });
        ShapedText::new(
            source,
            Size::square(1_000).expect("positive size"),
            Frame::FullEm,
            clusters,
        )
        .expect("valid shaped text")
    }

    fn proportional_one_cluster(source: &str) -> ShapedText {
        ShapedText::new(
            source,
            Size::square(1_000).expect("positive size"),
            Frame::Proportional,
            [Cluster::new(0..source.len(), 500)],
        )
        .expect("valid proportional cluster")
    }

    #[test]
    fn break_and_paragraph_accessors_preserve_declared_values() {
        let allowed = Break::allowed(1);
        let mandatory = Break::mandatory(2);
        let discretionary = Break::discretionary(3);
        assert_eq!(allowed.offset(), 1);
        assert!(!allowed.is_mandatory());
        assert!(!allowed.is_discretionary());
        assert_eq!(mandatory.offset(), 2);
        assert!(mandatory.is_mandatory());
        assert!(!mandatory.is_discretionary());
        assert_eq!(discretionary.offset(), 3);
        assert!(!discretionary.is_mandatory());
        assert!(discretionary.is_discretionary());

        let paragraph = Paragraph::builder(text("ab"), 2_000)
            .breaks([Break::allowed(1)])
            .constructs([Construct::emphasis_dots(0..1, '・')])
            .tab_stops([TabStop::new(700, TabAlignment::End).expect("valid stop")])
            .first_line_indent(123)
            .alignment(Alignment::End)
            .widow(Widow::MinimumClusters(2))
            .writing_mode(WritingMode::VerticalRl)
            .build()
            .expect("valid paragraph");
        assert_eq!(paragraph.line_extent(), 2_000);
        assert_eq!(paragraph.breaks().len(), 2);
        assert_eq!(paragraph.constructs().len(), 1);
        assert_eq!(paragraph.tab_stops().len(), 1);
        assert_eq!(paragraph.first_line_indent(), 123);
        assert_eq!(paragraph.alignment(), Alignment::End);
        assert_eq!(paragraph.widow(), Widow::MinimumClusters(2));
        assert_eq!(paragraph.writing_mode(), WritingMode::VerticalRl);
    }

    #[test]
    fn indent_and_construct_range_guards_are_independent() {
        for indent in [-1, 1_000] {
            let error = Paragraph::builder(text("a"), 1_000)
                .first_line_indent(indent)
                .build()
                .expect_err("invalid indent");
            assert_eq!(error.code(), "input.invalid-indent");
        }

        for construct in [
            Construct::emphasis_dots(0..0, '・'),
            Construct::emphasis_dots(0..2, '・'),
        ] {
            let error = Paragraph::builder(text("a"), 1_000)
                .constructs([construct])
                .build()
                .expect_err("invalid construct range");
            assert_eq!(error.code(), "input.construct-out-of-range");
        }

        let error = Paragraph::builder(proportional_one_cluster("ab"), 1_000)
            .constructs([Construct::emphasis_dots(0..1, '・')])
            .build()
            .expect_err("construct splits shaped cluster");
        assert_eq!(error.code(), "input.construct-splits-cluster");
    }

    #[test]
    fn ruby_and_special_construct_guards_are_independent() {
        let annotation = text("xy");
        let split_run_ruby = Ruby::new(
            RubyKind::Jukugo,
            0..2,
            annotation.clone(),
            [RubyRun::new(0..1, 0..1), RubyRun::new(1..2, 1..2)],
        )
        .expect("document-level ruby runs");
        let error = Paragraph::builder(proportional_one_cluster("ab"), 1_000)
            .constructs([Construct::ruby(split_run_ruby)])
            .build()
            .expect_err("ruby run splits the shaped base");
        assert_eq!(error.code(), "input.ruby-run-splits-cluster");

        let mono = Ruby::new(RubyKind::Mono, 0..2, text("x"), [RubyRun::new(0..2, 0..1)])
            .expect("document-level mono ruby");
        let error = Paragraph::builder(text("ab"), 1_000)
            .constructs([Construct::ruby(mono)])
            .build()
            .expect_err("mono run covers two clusters");
        assert_eq!(error.code(), "input.mono-ruby-run-shape");

        let cases = [
            (
                Construct::furawake(0..1, 0, 0),
                "input.invalid-furawake-columns",
            ),
            (
                Construct::furawake(0..1, 1, -1),
                "input.invalid-furawake-line-gap",
            ),
            (Construct::jidori(0..1, 0), "input.invalid-jidori-cells"),
        ];
        for (construct, code) in cases {
            let error = Paragraph::builder(text("a"), 1_000)
                .constructs([construct])
                .build()
                .expect_err("invalid special construct");
            assert_eq!(error.code(), code);
        }
    }

    #[test]
    fn construct_stack_accepts_adjacency_and_rejects_crossing() {
        let adjacent = Paragraph::builder(text("abc"), 1_000)
            .constructs([
                Construct::emphasis_dots(0..1, '・'),
                Construct::emphasis_dots(1..2, '・'),
            ])
            .build();
        assert!(adjacent.is_ok());

        let shared_end = Paragraph::builder(text("abc"), 1_000)
            .constructs([
                Construct::emphasis_dots(0..2, '・'),
                Construct::reference_mark(1..2, text("※")),
            ])
            .build();
        assert!(shared_end.is_ok());

        let crossing = Paragraph::builder(text("abc"), 1_000)
            .constructs([
                Construct::emphasis_dots(0..2, '・'),
                Construct::emphasis_dots(1..3, '・'),
            ])
            .build()
            .expect_err("crossing constructs");
        assert_eq!(crossing.code(), "input.crossing-constructs");
    }

    #[test]
    fn generated_tab_breaks_respect_start_blocking_and_existing_offsets() {
        let shaped = text("\tab");
        let mut breaks = vec![Break::allowed(1)];
        let line_tabs = vec![true, true, false];
        validate_breaks(&shaped, &[], &line_tabs, &mut breaks).expect("valid tab breaks");
        assert_eq!(
            breaks
                .iter()
                .map(|opportunity| opportunity.offset())
                .collect::<Vec<_>>(),
            vec![1, 3]
        );

        let shaped = text("a\tb");
        let construct = Construct::tate_chu_yoko(0..3);
        let mut breaks = Vec::new();
        validate_breaks(&shaped, &[construct], &[false, true, false], &mut breaks)
            .expect("blocked tab boundary is not generated");
        assert_eq!(
            breaks
                .iter()
                .map(|opportunity| opportunity.offset())
                .collect::<Vec<_>>(),
            vec![3]
        );
    }

    #[test]
    fn mandatory_break_resets_tab_stop_consumption() {
        let shaped = text("\tA\t");
        let mut stops = vec![TabStop::new(500, TabAlignment::Start).expect("valid stop")];
        validate_tabs(
            &shaped,
            &[Break::mandatory(2), Break::mandatory(3)],
            &[true, false, true],
            1_000,
            &mut stops,
        )
        .expect("one stop is reusable after a mandatory break");

        let before_break = text("\t\tA");
        let error = validate_tabs(
            &before_break,
            &[Break::mandatory(2)],
            &[true, true, false],
            1_000,
            &mut vec![TabStop::new(500, TabAlignment::Start).expect("valid stop")],
        )
        .expect_err("two tabs before the break need two stops");
        assert_eq!(error.code(), "input.insufficient-tab-stops");
    }
}
