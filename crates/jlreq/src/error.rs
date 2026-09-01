// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::fmt;
use std::ops::Range;

/// A validated option whose value was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OptionKind {
    /// Available inline length.
    LineExtent,
    /// Main text size.
    FontSize,
    /// Extra distance between lines.
    LineGap,
    /// OpenType variation coordinate.
    Variation,
    /// OpenType feature tag.
    Feature,
    /// Language tag.
    Language,
    /// Tab interval.
    TabWidth,
    /// A span-specific size.
    SpanFontSize,
    /// A construct-specific geometric value.
    ConstructGeometry,
    /// A four-byte OpenType tag.
    Tag,
    /// A physical point coordinate.
    Point,
}

/// A finite resource bounded by [`crate::ResourceLimits`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Resource {
    /// UTF-8 input bytes.
    InputBytes,
    /// Font resources made available to one layout.
    Fonts,
    /// Bytes across all registered fonts.
    FontBytes,
    /// Paragraphs in the input.
    Paragraphs,
    /// Shaping runs.
    Runs,
    /// Produced glyphs.
    Glyphs,
    /// Typed inline constructs.
    Constructs,
    /// Work charged by the deterministic core composer.
    CoreOperations,
}

/// Why high-level layout could not produce a complete result.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LayoutError {
    /// Font bytes or a TTC face index were invalid.
    InvalidFont {
        /// Index requested from a single font or collection.
        face_index: u32,
    },
    /// No font was registered.
    NoFonts,
    /// An option did not satisfy its public invariant.
    InvalidOption {
        /// The rejected option.
        option: OptionKind,
        /// Stable explanatory text.
        message: &'static str,
    },
    /// A UTF-8 range or typed document relationship was invalid.
    InvalidDocument {
        /// Stable machine-readable code.
        code: &'static str,
        /// Responsible source range, when applicable.
        range: Option<Range<usize>>,
        /// Stable explanatory text.
        message: &'static str,
    },
    /// A font registration, identifier, or system-family request could not be satisfied.
    InvalidFontRequest {
        /// Stable machine-readable code.
        code: &'static str,
        /// Stable explanatory text.
        message: &'static str,
    },
    /// A declared resource maximum was exceeded.
    ResourceLimit {
        /// Resource that stopped processing.
        resource: Resource,
        /// Configured inclusive limit.
        limit: usize,
        /// Required or observed amount.
        observed: usize,
    },
    /// Validated high-level data could not be represented by the core input model.
    CoreInput(jlreq_core::InputError),
    /// The deterministic core composer exhausted a declared budget.
    CoreComposition(jlreq_core::ComposeError),
}

impl LayoutError {
    pub(crate) const fn invalid_font(face_index: u32) -> Self {
        Self::InvalidFont { face_index }
    }

    pub(crate) const fn invalid_option(option: OptionKind, message: &'static str) -> Self {
        Self::InvalidOption { option, message }
    }

    pub(crate) fn invalid_document(
        code: &'static str,
        range: Option<Range<usize>>,
        message: &'static str,
    ) -> Self {
        Self::InvalidDocument {
            code,
            range,
            message,
        }
    }

    pub(crate) const fn invalid_font_request(code: &'static str, message: &'static str) -> Self {
        Self::InvalidFontRequest { code, message }
    }

    pub(crate) const fn resource(resource: Resource, limit: usize, observed: usize) -> Self {
        Self::ResourceLimit {
            resource,
            limit,
            observed,
        }
    }

    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidFont { .. } => "font.invalid",
            Self::NoFonts => "font.none-registered",
            Self::InvalidOption { .. } => "layout.invalid-option",
            Self::InvalidDocument { code, .. } | Self::InvalidFontRequest { code, .. } => code,
            Self::ResourceLimit { resource, .. } => match resource {
                Resource::InputBytes => "limit.input-bytes",
                Resource::Fonts => "limit.fonts",
                Resource::FontBytes => "limit.font-bytes",
                Resource::Paragraphs => "limit.paragraphs",
                Resource::Runs => "limit.runs",
                Resource::Glyphs => "limit.glyphs",
                Resource::Constructs => "limit.constructs",
                Resource::CoreOperations => "limit.core-operations",
            },
            Self::CoreInput(error) => error.code(),
            Self::CoreComposition(error) => error.code(),
        }
    }

    /// Responsible UTF-8 byte range, when one input range caused the error.
    #[must_use]
    pub fn range(&self) -> Option<Range<usize>> {
        match self {
            Self::InvalidDocument { range, .. } => range.clone(),
            Self::CoreInput(error) => error.range(),
            _ => None,
        }
    }

    /// Stable explanatory text, when the variant carries one.
    ///
    /// The text is intended for people; programs should match on
    /// [`code`](Self::code) instead.
    #[must_use]
    pub const fn message(&self) -> Option<&'static str> {
        match self {
            Self::InvalidOption { message, .. }
            | Self::InvalidDocument { message, .. }
            | Self::InvalidFontRequest { message, .. } => Some(message),
            Self::CoreInput(error) => Some(error.message()),
            _ => None,
        }
    }
}

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFont { face_index } => {
                write!(
                    formatter,
                    "invalid font data or TTC face index {face_index}"
                )
            },
            Self::NoFonts => formatter.write_str("no fonts are registered"),
            Self::InvalidOption { option, message } => {
                write!(formatter, "invalid {option:?}: {message}")
            },
            Self::InvalidDocument {
                code,
                range,
                message,
            } => {
                if let Some(range) = range {
                    write!(
                        formatter,
                        "{message} at bytes {}..{} ({code})",
                        range.start, range.end
                    )
                } else {
                    write!(formatter, "{message} ({code})")
                }
            },
            Self::InvalidFontRequest { code, message } => {
                write!(formatter, "{message} ({code})")
            },
            Self::ResourceLimit {
                resource,
                limit,
                observed,
            } => write!(
                formatter,
                "{resource:?} limit exceeded: limit {limit}, observed {observed}"
            ),
            Self::CoreInput(error) => error.fmt(formatter),
            Self::CoreComposition(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for LayoutError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CoreInput(error) => Some(error),
            Self::CoreComposition(error) => Some(error),
            _ => None,
        }
    }
}

