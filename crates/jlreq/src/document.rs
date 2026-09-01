// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::ops::Range;

use crate::units::{non_negative, positive};
use crate::{FontStyle, FontVariation, LayoutError, OpenTypeFeature, OptionKind};

const SPAN_RANGE_MESSAGE: &str =
    "a span range must be non-empty, inside the text, and on character boundaries";
const CONSTRUCT_RANGE_MESSAGE: &str =
    "a construct range must be non-empty, inside the text, and on character boundaries";

/// Conservative semantic classification that cannot always be inferred from text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum TextRole {
    /// Ordinary prose.
    #[default]
    Text,
    /// A numeral treated as one grouped Japanese numeral.
    GroupedNumeral,
    /// A unit-symbol character.
    UnitSymbol,
    /// A quantity symbol.
    QuantitySymbol,
    /// Mathematical or chemical notation.
    Formula,
    /// A bracket delimiting warichu.
    WarichuBracket,
}

impl TextRole {
    pub(crate) const fn core(self) -> jlreq_core::ClusterRole {
        match self {
            Self::Text => jlreq_core::ClusterRole::Text,
            Self::GroupedNumeral => jlreq_core::ClusterRole::GroupedNumeral,
            Self::UnitSymbol => jlreq_core::ClusterRole::UnitSymbol,
            Self::QuantitySymbol => jlreq_core::ClusterRole::QuantitySymbol,
            Self::Formula => jlreq_core::ClusterRole::Formula,
            Self::WarichuBracket => jlreq_core::ClusterRole::WarichuBracket,
        }
    }
}

/// Typed style applied to a UTF-8 byte range.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SpanStyle {
    pub(crate) families: Vec<String>,
    pub(crate) font_style: FontStyle,
    pub(crate) font_size: Option<i32>,
    pub(crate) language: Option<String>,
    pub(crate) features: Vec<OpenTypeFeature>,
    pub(crate) variations: Vec<FontVariation>,
    pub(crate) role: TextRole,
}

impl SpanStyle {
    /// Default span, inheriting size/language and using library fallback order.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            families: Vec::new(),
            font_style: FontStyle::new(400, 100, crate::FontSlant::Normal),
            font_size: None,
            language: None,
            features: Vec::new(),
            variations: Vec::new(),
            role: TextRole::Text,
        }
    }

    /// Add a preferred family ahead of normal fallback.
    #[must_use]
    pub fn with_family(mut self, family: impl Into<String>) -> Self {
        self.families.push(family.into());
        self
    }

    /// Replace every preferred family. An empty iterator clears them.
    #[must_use]
    pub fn with_families<I, S>(mut self, families: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.families = families.into_iter().map(Into::into).collect();
        self
    }

    /// Set requested family matching attributes.
    #[must_use]
    pub const fn with_font_style(mut self, style: FontStyle) -> Self {
        self.font_style = style;
        self
    }

    /// Override the main font size for this span.
    pub fn with_font_size(mut self, value: f32) -> Result<Self, LayoutError> {
        self.font_size = Some(positive(value, OptionKind::SpanFontSize)?);
        Ok(self)
    }

    /// Override shaping language for this span.
    pub fn with_language(mut self, value: impl Into<String>) -> Result<Self, LayoutError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 63
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(LayoutError::invalid_option(
                OptionKind::Language,
                "language must be a non-empty ASCII language tag of at most 63 bytes",
            ));
        }
        self.language = Some(value);
        Ok(self)
    }

    /// Add a span-specific OpenType feature.
    #[must_use]
    pub fn with_feature(mut self, value: OpenTypeFeature) -> Self {
        self.features.push(value);
        self
    }

    /// Replace every span-specific OpenType feature. An empty iterator clears them.
    #[must_use]
    pub fn with_features<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = OpenTypeFeature>,
    {
        self.features = values.into_iter().collect();
        self
    }

    /// Add a span-specific variable-font axis value.
    #[must_use]
    pub fn with_variation(mut self, value: FontVariation) -> Self {
        self.variations.push(value);
        self
    }

    /// Replace every span-specific variable-font axis value. An empty iterator clears them.
    #[must_use]
    pub fn with_variations<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = FontVariation>,
    {
        self.variations = values.into_iter().collect();
        self
    }

    /// Assert a semantic classification instead of relying on conservative inference.
    #[must_use]
    pub const fn with_role(mut self, value: TextRole) -> Self {
        self.role = value;
        self
    }

    /// Preferred families ahead of normal fallback, in request order.
    #[must_use]
    pub fn families(&self) -> &[String] {
        &self.families
    }

    /// Requested family matching attributes.
    #[must_use]
    pub const fn font_style(&self) -> FontStyle {
        self.font_style
    }

    /// Span font-size override after 26.6 quantization, when one is set.
    #[must_use]
    pub fn font_size(&self) -> Option<f32> {
        self.font_size.map(crate::units::to_f32)
    }

    /// Span shaping-language override, when one is set.
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// Span-specific OpenType features, in application order.
    #[must_use]
    pub fn features(&self) -> &[OpenTypeFeature] {
        &self.features
    }

    /// Span-specific variable-font axis values, in application order.
    #[must_use]
    pub fn variations(&self) -> &[FontVariation] {
        &self.variations
    }

    /// Asserted semantic classification.
    #[must_use]
    pub const fn role(&self) -> TextRole {
        self.role
    }
}

