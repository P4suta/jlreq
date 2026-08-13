// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fully typed choices at every place where JLReq 2020 permits alternatives.

/// The four convention levels in JLReq Appendix C.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KinsokuLevel {
    /// Newspaper practice.
    VeryLoose,
    /// Magazine practice.
    Loose,
    /// General-publication default.
    Strict,
    /// The strictest general-publication practice.
    VeryStrict,
}

/// Which of Tables 3, 4, and 5 supplies reduction amounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReductionTable {
    /// JLReq's adopted method.
    Table3,
    /// The alternative JLReq records from JIS X 4051.
    Table4,
    /// A further method seen in books.
    Table5,
}

/// Spacing from closing punctuation to the line end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LineEndPunctuation {
    /// Retain a half em.
    HalfEm,
    /// Set solid.
    Solid,
}

/// Spacing from a full stop or comma to the line end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LineEndFullStopComma {
    /// JLReq's preferred half-em reading.
    Preferred,
    /// The alternative JLReq records from JIS X 4051.
    Jis,
}

/// Paragraph-indent treatment when a line starts with an opening bracket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LineHeadOpeningBracket {
    /// One-em first-line indent and tentsuki continuation.
    Pattern1,
    /// One-and-a-half-em first line and half-em continuation.
    Pattern2,
    /// Half-em first line and tentsuki continuation.
    Pattern3,
}

/// Which neighboring Japanese characters ruby may overhang.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RubyOverhangKana {
    /// Hiragana and katakana according to JLReq's preferred reading.
    Kana,
    /// The narrower JIS reading recorded by JLReq.
    Jis,
    /// Any kana or ideograph.
    Any,
    /// No kana or ideograph.
    None,
}

/// Whether ruby may overhang the paragraph indent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RubyOverhangIndent {
    /// Overhang is permitted.
    Permitted,
    /// Overhang is prohibited.
    Prohibited,
}

/// Single-base ruby alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RubyAlignment {
    /// Center the reading on the base.
    Nakatsuki,
    /// Align the reading at the inline start.
    Katatsuki,
}

/// Distribution of a group-ruby reading no wider than its base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GroupRubyDistribution {
    /// Use the balanced JIS distribution recorded by JLReq.
    Jis,
    /// Keep both ends flush and divide interior space.
    Flush,
}

/// Layout method for jukugo ruby.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum JukugoRubyLayout {
    /// Lay the compound out as a group when necessary.
    Group,
    /// Follow phonetic structure and Appendix F.
    Phonetic,
}

/// Treatment of an ideographic iteration mark at line head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IterationMarkAtLineHead {
    /// Keep it away from line head through adjustment.
    Prohibited,
    /// Permit it at line head.
    Permitted,
    /// Replace it with the corresponding ideograph.
    Replaced,
}

/// Whether line-end punctuation may hang.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HangingPunctuation {
    /// Do not hang punctuation.
    None,
    /// Permit hanging punctuation.
    Hanging,
}

/// Breaking between grouped numerals and following Western text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GroupedNumeralBeforeWestern {
    /// The boundary is breakable.
    Breakable,
    /// The boundary is unbreakable.
    Unbreakable,
}

/// Spacing around a sentence-medial dividing mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SentenceMedialDividingMark {
    /// Set solid.
    Solid,
    /// Add a quarter em on both sides.
    QuarterEm,
}

/// Expansion ceiling between Japanese and Latin text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum JapaneseLatinExpansionCeiling {
    /// Expand up to half an em.
    HalfEm,
    /// Expand up to one third of an em.
    ThirdEm,
    /// Do not expand this space.
    Rigid,
}

/// Ordering of expansion operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExpansionOrder {
    /// Use the order JLReq records from JIS X 4051.
    Jis,
    /// Leave the order to the implementation.
    Implementation,
}

/// Whole-paragraph objective when several layouts are legal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AdjustmentPreference {
    /// Prefer the smallest total adjustment.
    LeastAdjustment,
    /// Prefer even texture between lines.
    EvenTexture,
}

/// Where indivisible integer remainder units are assigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Remainder {
    /// Assign from inline start.
    Leading,
    /// Assign from inline end.
    Trailing,
}

