// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::units::{finite, non_negative, positive, quantize, to_f32};
use crate::{LayoutError, OptionKind};

/// Physical writing mode requested from the high-level pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum WritingMode {
    /// Inline progression is left-to-right and lines progress top-to-bottom.
    #[default]
    HorizontalTb,
    /// Inline progression is top-to-bottom and lines progress right-to-left.
    VerticalRl,
}

impl WritingMode {
    pub(crate) const fn core(self) -> jlreq_core::WritingMode {
        match self {
            Self::HorizontalTb => jlreq_core::WritingMode::HorizontalTb,
            Self::VerticalRl => jlreq_core::WritingMode::VerticalRl,
        }
    }
}

/// Line alignment in the available inline extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Alignment {
    /// Align at inline start.
    Start,
    /// Center the occupied line.
    Center,
    /// Align at inline end.
    End,
    /// Apply JLReq adjustment to non-final lines.
    #[default]
    Justify,
}

impl Alignment {
    pub(crate) const fn core(self) -> jlreq_core::Alignment {
        match self {
            Self::Start => jlreq_core::Alignment::Start,
            Self::Center => jlreq_core::Alignment::Center,
            Self::End => jlreq_core::Alignment::End,
            Self::Justify => jlreq_core::Alignment::Justify,
        }
    }
}

/// Paragraph base-direction policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum BaseDirection {
    /// Resolve from the first strong character under UAX #9.
    #[default]
    Auto,
    /// Force a left-to-right paragraph embedding level.
    LeftToRight,
    /// Force a right-to-left paragraph embedding level.
    RightToLeft,
}

/// A four-byte OpenType tag whose bytes are kept private.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenTypeTag([u8; 4]);

impl OpenTypeTag {
    /// Validate an OpenType tag of exactly four printable ASCII bytes.
    ///
    /// Space padding is accepted only at the end, as required by the OpenType
    /// specification for tags shorter than four visible characters.
    pub fn try_new(tag: &str) -> Result<Self, LayoutError> {
        let bytes = tag.as_bytes();
        let printable = bytes.iter().all(|byte| matches!(byte, 0x20..=0x7e));
        let trailing_spaces_only =
            bytes
                .iter()
                .position(|byte| *byte == b' ')
                .is_none_or(|first_space| {
                    first_space > 0 && bytes[first_space..].iter().all(|byte| *byte == b' ')
                });
        if bytes.len() != 4 || !printable || !trailing_spaces_only {
            return Err(LayoutError::invalid_option(
                OptionKind::Tag,
                "an OpenType tag must contain four printable ASCII bytes with spaces only as trailing padding",
            ));
        }
        Ok(Self([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    #[cfg(feature = "system-fonts")]
    pub(crate) const fn from_bytes(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    /// Tag bytes in network order.
    #[must_use]
    pub const fn bytes(self) -> [u8; 4] {
        self.0
    }
}

/// One global OpenType feature setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenTypeFeature {
    tag: OpenTypeTag,
    value: u32,
}

impl OpenTypeFeature {
    /// Create a feature value applied to the whole shaping run.
    #[must_use]
    pub const fn new(tag: OpenTypeTag, value: u32) -> Self {
        Self { tag, value }
    }

    /// Feature tag.
    #[must_use]
    pub const fn tag(self) -> OpenTypeTag {
        self.tag
    }

    /// Feature value.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.value
    }
}

/// One variable-font axis value, quantized at the public boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontVariation {
    tag: OpenTypeTag,
    value: i32,
}

impl FontVariation {
    /// Validate and quantize a variation setting.
    pub fn try_new(tag: OpenTypeTag, value: f32) -> Result<Self, LayoutError> {
        let value = finite(value, OptionKind::Variation)?;
        Ok(Self {
            tag,
            value: quantize(value),
        })
    }

    /// Axis tag.
    #[must_use]
    pub const fn tag(self) -> OpenTypeTag {
        self.tag
    }

    /// Quantized user-space axis value.
    #[must_use]
    pub fn value(self) -> f32 {
        to_f32(self.value)
    }

    /// Quantized axis value in signed 26.6 fixed point.
    #[must_use]
    pub const fn value_26_6(self) -> i32 {
        self.value
    }
}

/// Direction-independent alignment at an explicit tab stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum TabAlignment {
    /// Content starts at the stop.
    #[default]
    Start,
    /// Content is centered on the stop.
    Center,
    /// Content ends at the stop.
    End,
    /// Occurrences of the character are aligned on the stop, decimal-point
    /// style (JLReq 3.6.2).
    Character(char),
}

impl TabAlignment {
    pub(crate) const fn core(self) -> jlreq_core::TabAlignment {
        match self {
            Self::Start => jlreq_core::TabAlignment::Start,
            Self::Center => jlreq_core::TabAlignment::Center,
            Self::End => jlreq_core::TabAlignment::End,
            Self::Character(character) => jlreq_core::TabAlignment::Character(character),
        }
    }
}

/// One explicit tab stop, positioned in the same units as the line extent.
///
/// Explicit stops replace the evenly spaced ladder derived from
/// [`LayoutOptions::with_tab_width`] and unlock the center, end, and
/// character (decimal-point) alignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabStop {
    position: i32,
    alignment: TabAlignment,
}

