// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

/// Dependency-free `no_std + alloc` composition API for already-shaped text.
pub use jlreq_core as core;
pub use jlreq_core::{Style, StyleBuilder, StyleError};

mod document;
mod engine;
mod error;
mod font;
mod options;
mod result;
mod units;

pub use document::{
    Document, DocumentBuilder, InlineConstruct, RubyKind, RubyRun, ScriptPosition, SpanStyle,
    TextRole,
};
pub use engine::LayoutEngine;
pub use error::{LayoutError, OptionKind, Resource};
pub use font::{FontId, FontLibrary, FontResource, FontSlant, FontStyle, FontSynthesis};
pub use options::{
    Alignment, BaseDirection, FontVariation, LayoutOptions, OpenTypeFeature, OpenTypeTag,
    ResourceLimits, WritingMode,
};
pub use result::{
    Affinity, AnnotationSource, Diagnostic, DiagnosticSeverity, GlyphPlacement, GlyphTransform,
    HitTest, Point, Rect, TextLayout, TextLine,
};

/// Shape and lay out plain UTF-8 text with a fresh engine.
///
/// Use [`LayoutEngine`] when processing repeatedly so font and shaper caches are reused.
pub fn layout(
    text: &str,
    fonts: &FontLibrary,
    options: LayoutOptions,
) -> Result<TextLayout, LayoutError> {
    LayoutEngine::new().layout(text, fonts, options)
}

/// Shape and lay out a typed document with a fresh engine.
pub fn layout_document(
    document: &Document,
    fonts: &FontLibrary,
    options: LayoutOptions,
) -> Result<TextLayout, LayoutError> {
    LayoutEngine::new().layout_document(document, fonts, options)
}
