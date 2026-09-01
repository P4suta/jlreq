// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;
use core::ops::Range;

use crate::model::{InputError, ShapedText};

/// The three ruby relationships JLReq lays out differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RubyKind {
    /// One reading run is associated with each base cluster.
    Mono,
    /// One reading is associated with the base as a whole.
    Group,
    /// Runs form one kanji compound and may be placed individually or as a group.
    Jukugo,
}

/// One base-to-reading association inside a ruby construct.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RubyRun {
    base: Range<usize>,
    annotation: Range<usize>,
}

impl RubyRun {
    /// Associate a base byte range with an annotation byte range.
    #[must_use]
    pub const fn new(base: Range<usize>, annotation: Range<usize>) -> Self {
        Self { base, annotation }
    }

    /// The base range in the paragraph source.
    #[must_use]
    pub fn base(&self) -> Range<usize> {
        self.base.clone()
    }

    /// The range in the ruby annotation's source.
    #[must_use]
    pub fn annotation(&self) -> Range<usize> {
        self.annotation.clone()
    }
}

/// A validated ruby annotation, its base, and its run associations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Ruby {
    kind: RubyKind,
    base: Range<usize>,
    annotation: ShapedText,
    runs: Vec<RubyRun>,
}

impl Ruby {
    /// Validate a ruby construct without exposing internal lowering runs.
    pub fn new<I>(
        kind: RubyKind,
        base: Range<usize>,
        annotation: ShapedText,
        runs: I,
    ) -> Result<Self, InputError>
    where
        I: IntoIterator<Item = RubyRun>,
    {
        if base.start >= base.end {
            return Err(InputError::new(
                "input.empty-construct",
                Some(base),
                "ruby must cover a non-empty base range",
            ));
        }
        let runs: Vec<_> = runs.into_iter().collect();
        if runs.is_empty() {
            return Err(InputError::new(
                "input.ruby-without-runs",
                Some(base),
                "ruby must contain at least one base-to-annotation run",
            ));
        }
        if kind == RubyKind::Group && runs.len() != 1 {
            return Err(InputError::new(
                "input.group-ruby-run-count",
                Some(base),
                "group ruby has exactly one run",
            ));
        }

        let mut base_cursor = base.start;
        let mut annotation_cursor = 0;
        for run in &runs {
            if run.base.start != base_cursor
                || run.base.end > base.end
                || run.base.start >= run.base.end
            {
                return Err(InputError::new(
                    "input.invalid-ruby-base-run",
                    Some(run.base.clone()),
                    "ruby base runs must partition the declared base in source order",
                ));
            }
            if run.annotation.start != annotation_cursor
                || run.annotation.end > annotation.source().len()
                || run.annotation.start >= run.annotation.end
                || !annotation.cluster_boundary(run.annotation.start)
                || !annotation.cluster_boundary(run.annotation.end)
            {
                return Err(InputError::new(
                    "input.invalid-ruby-annotation-run",
                    Some(run.annotation.clone()),
                    "ruby annotation runs must partition the shaped annotation",
                ));
            }
            base_cursor = run.base.end;
            annotation_cursor = run.annotation.end;
        }
        if base_cursor != base.end || annotation_cursor != annotation.source().len() {
            return Err(InputError::new(
                "input.incomplete-ruby-runs",
                Some(base),
                "ruby runs must cover both base and annotation completely",
            ));
        }
        Ok(Self {
            kind,
            base,
            annotation,
            runs,
        })
    }

    /// The ruby relationship.
    #[must_use]
    pub const fn kind(&self) -> RubyKind {
        self.kind
    }

    /// The paragraph byte range carrying the base.
    #[must_use]
    pub fn base(&self) -> Range<usize> {
        self.base.clone()
    }

    /// The pre-shaped annotation stream.
    #[must_use]
    pub const fn annotation(&self) -> &ShapedText {
        &self.annotation
    }

    /// Caller-declared base-to-reading associations.
    #[must_use]
    pub fn runs(&self) -> &[RubyRun] {
        &self.runs
    }
}

/// Placement side for a script attachment relative to its base text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ScriptPosition {
    /// The annotation side: above the line in horizontal writing, right of
    /// the line in vertical writing.
    #[default]
    Superscript,
    /// The mirrored side: below the line in horizontal writing, left of the
    /// line in vertical writing.
    Subscript,
}