impl TabStop {
    /// Validate and quantize a stop at a positive inline position.
    pub fn try_new(position: f32, alignment: TabAlignment) -> Result<Self, LayoutError> {
        Ok(Self {
            position: positive(position, OptionKind::TabStop)?,
            alignment,
        })
    }

    /// Inline position after 26.6 quantization.
    #[must_use]
    pub fn position(self) -> f32 {
        to_f32(self.position)
    }

    /// Alignment applied at the stop.
    #[must_use]
    pub const fn alignment(self) -> TabAlignment {
        self.alignment
    }

    pub(crate) fn core(self) -> Result<jlreq_core::TabStop, LayoutError> {
        Ok(jlreq_core::TabStop::new(
            self.position,
            self.alignment.core(),
        )?)
    }
}

/// Final-line widow policy (JLReq 3.5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Widow {
    /// Do not impose a final-line cluster minimum.
    #[default]
    Allow,
    /// Prefer at least this many base clusters on the final line.
    MinimumClusters(u16),
}

impl Widow {
    pub(crate) const fn core(self) -> jlreq_core::Widow {
        match self {
            Self::Allow => jlreq_core::Widow::Allow,
            Self::MinimumClusters(minimum) => jlreq_core::Widow::MinimumClusters(minimum),
        }
    }
}

/// Deterministic high-level resource limits for one call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ResourceLimits {
    pub(crate) input_bytes: usize,
    pub(crate) fonts: usize,
    pub(crate) font_bytes: usize,
    pub(crate) paragraphs: usize,
    pub(crate) runs: usize,
    pub(crate) glyphs: usize,
    pub(crate) constructs: usize,
    pub(crate) core_operations: usize,
}

impl ResourceLimits {
    /// Release defaults, usable in constants.
    pub const DEFAULT: Self = Self {
        input_bytes: 16 * 1024 * 1024,
        fonts: 256,
        font_bytes: 512 * 1024 * 1024,
        paragraphs: 65_536,
        runs: 262_144,
        glyphs: 1_000_000,
        constructs: 4_096,
        core_operations: 8_000_000,
    };

    /// Maximum input bytes.
    #[must_use]
    pub const fn max_input_bytes(self) -> usize {
        self.input_bytes
    }

    /// Maximum registered fonts considered by a call.
    #[must_use]
    pub const fn max_fonts(self) -> usize {
        self.fonts
    }

    /// Maximum total bytes in registered fonts.
    #[must_use]
    pub const fn max_font_bytes(self) -> usize {
        self.font_bytes
    }