impl Default for SpanStyle {
    fn default() -> Self {
        Self::new()
    }
}

/// Relationship between ruby base and annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RubyKind {
    /// Each base cluster receives an associated reading run.
    Mono,
    /// One reading belongs to the full base.
    Group,
    /// Reading runs form a compound word.
    Jukugo,
}

impl RubyKind {
    pub(crate) const fn core(self) -> jlreq_core::RubyKind {
        match self {
            Self::Mono => jlreq_core::RubyKind::Mono,
            Self::Group => jlreq_core::RubyKind::Group,
            Self::Jukugo => jlreq_core::RubyKind::Jukugo,
        }
    }
}

/// Explicit association inside mono or jukugo ruby.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RubyRun {
    base: Range<usize>,
    annotation: Range<usize>,
}

impl RubyRun {
    /// Associate a document base range with a range in the annotation string.
    #[must_use]
    pub const fn new(base: Range<usize>, annotation: Range<usize>) -> Self {
        Self { base, annotation }
    }

    /// Document base range.
    #[must_use]
    pub fn base(&self) -> Range<usize> {
        self.base.clone()
    }

    /// Annotation-local range.
    #[must_use]
    pub fn annotation(&self) -> Range<usize> {
        self.annotation.clone()
    }
}

/// Placement side for an automatically shaped script annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ScriptPosition {
    /// Superscript-side placement.
    Superscript,
    /// Subscript-side placement.
    Subscript,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DocumentConstruct {
    Ruby {
        kind: RubyKind,
        base: Range<usize>,
        annotation: String,
        runs: Vec<RubyRun>,
    },
    TateChuYoko(Range<usize>),
    Emphasis {
        range: Range<usize>,
        mark: char,
    },
    Warichu(Range<usize>),
    Furawake {
        range: Range<usize>,
        columns: u16,
        line_gap: i32,
    },
    Jidori {
        range: Range<usize>,
        cells: u16,
    },
    ReferenceMark {
        range: Range<usize>,
        mark: String,
    },
    Script {
        range: Range<usize>,
        annotation: String,
        position: ScriptPosition,
    },
    Formula(Range<usize>),
}

impl DocumentConstruct {
    pub(crate) fn range(&self) -> Range<usize> {
        match self {
            Self::Ruby { base, .. } => base.clone(),
            Self::TateChuYoko(range)
            | Self::Warichu(range)
            | Self::Formula(range)
            | Self::Emphasis { range, .. }
            | Self::Furawake { range, .. }
            | Self::Jidori { range, .. }
            | Self::ReferenceMark { range, .. }
            | Self::Script { range, .. } => range.clone(),
        }
    }

    fn view(&self) -> InlineConstruct<'_> {
        match self {
            Self::Ruby {
                kind,
                base,
                annotation,
                runs,
            } => InlineConstruct::Ruby {
                kind: *kind,
                base: base.clone(),
                annotation,
                runs,
            },
            Self::TateChuYoko(range) => InlineConstruct::TateChuYoko {
                range: range.clone(),
            },
            Self::Emphasis { range, mark } => InlineConstruct::EmphasisDots {
                range: range.clone(),
                mark: *mark,
            },
            Self::Warichu(range) => InlineConstruct::Warichu {
                range: range.clone(),
            },
            Self::Furawake {
                range,
                columns,
                line_gap,
            } => InlineConstruct::Furawake {
                range: range.clone(),
                columns: *columns,
                line_gap: crate::units::to_f32(*line_gap),
            },
            Self::Jidori { range, cells } => InlineConstruct::Jidori {
                range: range.clone(),
                cells: *cells,
            },
            Self::ReferenceMark { range, mark } => InlineConstruct::ReferenceMark {
                range: range.clone(),
                mark,
            },
            Self::Script {
                range,
                annotation,
                position,
            } => InlineConstruct::Script {
                range: range.clone(),
                annotation,
                position: *position,
            },
            Self::Formula(range) => InlineConstruct::Formula {
                range: range.clone(),
            },
        }
    }
}