/// Classification of a code point absent from Appendix A.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UnlistedCodePoint {
    /// Resolve from the caller's metrics frame.
    ByFrame,
    /// Treat it as an ideograph.
    Ideographic,
}

/// Resolution when context leaves more than one class possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AmbiguousContext {
    /// Choose the lowest numbered class.
    LowestClass,
    /// Choose the highest numbered class.
    HighestClass,
}

/// Qualification of European numerals as grouped numerals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GroupedNumeralQualification {
    /// Resolve from shaped width.
    ByWidth,
    /// Resolve from the explicit document role.
    ByRole,
}

/// Mechanism used when relaxing the strictest kinsoku level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RelaxationMechanism {
    /// Reclassify the affected character as ideographic.
    Reclassify,
    /// Relax the prohibition matrix directly.
    Matrix,
}

/// A complete, internally consistent selection of JLReq 2020 alternatives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Style {
    pub(crate) kinsoku_level: KinsokuLevel,
    pub(crate) reduction_table: ReductionTable,
    pub(crate) line_end_punctuation: LineEndPunctuation,
    pub(crate) line_end_full_stop_comma: LineEndFullStopComma,
    pub(crate) line_head_opening_bracket: LineHeadOpeningBracket,
    pub(crate) ruby_overhang_kana: RubyOverhangKana,
    pub(crate) ruby_overhang_indent: RubyOverhangIndent,
    pub(crate) ruby_alignment: RubyAlignment,
    pub(crate) group_ruby_distribution: GroupRubyDistribution,
    pub(crate) jukugo_ruby_layout: JukugoRubyLayout,
    pub(crate) iteration_mark_at_line_head: IterationMarkAtLineHead,
    pub(crate) hanging_punctuation: HangingPunctuation,
    pub(crate) grouped_numeral_before_western: GroupedNumeralBeforeWestern,
    pub(crate) sentence_medial_dividing_mark: SentenceMedialDividingMark,
    pub(crate) japanese_latin_expansion_ceiling: JapaneseLatinExpansionCeiling,
    pub(crate) expansion_order: ExpansionOrder,
    pub(crate) adjustment_preference: AdjustmentPreference,
    pub(crate) remainder: Remainder,
    pub(crate) unlisted_code_point: UnlistedCodePoint,
    pub(crate) ambiguous_context: AmbiguousContext,
    pub(crate) grouped_numeral_qualification: GroupedNumeralQualification,
    pub(crate) relaxation_mechanism: RelaxationMechanism,
}

impl Style {
    const JLREQ_2020: Self = Self {
        kinsoku_level: KinsokuLevel::Strict,
        reduction_table: ReductionTable::Table3,
        line_end_punctuation: LineEndPunctuation::HalfEm,
        line_end_full_stop_comma: LineEndFullStopComma::Preferred,
        line_head_opening_bracket: LineHeadOpeningBracket::Pattern1,
        ruby_overhang_kana: RubyOverhangKana::Kana,
        ruby_overhang_indent: RubyOverhangIndent::Permitted,
        ruby_alignment: RubyAlignment::Nakatsuki,
        group_ruby_distribution: GroupRubyDistribution::Jis,
        jukugo_ruby_layout: JukugoRubyLayout::Group,
        iteration_mark_at_line_head: IterationMarkAtLineHead::Prohibited,
        hanging_punctuation: HangingPunctuation::None,
        grouped_numeral_before_western: GroupedNumeralBeforeWestern::Breakable,
        sentence_medial_dividing_mark: SentenceMedialDividingMark::Solid,
        japanese_latin_expansion_ceiling: JapaneseLatinExpansionCeiling::HalfEm,
        expansion_order: ExpansionOrder::Jis,
        adjustment_preference: AdjustmentPreference::LeastAdjustment,
        remainder: Remainder::Leading,
        unlisted_code_point: UnlistedCodePoint::ByFrame,
        ambiguous_context: AmbiguousContext::LowestClass,
        grouped_numeral_qualification: GroupedNumeralQualification::ByWidth,
        relaxation_mechanism: RelaxationMechanism::Reclassify,
    };

    /// JLReq's preferred or first-stated 2020 readings.
    #[must_use]
    pub const fn jlreq_2020() -> Self {
        Self::JLREQ_2020
    }