    /// Maximum paragraphs.
    #[must_use]
    pub const fn max_paragraphs(self) -> usize {
        self.paragraphs
    }

    /// Maximum shaping runs.
    #[must_use]
    pub const fn max_runs(self) -> usize {
        self.runs
    }

    /// Maximum produced glyphs.
    #[must_use]
    pub const fn max_glyphs(self) -> usize {
        self.glyphs
    }

    /// Maximum inline constructs.
    #[must_use]
    pub const fn max_constructs(self) -> usize {
        self.constructs
    }

    /// Maximum core operations.
    #[must_use]
    pub const fn max_core_operations(self) -> usize {
        self.core_operations
    }

    /// Replace the input-byte maximum.
    #[must_use]
    pub const fn with_max_input_bytes(mut self, value: usize) -> Self {
        self.input_bytes = value;
        self
    }

    /// Replace the font-count maximum.
    #[must_use]
    pub const fn with_max_fonts(mut self, value: usize) -> Self {
        self.fonts = value;
        self
    }

    /// Replace the font-byte maximum.
    #[must_use]
    pub const fn with_max_font_bytes(mut self, value: usize) -> Self {
        self.font_bytes = value;
        self
    }

    /// Replace the paragraph maximum.
    #[must_use]
    pub const fn with_max_paragraphs(mut self, value: usize) -> Self {
        self.paragraphs = value;
        self
    }

    /// Replace the run maximum.
    #[must_use]
    pub const fn with_max_runs(mut self, value: usize) -> Self {
        self.runs = value;
        self
    }

    /// Replace the glyph maximum.
    #[must_use]
    pub const fn with_max_glyphs(mut self, value: usize) -> Self {
        self.glyphs = value;
        self
    }

    /// Replace the construct maximum.
    #[must_use]
    pub const fn with_max_constructs(mut self, value: usize) -> Self {
        self.constructs = value;
        self
    }