/// One typed inline construct read back from a [`Document`].
///
/// Values borrow from the document and mirror what the corresponding
/// [`DocumentBuilder`] call accepted, so a document can be inspected,
/// diffed, or serialized by its consumer.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum InlineConstruct<'a> {
    /// Ruby annotation over a base range.
    Ruby {
        /// Relationship between base and annotation.
        kind: RubyKind,
        /// Base UTF-8 range in the document text.
        base: Range<usize>,
        /// Annotation string.
        annotation: &'a str,
        /// Explicit runs; empty when associations are derived automatically.
        runs: &'a [RubyRun],
    },
    /// A range kept horizontal in vertical text (縦中横).
    TateChuYoko {
        /// UTF-8 range in the document text.
        range: Range<usize>,
    },
    /// An emphasis mark repeated alongside every base cluster (圏点).
    EmphasisDots {
        /// UTF-8 range in the document text.
        range: Range<usize>,
        /// The repeated mark.
        mark: char,
    },
    /// An inline cutting note (割注).
    Warichu {
        /// UTF-8 range in the document text.
        range: Range<usize>,
    },
    /// A range distributed over aligned sublines (振分け).
    Furawake {
        /// UTF-8 range in the document text.
        range: Range<usize>,
        /// Number of sublines.
        columns: u16,
        /// Extra block-axis distance between sublines.
        line_gap: f32,
    },
    /// A range fit into a fixed number of full-em cells (字取り).
    Jidori {
        /// UTF-8 range in the document text.
        range: Range<usize>,
        /// Cell count.
        cells: u16,
    },
    /// An attached reference mark (合印).
    ReferenceMark {
        /// UTF-8 range in the document text.
        range: Range<usize>,
        /// The mark string.
        mark: &'a str,
    },
    /// Attached superscript or subscript text (添字).
    Script {
        /// UTF-8 range in the document text.
        range: Range<usize>,
        /// Annotation string.
        annotation: &'a str,
        /// Placement side.
        position: ScriptPosition,
    },
    /// A range marked as mathematical content (数式).
    Formula {
        /// UTF-8 range in the document text.
        range: Range<usize>,
    },
}

impl InlineConstruct<'_> {
    /// Document UTF-8 range the construct covers.
    ///
    /// For ruby this is the base range.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        match self {
            Self::Ruby { base, .. } => base.clone(),
            Self::TateChuYoko { range }
            | Self::EmphasisDots { range, .. }
            | Self::Warichu { range }
            | Self::Furawake { range, .. }
            | Self::Jidori { range, .. }
            | Self::ReferenceMark { range, .. }
            | Self::Script { range, .. }
            | Self::Formula { range } => range.clone(),
        }
    }
}

/// Validated styled text and typed inline constructs.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Document {
    pub(crate) text: String,
    pub(crate) spans: Vec<(Range<usize>, SpanStyle)>,
    pub(crate) constructs: Vec<DocumentConstruct>,
    pub(crate) mandatory_breaks: Vec<usize>,
    pub(crate) prohibited_breaks: Vec<usize>,
}

impl Document {
    /// Original UTF-8 source.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Number of typed inline constructs.
    #[must_use]
    pub const fn construct_count(&self) -> usize {
        self.constructs.len()
    }

    /// Styled spans with their ranges, ascending by start offset.
    pub fn spans(&self) -> impl Iterator<Item = (Range<usize>, &SpanStyle)> {
        self.spans
            .iter()
            .map(|(range, style)| (range.clone(), style))
    }