/// One of the nine public inline structures.
///
/// Its representation and lowering data are private. Named constructors carry only
/// document-level inputs, and range is the common attribution view.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Construct {
    kind: ConstructKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConstructKind {
    Ruby(Ruby),
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
        mark: ShapedText,
    },
    Script {
        range: Range<usize>,
        annotation: ShapedText,
        position: ScriptPosition,
    },
    Formula(Range<usize>),
}

impl Construct {
    /// Construct ruby.
    #[must_use]
    pub const fn ruby(ruby: Ruby) -> Self {
        Self {
            kind: ConstructKind::Ruby(ruby),
        }
    }

    /// Keep a byte range horizontal inside vertical writing.
    #[must_use]
    pub const fn tate_chu_yoko(range: Range<usize>) -> Self {
        Self {
            kind: ConstructKind::TateChuYoko(range),
        }
    }

    /// Attach an emphasis mark to every base cluster in a byte range.
    #[must_use]
    pub const fn emphasis_dots(range: Range<usize>, mark: char) -> Self {
        Self {
            kind: ConstructKind::Emphasis { range, mark },
        }
    }

    /// Lay a byte range out as an inline cutting note.
    #[must_use]
    pub const fn warichu(range: Range<usize>) -> Self {
        Self {
            kind: ConstructKind::Warichu(range),
        }
    }

    /// Distribute a byte range across the requested number of aligned sublines.
    ///
    /// Paragraph break opportunities strictly inside the range declare the boundaries
    /// between those sublines. The line gap is the non-negative block-axis gap between
    /// adjacent sublines.
    #[must_use]
    pub const fn furawake(range: Range<usize>, columns: u16, line_gap: i32) -> Self {
        Self {
            kind: ConstructKind::Furawake {
                range,
                columns,
                line_gap,
            },
        }
    }

    /// Fit a byte range into a fixed number of full-em cells (字取り).
    #[must_use]
    pub const fn jidori(range: Range<usize>, cells: u16) -> Self {
        Self {
            kind: ConstructKind::Jidori { range, cells },
        }
    }

    /// Attach a pre-shaped reference mark (合印) to a base range.
    #[must_use]
    pub const fn reference_mark(range: Range<usize>, mark: ShapedText) -> Self {
        Self {
            kind: ConstructKind::ReferenceMark { range, mark },
        }
    }

    /// Attach a pre-shaped script complex on the superscript side.
    ///
    /// Equivalent to [`script_at`](Self::script_at) with
    /// [`ScriptPosition::Superscript`].
    #[must_use]
    pub const fn script(range: Range<usize>, annotation: ShapedText) -> Self {
        Self::script_at(range, annotation, ScriptPosition::Superscript)
    }

    /// Attach a pre-shaped script complex on an explicit side.
    ///
    /// [`ScriptPosition::Superscript`] places the annotation on the same side
    /// as ruby; [`ScriptPosition::Subscript`] mirrors it to the opposite
    /// block side, and the line reserves space there.
    #[must_use]
    pub const fn script_at(
        range: Range<usize>,
        annotation: ShapedText,
        position: ScriptPosition,
    ) -> Self {
        Self {
            kind: ConstructKind::Script {
                range,
                annotation,
                position,
            },
        }
    }

    /// Treat a pre-shaped math range as one formula structure.
    ///
    /// Declared paragraph breaks are accepted only next to a math symbol or operator.
    #[must_use]
    pub const fn formula(range: Range<usize>) -> Self {
        Self {
            kind: ConstructKind::Formula(range),
        }
    }

    /// The base UTF-8 byte range in the paragraph source.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        match &self.kind {
            ConstructKind::Ruby(ruby) => ruby.base(),
            ConstructKind::TateChuYoko(range)
            | ConstructKind::Warichu(range)
            | ConstructKind::Formula(range)
            | ConstructKind::Emphasis { range, .. }
            | ConstructKind::Furawake { range, .. }
            | ConstructKind::Jidori { range, .. }
            | ConstructKind::ReferenceMark { range, .. }
            | ConstructKind::Script { range, .. } => range.clone(),
        }
    }

    pub(crate) const fn kind(&self) -> &ConstructKind {
        &self.kind
    }
}

pub(crate) fn is_math_symbol(character: char) -> bool {
    crate::spec::single_has_class(character, crate::spec::MATH_SYMBOL)
}

pub(crate) fn is_math_operator(character: char) -> bool {
    crate::spec::single_has_class(character, crate::spec::MATH_OPERATOR)
}

pub(crate) fn is_math_token(character: char) -> bool {
    is_math_symbol(character) || is_math_operator(character)
}
