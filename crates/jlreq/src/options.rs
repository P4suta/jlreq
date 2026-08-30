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
                OptionKind::Feature,
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
#[derive(Debug, Clone, PartialEq)]
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
    pub(crate) limits: ResourceLimits,
}

impl LayoutOptions {
    /// Validate the two required floating-point inputs and quantize them to 26.6.
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
            limits: ResourceLimits::default(),
        })
    }

    /// Set horizontal or vertical writing.
    #[must_use]
    pub const fn writing_mode(mut self, value: WritingMode) -> Self {
        self.writing_mode = value;
        self
    }

    /// Set line alignment.
    #[must_use]
    pub const fn alignment(mut self, value: Alignment) -> Self {
        self.alignment = value;
        self
    }

    /// Set the complete low-level JLReq policy profile.
    #[must_use]
    pub fn style(mut self, value: jlreq_core::Style) -> Self {
        self.style = value;
        self
    }

    /// Set a BCP 47/OpenType language tag.
    pub fn language(mut self, value: impl Into<String>) -> Result<Self, LayoutError> {
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
    pub const fn base_direction(mut self, value: BaseDirection) -> Self {
        self.base_direction = value;
        self
    }

    /// Set extra block-axis distance between adjacent lines.
    pub fn line_gap(mut self, value: f32) -> Result<Self, LayoutError> {
        self.line_gap = non_negative(value, OptionKind::LineGap)?;
        Ok(self)
    }

    /// Set the tab interval in space cells.
    pub fn tab_width(mut self, value: u16) -> Result<Self, LayoutError> {
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
    pub fn feature(mut self, value: OpenTypeFeature) -> Self {
        self.features.push(value);
        self
    }

    /// Add one global variable-font axis value.
    #[must_use]
    pub fn variation(mut self, value: FontVariation) -> Self {
        self.variations.push(value);
        self
    }

    /// Replace all high-level resource limits.
    #[must_use]
    pub const fn limits(mut self, value: ResourceLimits) -> Self {
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
    pub const fn writing_mode_value(&self) -> WritingMode {
        self.writing_mode
    }

    /// Current limits.
    #[must_use]
    pub const fn resource_limits(&self) -> ResourceLimits {
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
                .language("a".repeat(63))
                .is_ok()
        );
        assert!(
            LayoutOptions::try_new(100.0, 16.0)
                .unwrap()
                .language("a".repeat(64))
                .is_err()
        );
        assert!(
            LayoutOptions::try_new(100.0, 16.0)
                .unwrap()
                .language("a".repeat(65))
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