    /// Typed inline constructs, ascending by range.
    ///
    /// The iteration order matches the construct ordinals reported by
    /// [`crate::AnnotationSource::construct`].
    pub fn constructs(&self) -> impl Iterator<Item = InlineConstruct<'_>> {
        self.constructs.iter().map(DocumentConstruct::view)
    }

    /// One typed inline construct by ordinal, when it exists.
    #[must_use]
    pub fn construct(&self, ordinal: usize) -> Option<InlineConstruct<'_>> {
        self.constructs.get(ordinal).map(DocumentConstruct::view)
    }

    /// Required break offsets, ascending and deduplicated.
    #[must_use]
    pub fn mandatory_breaks(&self) -> &[usize] {
        &self.mandatory_breaks
    }

    /// Removed automatic break opportunities, ascending and deduplicated.
    #[must_use]
    pub fn prohibited_breaks(&self) -> &[usize] {
        &self.prohibited_breaks
    }
}

/// Incrementally builds a [`Document`] while preserving UTF-8 ranges.
#[derive(Debug, Clone)]
pub struct DocumentBuilder {
    text: String,
    spans: Vec<(Range<usize>, SpanStyle)>,
    constructs: Vec<DocumentConstruct>,
    mandatory_breaks: Vec<usize>,
    prohibited_breaks: Vec<usize>,
}