    /// The dated book profile: Table 5, pattern 3, and hanging punctuation.
    #[must_use]
    pub const fn book_2020() -> Self {
        Self {
            reduction_table: ReductionTable::Table5,
            line_head_opening_bracket: LineHeadOpeningBracket::Pattern3,
            hanging_punctuation: HangingPunctuation::Hanging,
            ..Self::JLREQ_2020
        }
    }

    /// The dated magazine profile, using loose kinsoku.
    #[must_use]
    pub const fn magazine_2020() -> Self {
        Self {
            kinsoku_level: KinsokuLevel::Loose,
            ..Self::JLREQ_2020
        }
    }

    /// The dated newspaper profile, using very loose kinsoku.
    #[must_use]
    pub const fn newspaper_2020() -> Self {
        Self {
            kinsoku_level: KinsokuLevel::VeryLoose,
            ..Self::JLREQ_2020
        }
    }

    /// JLReq's record of JIS alternatives, not a claim of full JIS X 4051 conformance.
    #[must_use]
    pub const fn jis_reading_2020() -> Self {
        Self {
            reduction_table: ReductionTable::Table4,
            line_end_punctuation: LineEndPunctuation::Solid,
            line_end_full_stop_comma: LineEndFullStopComma::Jis,
            ruby_overhang_kana: RubyOverhangKana::Jis,
            ..Self::JLREQ_2020
        }
    }

    /// Begin with the JLReq 2020 profile and override typed choices.
    #[must_use]
    pub const fn builder() -> StyleBuilder {
        StyleBuilder {
            style: Self::JLREQ_2020,
        }
    }

    /// Begin with this complete style and override typed choices.
    #[must_use]
    pub const fn to_builder(self) -> StyleBuilder {
        StyleBuilder { style: self }
    }

    /// The selected kinsoku level.
    #[must_use]
    pub const fn kinsoku_level(self) -> KinsokuLevel {
        self.kinsoku_level
    }

    /// The selected reduction table.
    #[must_use]
    pub const fn reduction_table(self) -> ReductionTable {
        self.reduction_table
    }

    /// The selected closing-punctuation line-end spacing.
    #[must_use]
    pub const fn line_end_punctuation(self) -> LineEndPunctuation {
        self.line_end_punctuation
    }

    /// The selected full-stop/comma line-end spacing.
    #[must_use]
    pub const fn line_end_full_stop_comma(self) -> LineEndFullStopComma {
        self.line_end_full_stop_comma
    }

    /// The selected opening-bracket line-head pattern.
    #[must_use]
    pub const fn line_head_opening_bracket(self) -> LineHeadOpeningBracket {
        self.line_head_opening_bracket
    }

    /// The selected ruby-overhang neighbor set.
    #[must_use]
    pub const fn ruby_overhang_kana(self) -> RubyOverhangKana {
        self.ruby_overhang_kana
    }

    /// Whether ruby can overhang the indent.
    #[must_use]
    pub const fn ruby_overhang_indent(self) -> RubyOverhangIndent {
        self.ruby_overhang_indent
    }

    /// The selected mono-ruby alignment.
    #[must_use]
    pub const fn ruby_alignment(self) -> RubyAlignment {
        self.ruby_alignment
    }

    /// The selected group-ruby distribution.
    #[must_use]
    pub const fn group_ruby_distribution(self) -> GroupRubyDistribution {
        self.group_ruby_distribution
    }

    /// The selected jukugo-ruby layout.
    #[must_use]
    pub const fn jukugo_ruby_layout(self) -> JukugoRubyLayout {
        self.jukugo_ruby_layout
    }

    /// The selected line-head iteration-mark treatment.
    #[must_use]
    pub const fn iteration_mark_at_line_head(self) -> IterationMarkAtLineHead {
        self.iteration_mark_at_line_head
    }

    /// The selected hanging-punctuation policy.
    #[must_use]
    pub const fn hanging_punctuation(self) -> HangingPunctuation {
        self.hanging_punctuation
    }

    /// The selected grouped-numeral/Western break rule.
    #[must_use]
    pub const fn grouped_numeral_before_western(self) -> GroupedNumeralBeforeWestern {
        self.grouped_numeral_before_western
    }