    /// Replace the core-operation maximum.
    #[must_use]
    pub const fn with_max_core_operations(mut self, value: usize) -> Self {
        self.core_operations = value;
        self
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Validated controls for automatic shaping and line layout.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LayoutOptions {
    pub(crate) line_extent: i32,
    pub(crate) font_size: i32,
    pub(crate) writing_mode: WritingMode,
    pub(crate) alignment: Alignment,
    pub(crate) style: jlreq_core::Style,
    pub(crate) language: String,
    pub(crate) base_direction: BaseDirection,
    pub(crate) line_gap: i32,
    pub(crate) tab_width: u16,
    pub(crate) features: Vec<OpenTypeFeature>,
    pub(crate) variations: Vec<FontVariation>,
    pub(crate) widow: Widow,
    pub(crate) first_line_indent: i32,
    pub(crate) tab_stops: Vec<TabStop>,
    pub(crate) limits: ResourceLimits,
}

impl LayoutOptions {
    /// Validate the two required floating-point inputs and quantize them to 26.6.
    ///
    /// The line extent and font size are constructor invariants; every other
    /// control has a `with_*` setter and a same-named getter.
    pub fn try_new(line_extent: f32, font_size: f32) -> Result<Self, LayoutError> {
        Ok(Self {
            line_extent: positive(line_extent, OptionKind::LineExtent)?,
            font_size: positive(font_size, OptionKind::FontSize)?,
            writing_mode: WritingMode::default(),
            alignment: Alignment::default(),
            style: jlreq_core::Style::default(),
            language: "und".to_owned(),
            base_direction: BaseDirection::default(),
            line_gap: 0,
            tab_width: 4,
            features: Vec::new(),
            variations: Vec::new(),
            widow: Widow::Allow,
            first_line_indent: 0,
            tab_stops: Vec::new(),
            limits: ResourceLimits::default(),
        })
    }

    /// Set horizontal or vertical writing.
    #[must_use]
    pub const fn with_writing_mode(mut self, value: WritingMode) -> Self {
        self.writing_mode = value;
        self
    }

    /// Set line alignment.
    #[must_use]
    pub const fn with_alignment(mut self, value: Alignment) -> Self {
        self.alignment = value;
        self
    }

    /// Set the complete low-level JLReq policy profile.
    #[must_use]
    pub fn with_style(mut self, value: jlreq_core::Style) -> Self {
        self.style = value;
        self
    }

    /// Set a BCP 47/OpenType language tag.
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
        self.language = value;
        Ok(self)
    }

    /// Set paragraph base direction.
    #[must_use]
    pub const fn with_base_direction(mut self, value: BaseDirection) -> Self {
        self.base_direction = value;
        self
    }

    /// Set extra block-axis distance between adjacent lines.
    pub fn with_line_gap(mut self, value: f32) -> Result<Self, LayoutError> {
        self.line_gap = non_negative(value, OptionKind::LineGap)?;
        Ok(self)
    }

    /// Set the tab interval in space cells.
    pub fn with_tab_width(mut self, value: u16) -> Result<Self, LayoutError> {
        if value == 0 {
            return Err(LayoutError::invalid_option(
                OptionKind::TabWidth,
                "tab width must be at least one space cell",
            ));
        }
        self.tab_width = value;
        Ok(self)
    }

    /// Add one global OpenType feature.
    #[must_use]
    pub fn with_feature(mut self, value: OpenTypeFeature) -> Self {
        self.features.push(value);
        self
    }

    /// Replace every global OpenType feature. An empty iterator clears them.
    #[must_use]
    pub fn with_features<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = OpenTypeFeature>,
    {
        self.features = values.into_iter().collect();
        self
    }

    /// Add one global variable-font axis value.
    #[must_use]
    pub fn with_variation(mut self, value: FontVariation) -> Self {
        self.variations.push(value);
        self
    }

    /// Replace every global variable-font axis value. An empty iterator clears them.
    #[must_use]
    pub fn with_variations<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = FontVariation>,
    {
        self.variations = values.into_iter().collect();
        self
    }

    /// Set the document-wide final-line widow policy.
    #[must_use]
    pub const fn with_widow(mut self, value: Widow) -> Self {
        self.widow = value;
        self
    }

    /// Set the document-wide non-negative first-line indent (字下げ).
    ///
    /// Layout additionally requires the indent to be smaller than the line
    /// extent.
    pub fn with_first_line_indent(mut self, value: f32) -> Result<Self, LayoutError> {
        self.first_line_indent = non_negative(value, OptionKind::FirstLineIndent)?;
        Ok(self)
    }

    /// Replace the document-wide explicit tab stops.
    ///
    /// Non-empty stops replace the evenly spaced ladder derived from
    /// [`with_tab_width`](Self::with_tab_width); an empty iterator restores
    /// the ladder. Stops only apply to paragraphs that contain a tab.
    #[must_use]
    pub fn with_tab_stops<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = TabStop>,
    {
        self.tab_stops = values.into_iter().collect();
        self
    }

    /// Replace all high-level resource limits.
    #[must_use]
    pub const fn with_limits(mut self, value: ResourceLimits) -> Self {
        self.limits = value;
        self
    }

    /// Available inline length after 26.6 quantization.
    #[must_use]
    pub fn line_extent(&self) -> f32 {
        crate::units::to_f32(self.line_extent)
    }

    /// Main text size after 26.6 quantization.
    #[must_use]
    pub fn font_size(&self) -> f32 {
        crate::units::to_f32(self.font_size)
    }

    /// Current writing mode.
    #[must_use]
    pub const fn writing_mode(&self) -> WritingMode {
        self.writing_mode
    }

    /// Current line alignment.
    #[must_use]
    pub const fn alignment(&self) -> Alignment {
        self.alignment
    }

    /// Current low-level JLReq policy profile.
    #[must_use]
    pub const fn style(&self) -> &jlreq_core::Style {
        &self.style
    }

    /// Current shaping language tag.
    #[must_use]
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Current paragraph base direction.
    #[must_use]
    pub const fn base_direction(&self) -> BaseDirection {
        self.base_direction
    }

    /// Current extra block-axis distance between adjacent lines.
    #[must_use]
    pub fn line_gap(&self) -> f32 {
        crate::units::to_f32(self.line_gap)
    }

    /// Current tab interval in space cells.
    #[must_use]
    pub const fn tab_width(&self) -> u16 {
        self.tab_width
    }

    /// Current global OpenType features, in application order.
    #[must_use]
    pub fn features(&self) -> &[OpenTypeFeature] {
        &self.features
    }

    /// Current global variable-font axis values, in application order.
    #[must_use]
    pub fn variations(&self) -> &[FontVariation] {
        &self.variations
    }

    /// Current document-wide final-line widow policy.
    #[must_use]
    pub const fn widow(&self) -> Widow {
        self.widow
    }

    /// Current document-wide first-line indent after 26.6 quantization.
    #[must_use]
    pub fn first_line_indent(&self) -> f32 {
        crate::units::to_f32(self.first_line_indent)
    }

    /// Current document-wide explicit tab stops, in application order.
    #[must_use]
    pub fn tab_stops(&self) -> &[TabStop] {
        &self.tab_stops
    }

    /// Current limits.
    #[must_use]
    pub const fn limits(&self) -> ResourceLimits {
        self.limits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_resource_defaults_are_exact_byte_counts() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_input_bytes(), 16_777_216);
        assert_eq!(limits.max_fonts(), 256);
        assert_eq!(limits.max_font_bytes(), 536_870_912);
        assert_eq!(limits.max_paragraphs(), 65_536);
        assert_eq!(limits.max_runs(), 262_144);
        assert_eq!(limits.max_glyphs(), 1_000_000);
        assert_eq!(limits.max_constructs(), 4_096);
        assert_eq!(limits.max_core_operations(), 8_000_000);
    }