impl DocumentBuilder {
    /// Start a document around owned UTF-8 text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            spans: Vec::new(),
            constructs: Vec::new(),
            mandatory_breaks: Vec::new(),
            prohibited_breaks: Vec::new(),
        }
    }

    /// Add a non-overlapping span style.
    pub fn span(
        &mut self,
        range: Range<usize>,
        style: SpanStyle,
    ) -> Result<&mut Self, LayoutError> {
        self.validate_non_empty_range(&range, "document.invalid-span-range", SPAN_RANGE_MESSAGE)?;
        let insertion = span_insertion_point(&self.spans, range.start);
        let overlaps_previous = insertion
            .checked_sub(1)
            .and_then(|index| self.spans.get(index))
            .is_some_and(|(other, _)| ranges_overlap(other, &range));
        let overlaps_next = self
            .spans
            .get(insertion)
            .is_some_and(|(other, _)| ranges_overlap(other, &range));
        if overlaps_previous || overlaps_next {
            return Err(LayoutError::invalid_document(
                "document.overlapping-spans",
                Some(range),
                "span styles must not overlap",
            ));
        }
        self.spans.insert(insertion, (range, style));
        Ok(self)
    }

    /// Require a break at a UTF-8 boundary inside a paragraph.
    pub fn mandatory_break(&mut self, offset: usize) -> Result<&mut Self, LayoutError> {
        self.validate_offset(offset, "document.invalid-break")?;
        self.mandatory_breaks.push(offset);
        Ok(self)
    }

    /// Remove an otherwise automatic UAX #14 opportunity.
    pub fn prohibit_break(&mut self, offset: usize) -> Result<&mut Self, LayoutError> {
        self.validate_offset(offset, "document.invalid-break")?;
        self.prohibited_breaks.push(offset);
        Ok(self)
    }

    /// Add ruby with explicit or automatically derived associations.
    pub fn ruby<I>(
        &mut self,
        kind: RubyKind,
        base: Range<usize>,
        annotation: impl Into<String>,
        runs: I,
    ) -> Result<&mut Self, LayoutError>
    where
        I: IntoIterator<Item = RubyRun>,
    {
        self.validate_non_empty_range(
            &base,
            "document.invalid-construct-range",
            CONSTRUCT_RANGE_MESSAGE,
        )?;
        let annotation = annotation.into();
        if annotation.is_empty() {
            return Err(LayoutError::invalid_document(
                "document.empty-ruby-annotation",
                Some(base),
                "a ruby annotation must contain at least one character",
            ));
        }
        self.constructs.push(DocumentConstruct::Ruby {
            kind,
            base,
            annotation,
            runs: runs.into_iter().collect(),
        });
        Ok(self)
    }

    /// Add automatically associated group ruby.
    pub fn group_ruby(
        &mut self,
        base: Range<usize>,
        annotation: impl Into<String>,
    ) -> Result<&mut Self, LayoutError> {
        self.ruby(RubyKind::Group, base, annotation, [])
    }

    /// Add automatically partitioned mono ruby.
    pub fn mono_ruby(
        &mut self,
        base: Range<usize>,
        annotation: impl Into<String>,
    ) -> Result<&mut Self, LayoutError> {
        self.ruby(RubyKind::Mono, base, annotation, [])
    }

    /// Add automatically associated jukugo ruby.
    pub fn jukugo_ruby(
        &mut self,
        base: Range<usize>,
        annotation: impl Into<String>,
    ) -> Result<&mut Self, LayoutError> {
        self.ruby(RubyKind::Jukugo, base, annotation, [])
    }

    /// Keep a range horizontal in vertical text.
    pub fn tate_chu_yoko(&mut self, range: Range<usize>) -> Result<&mut Self, LayoutError> {
        self.push_range_construct(range, DocumentConstruct::TateChuYoko)
    }

    /// Repeat an emphasis mark alongside every base cluster.
    pub fn emphasis_dots(
        &mut self,
        range: Range<usize>,
        mark: char,
    ) -> Result<&mut Self, LayoutError> {
        self.validate_non_empty_range(
            &range,
            "document.invalid-construct-range",
            CONSTRUCT_RANGE_MESSAGE,
        )?;
        self.constructs
            .push(DocumentConstruct::Emphasis { range, mark });
        Ok(self)
    }

    /// Add an inline cutting note (割注).
    pub fn warichu(&mut self, range: Range<usize>) -> Result<&mut Self, LayoutError> {
        self.push_range_construct(range, DocumentConstruct::Warichu)
    }

    /// Distribute a range over aligned sublines (振分け).
    ///
    /// When no [`mandatory_break`](Self::mandatory_break) falls strictly
    /// inside the range, layout balances the range's shaped clusters across
    /// the requested columns automatically, earlier columns taking the
    /// remainder. Supplying any break inside the range takes over completely:
    /// exactly `columns - 1` splits are then required, and layout reports
    /// `input.furawake-split-count` otherwise. Automatic balancing needs at
    /// least one cluster per column and every synthesized split to be a legal
    /// break point (a split cannot fall inside an indivisible nested
    /// construct); outside those bounds, supply the splits explicitly.
    pub fn furawake(
        &mut self,
        range: Range<usize>,
        columns: u16,
        line_gap: f32,
    ) -> Result<&mut Self, LayoutError> {
        self.validate_non_empty_range(
            &range,
            "document.invalid-construct-range",
            CONSTRUCT_RANGE_MESSAGE,
        )?;
        if columns < 2 {
            return Err(LayoutError::invalid_document(
                "document.invalid-furawake-columns",
                Some(range),
                "furawake requires at least two columns",
            ));
        }
        self.constructs.push(DocumentConstruct::Furawake {
            range,
            columns,
            line_gap: non_negative(line_gap, OptionKind::ConstructGeometry)?,
        });
        Ok(self)
    }

    /// Fit a range in a fixed number of full-em cells (字取り).
    pub fn jidori(&mut self, range: Range<usize>, cells: u16) -> Result<&mut Self, LayoutError> {
        self.validate_non_empty_range(
            &range,
            "document.invalid-construct-range",
            CONSTRUCT_RANGE_MESSAGE,
        )?;
        if cells == 0 {
            return Err(LayoutError::invalid_document(
                "document.invalid-jidori-cells",
                Some(range),
                "jidori requires at least one cell",
            ));
        }
        self.constructs
            .push(DocumentConstruct::Jidori { range, cells });
        Ok(self)
    }

    /// Attach an automatically shaped reference mark (合印).
    pub fn reference_mark(
        &mut self,
        range: Range<usize>,
        mark: impl Into<String>,
    ) -> Result<&mut Self, LayoutError> {
        self.validate_non_empty_range(
            &range,
            "document.invalid-construct-range",
            CONSTRUCT_RANGE_MESSAGE,
        )?;
        let mark = mark.into();
        if mark.is_empty() {
            return Err(LayoutError::invalid_document(
                "document.empty-reference-mark",
                Some(range),
                "a reference mark must contain at least one character",
            ));
        }
        self.constructs
            .push(DocumentConstruct::ReferenceMark { range, mark });
        Ok(self)
    }

    /// Attach automatically shaped superscript or subscript text.
    pub fn script(
        &mut self,
        range: Range<usize>,
        annotation: impl Into<String>,
        position: ScriptPosition,
    ) -> Result<&mut Self, LayoutError> {
        self.validate_non_empty_range(
            &range,
            "document.invalid-construct-range",
            CONSTRUCT_RANGE_MESSAGE,
        )?;
        let annotation = annotation.into();
        if annotation.is_empty() {
            return Err(LayoutError::invalid_document(
                "document.empty-script-annotation",
                Some(range),
                "a script annotation must contain at least one character",
            ));
        }
        self.constructs.push(DocumentConstruct::Script {
            range,
            annotation,
            position,
        });
        Ok(self)
    }

    /// Mark a pre-existing range as mathematical content.
    pub fn formula(&mut self, range: Range<usize>) -> Result<&mut Self, LayoutError> {
        self.push_range_construct(range, DocumentConstruct::Formula)
    }

    /// Validate cross-field relationships and finish the document atomically.
    pub fn build(mut self) -> Result<Document, LayoutError> {
        self.spans.sort_by_key(|(range, _)| range.start);
        self.constructs.sort_by_key(|construct| {
            let range = construct.range();
            (range.start, range.end)
        });
        self.mandatory_breaks.sort_unstable();
        self.mandatory_breaks.dedup();
        self.prohibited_breaks.sort_unstable();
        self.prohibited_breaks.dedup();
        if let Some(offset) = self
            .mandatory_breaks
            .iter()
            .find(|offset| self.prohibited_breaks.binary_search(offset).is_ok())
        {
            return Err(LayoutError::invalid_document(
                "document.conflicting-break",
                Some(*offset..*offset),
                "an offset cannot be both a mandatory and a prohibited break",
            ));
        }
        for construct in &self.constructs {
            if let DocumentConstruct::Ruby {
                kind,
                base,
                annotation,
                runs,
            } = construct
            {
                validate_ruby_runs(&self.text, *kind, base, annotation, runs)?;
            }
        }
        Ok(Document {
            text: self.text,
            spans: self.spans,
            constructs: self.constructs,
            mandatory_breaks: self.mandatory_breaks,
            prohibited_breaks: self.prohibited_breaks,
        })
    }

    fn validate_non_empty_range(
        &self,
        range: &Range<usize>,
        code: &'static str,
        message: &'static str,
    ) -> Result<(), LayoutError> {
        if range.start >= range.end
            || range.end > self.text.len()
            || !self.text.is_char_boundary(range.start)
            || !self.text.is_char_boundary(range.end)
        {
            return Err(LayoutError::invalid_document(
                code,
                Some(range.clone()),
                message,
            ));
        }
        Ok(())
    }

    fn validate_offset(&self, offset: usize, code: &'static str) -> Result<(), LayoutError> {
        if offset == 0 || offset >= self.text.len() || !self.text.is_char_boundary(offset) {
            return Err(LayoutError::invalid_document(
                code,
                Some(offset..offset),
                "a break offset must be a character boundary strictly inside the text",
            ));
        }
        Ok(())
    }

    fn push_range_construct(
        &mut self,
        range: Range<usize>,
        constructor: fn(Range<usize>) -> DocumentConstruct,
    ) -> Result<&mut Self, LayoutError> {
        self.validate_non_empty_range(
            &range,
            "document.invalid-construct-range",
            CONSTRUCT_RANGE_MESSAGE,
        )?;
        self.constructs.push(constructor(range));
        Ok(self)
    }
}