    /// The selected spacing around sentence-medial dividing marks.
    #[must_use]
    pub const fn sentence_medial_dividing_mark(self) -> SentenceMedialDividingMark {
        self.sentence_medial_dividing_mark
    }

    /// The selected Japanese/Latin expansion ceiling.
    #[must_use]
    pub const fn japanese_latin_expansion_ceiling(self) -> JapaneseLatinExpansionCeiling {
        self.japanese_latin_expansion_ceiling
    }

    /// The selected expansion order.
    #[must_use]
    pub const fn expansion_order(self) -> ExpansionOrder {
        self.expansion_order
    }

    /// The selected paragraph objective.
    #[must_use]
    pub const fn adjustment_preference(self) -> AdjustmentPreference {
        self.adjustment_preference
    }

    /// The selected integer-remainder direction.
    #[must_use]
    pub const fn remainder(self) -> Remainder {
        self.remainder
    }

    /// The selected unlisted-code-point classification.
    #[must_use]
    pub const fn unlisted_code_point(self) -> UnlistedCodePoint {
        self.unlisted_code_point
    }

    /// The selected ambiguous-context resolution.
    #[must_use]
    pub const fn ambiguous_context(self) -> AmbiguousContext {
        self.ambiguous_context
    }

    /// The selected grouped-numeral qualification.
    #[must_use]
    pub const fn grouped_numeral_qualification(self) -> GroupedNumeralQualification {
        self.grouped_numeral_qualification
    }

    /// The selected kinsoku relaxation mechanism.
    #[must_use]
    pub const fn relaxation_mechanism(self) -> RelaxationMechanism {
        self.relaxation_mechanism
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::jlreq_2020()
    }
}

/// A builder that validates cross-choice exclusions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct StyleBuilder {
    style: Style,
}

impl StyleBuilder {
    /// Set the typed kinsoku-level choice.
    #[must_use]
    pub const fn kinsoku_level(mut self, value: KinsokuLevel) -> Self {
        self.style.kinsoku_level = value;
        self
    }

    /// Set the typed reduction-table choice.
    #[must_use]
    pub const fn reduction_table(mut self, value: ReductionTable) -> Self {
        self.style.reduction_table = value;
        self
    }

    /// Set the typed line-end punctuation choice.
    #[must_use]
    pub const fn line_end_punctuation(mut self, value: LineEndPunctuation) -> Self {
        self.style.line_end_punctuation = value;
        self
    }

    /// Set the typed full-stop/comma line-end choice.
    #[must_use]
    pub const fn line_end_full_stop_comma(mut self, value: LineEndFullStopComma) -> Self {
        self.style.line_end_full_stop_comma = value;
        self
    }

    /// Set the typed opening-bracket line-head choice.
    #[must_use]
    pub const fn line_head_opening_bracket(mut self, value: LineHeadOpeningBracket) -> Self {
        self.style.line_head_opening_bracket = value;
        self
    }

    /// Set the typed ruby-overhang neighbor choice.
    #[must_use]
    pub const fn ruby_overhang_kana(mut self, value: RubyOverhangKana) -> Self {
        self.style.ruby_overhang_kana = value;
        self
    }

    /// Set the typed ruby-overhang indent choice.
    #[must_use]
    pub const fn ruby_overhang_indent(mut self, value: RubyOverhangIndent) -> Self {
        self.style.ruby_overhang_indent = value;
        self
    }

    /// Set the typed mono-ruby alignment choice.
    #[must_use]
    pub const fn ruby_alignment(mut self, value: RubyAlignment) -> Self {
        self.style.ruby_alignment = value;
        self
    }

    /// Set the typed group-ruby distribution choice.
    #[must_use]
    pub const fn group_ruby_distribution(mut self, value: GroupRubyDistribution) -> Self {
        self.style.group_ruby_distribution = value;
        self
    }

    /// Set the typed jukugo-ruby layout choice.
    #[must_use]
    pub const fn jukugo_ruby_layout(mut self, value: JukugoRubyLayout) -> Self {
        self.style.jukugo_ruby_layout = value;
        self
    }