    #[test]
    fn layout_language_accepts_exactly_the_documented_length_boundary() {
        assert!(
            LayoutOptions::try_new(100.0, 16.0)
                .unwrap()
                .with_language("a".repeat(63))
                .is_ok()
        );
        assert!(
            LayoutOptions::try_new(100.0, 16.0)
                .unwrap()
                .with_language("a".repeat(64))
                .is_err()
        );
        assert!(
            LayoutOptions::try_new(100.0, 16.0)
                .unwrap()
                .with_language("a".repeat(65))
                .is_err()
        );
    }

    #[test]
    fn opentype_tags_accept_only_printable_bytes_and_trailing_padding() {
        for valid in ["wght", "abc ", "a   ", "~~~~"] {
            assert_eq!(
                OpenTypeTag::try_new(valid).unwrap().bytes(),
                valid.as_bytes()
            );
        }
        for invalid in ["abc", "abcde", " abc", "a bc", "    ", "ab\n ", "éab"] {
            assert!(OpenTypeTag::try_new(invalid).is_err(), "{invalid:?}");
        }
    }

    #[test]
    fn font_variations_are_fixed_point_equal_and_hashable() {
        use std::hash::{Hash, Hasher};

        let tag = OpenTypeTag::try_new("wght").unwrap();
        let first = FontVariation::try_new(tag, 650.124).unwrap();
        let same_cell = FontVariation::try_new(tag, 650.125).unwrap();
        let different = FontVariation::try_new(tag, 650.14).unwrap();
        assert_eq!(first.value_26_6(), 41_608);
        assert_eq!(first.value().to_bits(), 650.125_f32.to_bits());
        assert_eq!(first, same_cell);
        assert_ne!(first, different);
        let hash = |value: FontVariation| {
            let mut state = std::collections::hash_map::DefaultHasher::new();
            value.hash(&mut state);
            state.finish()
        };
        assert_eq!(hash(first), hash(same_cell));
        assert_ne!(hash(first), hash(different));
    }
}