fn span_insertion_point(spans: &[(Range<usize>, SpanStyle)], start: usize) -> usize {
    spans.partition_point(|(other, _)| other.start < start)
}

fn validate_ruby_runs(
    text: &str,
    kind: RubyKind,
    base: &Range<usize>,
    annotation: &str,
    runs: &[RubyRun],
) -> Result<(), LayoutError> {
    if runs.is_empty() {
        return Ok(());
    }
    if kind == RubyKind::Group && runs.len() != 1 {
        return Err(LayoutError::invalid_document(
            "document.group-ruby-run-count",
            Some(base.clone()),
            "group ruby accepts exactly one explicit run",
        ));
    }
    let mut base_cursor = base.start;
    let mut annotation_cursor = 0;
    for run in runs {
        if run.base.start != base_cursor
            || run.base.end > base.end
            || run.base.start >= run.base.end
            || !text.is_char_boundary(run.base.start)
            || !text.is_char_boundary(run.base.end)
            || run.annotation.start != annotation_cursor
            || run.annotation.end > annotation.len()
            || run.annotation.start >= run.annotation.end
            || !annotation.is_char_boundary(run.annotation.start)
            || !annotation.is_char_boundary(run.annotation.end)
        {
            return Err(LayoutError::invalid_document(
                "document.invalid-ruby-run",
                Some(run.base.clone()),
                "ruby runs must advance through the base and annotation on character boundaries",
            ));
        }
        base_cursor = run.base.end;
        annotation_cursor = run.annotation.end;
    }
    if base_cursor != base.end || annotation_cursor != annotation.len() {
        return Err(LayoutError::invalid_document(
            "document.incomplete-ruby-runs",
            Some(base.clone()),
            "explicit ruby runs must cover the whole base and annotation",
        ));
    }
    Ok(())
}