impl From<jlreq_core::InputError> for LayoutError {
    fn from(value: jlreq_core::InputError) -> Self {
        Self::CoreInput(value)
    }
}

impl From<jlreq_core::ComposeError> for LayoutError {
    fn from(value: jlreq_core::ComposeError) -> Self {
        Self::CoreComposition(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    fn core_input_error() -> jlreq_core::InputError {
        jlreq_core::ShapedText::new(
            "é",
            jlreq_core::Size::square(64).unwrap(),
            jlreq_core::Frame::FullEm,
            [jlreq_core::Cluster::new(1..2, 64)],
        )
        .expect_err("the cluster starts inside a UTF-8 scalar")
    }

    fn core_compose_error() -> jlreq_core::ComposeError {
        let shaped = jlreq_core::ShapedText::new(
            "A",
            jlreq_core::Size::square(64).unwrap(),
            jlreq_core::Frame::FullEm,
            [jlreq_core::Cluster::new(0..1, 64)],
        )
        .unwrap();
        let paragraph = jlreq_core::Paragraph::builder(shaped, 64).build().unwrap();
        jlreq_core::Composer::with_limits(
            jlreq_core::CompositionLimits::default().with_max_clusters(0),
        )
        .compose(&paragraph, &jlreq_core::Style::default())
        .expect_err("one cluster exceeds a zero cluster budget")
    }

    #[test]
    fn ranges_and_error_sources_are_preserved_for_every_wrapped_error() {
        let document = LayoutError::invalid_document(
            "document.invalid-span-range",
            Some(2..5),
            "a span range must be valid",
        );
        assert_eq!(document.range(), Some(2..5));
        assert!(document.source().is_none());

        let input = LayoutError::from(core_input_error());
        assert_eq!(input.range(), Some(1..2));
        assert!(input.source().is_some());

        let composition = LayoutError::from(core_compose_error());
        assert_eq!(composition.range(), None);
        assert!(composition.source().is_some());

        assert_eq!(LayoutError::NoFonts.range(), None);
        assert!(LayoutError::NoFonts.source().is_none());
    }

    #[test]
    fn messages_and_display_carry_the_explanation_code_and_range() {
        let ranged = LayoutError::invalid_document(
            "document.invalid-span-range",
            Some(2..5),
            "a span range must be valid",
        );
        assert_eq!(ranged.message(), Some("a span range must be valid"));
        assert_eq!(
            ranged.to_string(),
            "a span range must be valid at bytes 2..5 (document.invalid-span-range)"
        );

        let unranged = LayoutError::invalid_document(
            "document.conflicting-break",
            None,
            "an offset cannot conflict",
        );
        assert_eq!(
            unranged.to_string(),
            "an offset cannot conflict (document.conflicting-break)"
        );

        let font = LayoutError::invalid_font_request("font.unknown-id", "the id is foreign");
        assert_eq!(font.code(), "font.unknown-id");
        assert_eq!(font.range(), None);
        assert_eq!(font.message(), Some("the id is foreign"));
        assert!(font.source().is_none());
        assert_eq!(font.to_string(), "the id is foreign (font.unknown-id)");

        let option = LayoutError::invalid_option(OptionKind::Point, "value must be finite");
        assert_eq!(option.message(), Some("value must be finite"));
        assert_eq!(option.to_string(), "invalid Point: value must be finite");
        assert_eq!(
            LayoutError::invalid_option(OptionKind::Tag, "four bytes").to_string(),
            "invalid Tag: four bytes"
        );

        assert_eq!(
            LayoutError::from(core_input_error()).message(),
            Some(core_input_error().message())
        );
        assert_eq!(LayoutError::NoFonts.message(), None);
        assert_eq!(LayoutError::from(core_compose_error()).message(), None);
        assert_eq!(LayoutError::invalid_font(3).message(), None);
    }
}