    /// Set the typed line-head iteration-mark choice.
    #[must_use]
    pub const fn iteration_mark_at_line_head(mut self, value: IterationMarkAtLineHead) -> Self {
        self.style.iteration_mark_at_line_head = value;
        self
    }

    /// Set the typed hanging-punctuation choice.
    #[must_use]
    pub const fn hanging_punctuation(mut self, value: HangingPunctuation) -> Self {
        self.style.hanging_punctuation = value;
        self
    }

    /// Set the typed grouped-numeral/Western boundary choice.
    #[must_use]
    pub const fn grouped_numeral_before_western(
        mut self,
        value: GroupedNumeralBeforeWestern,
    ) -> Self {
        self.style.grouped_numeral_before_western = value;
        self
    }

    /// Set the typed sentence-medial dividing-mark choice.
    #[must_use]
    pub const fn sentence_medial_dividing_mark(
        mut self,
        value: SentenceMedialDividingMark,
    ) -> Self {
        self.style.sentence_medial_dividing_mark = value;
        self
    }

    /// Set the typed Japanese/Latin expansion-ceiling choice.
    #[must_use]
    pub const fn japanese_latin_expansion_ceiling(
        mut self,
        value: JapaneseLatinExpansionCeiling,
    ) -> Self {
        self.style.japanese_latin_expansion_ceiling = value;
        self
    }

    /// Set the typed expansion-order choice.
    #[must_use]
    pub const fn expansion_order(mut self, value: ExpansionOrder) -> Self {
        self.style.expansion_order = value;
        self
    }

    /// Set the typed paragraph-objective choice.
    #[must_use]
    pub const fn adjustment_preference(mut self, value: AdjustmentPreference) -> Self {
        self.style.adjustment_preference = value;
        self
    }

    /// Set the typed integer-remainder choice.
    #[must_use]
    pub const fn remainder(mut self, value: Remainder) -> Self {
        self.style.remainder = value;
        self
    }

    /// Set the typed unlisted-code-point choice.
    #[must_use]
    pub const fn unlisted_code_point(mut self, value: UnlistedCodePoint) -> Self {
        self.style.unlisted_code_point = value;
        self
    }

    /// Set the typed ambiguous-context choice.
    #[must_use]
    pub const fn ambiguous_context(mut self, value: AmbiguousContext) -> Self {
        self.style.ambiguous_context = value;
        self
    }

    /// Set the typed grouped-numeral qualification choice.
    #[must_use]
    pub const fn grouped_numeral_qualification(
        mut self,
        value: GroupedNumeralQualification,
    ) -> Self {
        self.style.grouped_numeral_qualification = value;
        self
    }

    /// Set the typed kinsoku-relaxation choice.
    #[must_use]
    pub const fn relaxation_mechanism(mut self, value: RelaxationMechanism) -> Self {
        self.style.relaxation_mechanism = value;
        self
    }
    /// Validate exclusions and return an immutable style.
    pub fn build(self) -> Result<Style, StyleError> {
        if self.style.kinsoku_level == KinsokuLevel::VeryStrict
            && self.style.grouped_numeral_before_western == GroupedNumeralBeforeWestern::Breakable
        {
            return Err(StyleError {
                code: "style.very-strict-grouped-numeral",
                message: "very-strict kinsoku excludes a breakable grouped-numeral boundary",
            });
        }
        if self.style.kinsoku_level == KinsokuLevel::VeryStrict
            && self.style.relaxation_mechanism == RelaxationMechanism::Reclassify
        {
            return Err(StyleError {
                code: "style.very-strict-relaxation",
                message: "very-strict kinsoku excludes the reclassification relaxation",
            });
        }
        Ok(self.style)
    }
}

impl Default for StyleBuilder {
    fn default() -> Self {
        Style::builder()
    }
}

/// A contradictory combination of otherwise valid typed choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct StyleError {
    code: &'static str,
    message: &'static str,
}

impl StyleError {
    /// A stable, language-independent conflict code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.code
    }

    /// A human-readable explanation of the conflict.
    #[must_use]
    pub const fn message(self) -> &'static str {
        self.message
    }
}

impl core::fmt::Display for StyleError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.message)
    }
}