fn ranges_overlap(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FontSlant, OpenTypeTag};

    fn ruby_error(
        text: &str,
        kind: RubyKind,
        base: Range<usize>,
        annotation: &str,
        runs: &[RubyRun],
    ) -> &'static str {
        validate_ruby_runs(text, kind, &base, annotation, runs)
            .expect_err("the run must be rejected")
            .code()
    }

    #[test]
    fn span_style_builders_preserve_every_supplied_value_and_language_boundary() {
        let feature = OpenTypeFeature::new(OpenTypeTag::try_new("liga").unwrap(), 0);
        let variation =
            FontVariation::try_new(OpenTypeTag::try_new("wght").unwrap(), 525.0).unwrap();
        let font_style = FontStyle::new(525, 90, FontSlant::Italic);
        let style = SpanStyle::new()
            .with_family("first")
            .with_family("second")
            .with_font_style(font_style)
            .with_font_size(17.25)
            .unwrap()
            .with_language("ja-Latn-JP")
            .unwrap()
            .with_feature(feature)
            .with_variation(variation)
            .with_role(TextRole::Formula);

        assert_eq!(style.families(), ["first", "second"]);
        assert_eq!(style.font_size, Some(17 * 64 + 16));
        assert_eq!(style.font_style(), font_style);
        assert_eq!(style.font_size(), Some(17.25));
        assert_eq!(style.language(), Some("ja-Latn-JP"));
        assert_eq!(style.features(), [feature]);
        assert_eq!(style.variations(), [variation]);
        assert_eq!(style.role(), TextRole::Formula);

        let defaults = SpanStyle::new();
        assert!(defaults.families().is_empty());
        assert_eq!(defaults.font_size(), None);
        assert_eq!(defaults.language(), None);
        assert!(defaults.features().is_empty());
        assert!(defaults.variations().is_empty());
        assert_eq!(defaults.role(), TextRole::Text);

        let replaced = style
            .clone()
            .with_families(["third"])
            .with_features([feature, feature])
            .with_variations([variation, variation]);
        assert_eq!(replaced.families(), ["third"]);
        assert_eq!(replaced.features(), [feature, feature]);
        assert_eq!(replaced.variations(), [variation, variation]);
        let cleared = replaced
            .with_families(Vec::<String>::new())
            .with_features([])
            .with_variations([]);
        assert!(cleared.families().is_empty());
        assert!(cleared.features().is_empty());
        assert!(cleared.variations().is_empty());

        assert!(SpanStyle::new().with_language("a".repeat(63)).is_ok());
        assert!(SpanStyle::new().with_language("a".repeat(64)).is_err());
        assert!(SpanStyle::new().with_language("a".repeat(65)).is_err());
        assert!(SpanStyle::new().with_language("").is_err());
        assert!(SpanStyle::new().with_language("ja_JP").is_err());
    }

    #[test]
    fn documents_read_back_spans_constructs_and_breaks() {
        let text = "漢字12 注記 割注 振分 字取 * H2O x+y";
        let mut builder = DocumentBuilder::new(text);
        builder
            .span(0..6, SpanStyle::new().with_family("Main"))
            .unwrap();
        builder.group_ruby(0..6, "かんじ").unwrap();
        builder.tate_chu_yoko(6..8).unwrap();
        builder.emphasis_dots(9..15, '・').unwrap();
        builder.warichu(16..22).unwrap();
        builder.furawake(23..29, 2, 1.5).unwrap();
        builder.jidori(30..36, 4).unwrap();
        builder.reference_mark(37..38, "※").unwrap();
        builder
            .script(39..42, "2", ScriptPosition::Subscript)
            .unwrap();
        builder.formula(43..46).unwrap();
        builder.mandatory_break(26).unwrap();
        builder.prohibit_break(3).unwrap();
        let document = builder.build().unwrap();

        assert_eq!(document.text(), text);
        assert_eq!(document.mandatory_breaks(), [26]);
        assert_eq!(document.prohibited_breaks(), [3]);

        let spans: Vec<_> = document.spans().collect();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].0, 0..6);
        assert_eq!(spans[0].1.families(), ["Main"]);

        assert_eq!(document.construct_count(), 9);
        let constructs: Vec<_> = document.constructs().collect();
        assert_eq!(constructs.len(), 9);
        assert_eq!(
            constructs[0],
            InlineConstruct::Ruby {
                kind: RubyKind::Group,
                base: 0..6,
                annotation: "かんじ",
                runs: &[],
            }
        );
        assert_eq!(constructs[1], InlineConstruct::TateChuYoko { range: 6..8 });
        assert_eq!(
            constructs[2],
            InlineConstruct::EmphasisDots {
                range: 9..15,
                mark: '・',
            }
        );
        assert_eq!(constructs[3], InlineConstruct::Warichu { range: 16..22 });
        assert_eq!(
            constructs[4],
            InlineConstruct::Furawake {
                range: 23..29,
                columns: 2,
                line_gap: 1.5,
            }
        );
        assert_eq!(
            constructs[5],
            InlineConstruct::Jidori {
                range: 30..36,
                cells: 4,
            }
        );
        assert_eq!(
            constructs[6],
            InlineConstruct::ReferenceMark {
                range: 37..38,
                mark: "※",
            }
        );
        assert_eq!(
            constructs[7],
            InlineConstruct::Script {
                range: 39..42,
                annotation: "2",
                position: ScriptPosition::Subscript,
            }
        );
        assert_eq!(constructs[8], InlineConstruct::Formula { range: 43..46 });

        for (ordinal, construct) in document.constructs().enumerate() {
            assert_eq!(document.construct(ordinal), Some(construct.clone()));
            let range = construct.range();
            assert!(range.start < range.end);
            assert!(range.end <= text.len());
        }
        assert_eq!(document.construct(9), None);
        assert_eq!(constructs[0].range(), 0..6);
        assert_eq!(constructs[8].range(), 43..46);
    }

    #[test]
    fn ruby_run_validation_reports_each_malformed_component() {
        let complete = vec![RubyRun::new(0..2, 0..2), RubyRun::new(2..4, 2..4)];
        assert!(validate_ruby_runs("abcd", RubyKind::Mono, &(0..4), "wxyz", &complete).is_ok());
        assert_eq!(
            ruby_error("abcd", RubyKind::Group, 0..4, "wxyz", &complete),
            "document.group-ruby-run-count"
        );

        let invalid_cases = [
            ("abc", 0..2, "x", RubyRun::new(1..2, 0..1)),
            ("abc", 0..2, "x", RubyRun::new(0..3, 0..1)),
            ("abc", 0..2, "xy", RubyRun::new(0..0, 0..1)),
            ("éA", 1..3, "x", RubyRun::new(1..3, 0..1)),
            ("éA", 0..1, "x", RubyRun::new(0..1, 0..1)),
            ("ab", 0..2, "xy", RubyRun::new(0..2, 1..2)),
            ("ab", 0..2, "xy", RubyRun::new(0..2, 0..3)),
            ("ab", 0..2, "xy", RubyRun::new(0..2, 0..0)),
            ("ab", 0..2, "éA", RubyRun::new(0..2, 1..3)),
            ("ab", 0..2, "éA", RubyRun::new(0..2, 0..1)),
        ];
        for (text, base, annotation, run) in invalid_cases {
            assert_eq!(
                ruby_error(text, RubyKind::Mono, base, annotation, &[run]),
                "document.invalid-ruby-run"
            );
        }

        assert_eq!(
            ruby_error(
                "abcd",
                RubyKind::Mono,
                0..4,
                "x",
                &[RubyRun::new(0..2, 0..1)],
            ),
            "document.incomplete-ruby-runs"
        );
        assert_eq!(
            ruby_error(
                "ab",
                RubyKind::Mono,
                0..2,
                "xy",
                &[RubyRun::new(0..2, 0..1)],
            ),
            "document.incomplete-ruby-runs"
        );
    }

    #[test]
    fn overlap_uses_half_open_range_edges() {
        assert!(ranges_overlap(&(0..2), &(1..3)));
        assert!(!ranges_overlap(&(0..1), &(1..2)));
        assert!(!ranges_overlap(&(1..2), &(0..1)));
    }

    #[test]
    fn span_insertion_uses_the_lower_bound_for_every_ordering_case() {
        let spans = [(1..2, SpanStyle::default()), (3..4, SpanStyle::default())];
        assert_eq!(span_insertion_point(&spans, 0), 0);
        assert_eq!(span_insertion_point(&spans, 2), 1);
        assert_eq!(span_insertion_point(&spans, 3), 1);
        assert_eq!(span_insertion_point(&spans, 5), 2);
    }
}
