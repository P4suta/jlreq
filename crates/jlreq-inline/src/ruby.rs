// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Ruby (ルビ): [`Ruby`], its declared runs, style and alignment, and the four ways
//! declaring one is refused.
//!
//! One type, not three, because §3.3.7 states that a jukugo compound whose every base
//! carries two or fewer ruby characters *is* composed as mono-ruby, and §3.3.1's note says
//! the two then produce identical geometry and differ only in line-adjustment behavior — a
//! relationship [`RubyRun`]'s shape expresses rather than three duplicated types.
//!
//! The reading is a second stream, [`Annotation`], not a range of the annotated text: a
//! base character and the ruby attached to it are not an adjacency any cell of Table 1 is
//! indexed by, and the caller's break candidates were computed over the document, which does
//! not contain the reading interleaved into it (ADR-0016).
//!
//! JLReq: §3.3.1–§3.3.8, §F

use core::ops::Range;

use jlreq_class::{Annotation, AnnotationIndex, Text};
use jlreq_unit::ItemIndex;

/// Ruby (ルビ): a smaller reading set beside a base.
///
/// JLReq: §3.3.1–§3.3.8, §F
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Ruby<'r> {
    text: Text<'r>,
    base_first: ItemIndex,
    base_past: ItemIndex,
    annotation: Annotation<'r>,
    runs: &'r [RubyRun],
    style: RubyStyle,
    alignment: Option<RubyAlignment>,
}

impl<'r> Ruby<'r> {
    /// Declare a ruby over `base`, a range of `text`'s own items, read by `annotation`
    /// through `runs`, composed as `style`.
    ///
    /// Takes **both** streams, which is what lets it validate both ranges: `base` is a
    /// `Range<ItemIndex>` and each run's annotation is a `Range<AnnotationIndex>`, so a
    /// swapped pair is a compile error rather than a review finding (ADR-0016).
    ///
    /// Validated: `base` lies inside `text`, every run's base lies inside `base`, every
    /// run's annotation lies inside `annotation`, the runs cover both in order without
    /// overlap, and the count matches what `style` requires.
    ///
    /// JLReq: §3.3.5, §3.3.6, §3.3.7
    pub fn new(
        text: Text<'r>,
        base: Range<ItemIndex>,
        annotation: Annotation<'r>,
        runs: &'r [RubyRun],
        style: RubyStyle,
    ) -> Result<Self, RubyError> {
        let base_first = base.start;
        let base_past = base.end;
        check_base_in_text(text, base_first..base_past)?;
        check_run_count(base_first..base_past, style, runs.len())?;
        check_runs(base_first..base_past, annotation, runs)?;
        Ok(Self {
            text,
            base_first,
            base_past,
            annotation,
            runs,
            style,
            alignment: None,
        })
    }

    /// Override [`jlreq_spec::Question::RUBY_ALIGNMENT`] for this ruby, which is the precedence
    /// rule of ADR-0019: the policy is the document's default and a per-construct statement
    /// wins for that construct.
    ///
    /// Not a `Result`, and not a `Direction` parameter. §3.3.5 says katatsuki (肩付き)
    /// "should not be adopted" for horizontal writing — a recommendation about a construct
    /// that is well defined there, unlike §3.2.5's tate-chu-yoko, which JLReq does not
    /// define horizontally at all. Refusing it at construction would publish a prohibition
    /// the specification does not state, so [`crate::lower`] is where the direction is
    /// read, and a caller who overrides it is honored.
    ///
    /// JLReq: §3.3.5
    #[must_use]
    pub const fn with_alignment(mut self, alignment: RubyAlignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    /// The stream this ruby annotates. Crate-visible: [`crate::lower`] reads it, and it is
    /// not part of the published surface (ADR-0012 elides `Ruby`'s fields entirely).
    pub(crate) const fn text(self) -> Text<'r> {
        self.text
    }

    /// The base range this ruby annotates, in `text`'s own items.
    pub(crate) const fn base(self) -> Range<ItemIndex> {
        self.base_first..self.base_past
    }

    /// The reading.
    pub(crate) const fn annotation(self) -> Annotation<'r> {
        self.annotation
    }

    /// The declared runs, in order.
    pub(crate) const fn runs(self) -> &'r [RubyRun] {
        self.runs
    }

    /// Which of the three attachment styles this ruby is.
    pub(crate) const fn style(self) -> RubyStyle {
        self.style
    }

    /// The per-construct alignment override, if the caller declared one.
    pub(crate) const fn alignment(self) -> Option<RubyAlignment> {
        self.alignment
    }
}

/// Why a declared [`Ruby`] is refused.
///
/// JLReq: §3.3.5, §3.3.6, §3.3.7
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RubyError {
    /// A base range lies outside the annotated stream.
    BaseOutOfRange {
        /// The offending ordinal.
        at: ItemIndex,
    },
    /// An annotation range lies outside the reading.
    AnnotationOutOfRange {
        /// The offending ordinal.
        at: AnnotationIndex,
    },
    /// The runs do not cover their ranges in order without overlap.
    RunsNotContiguous {
        /// The position in `runs` where contiguity broke, or `runs.len()` when the runs
        /// stop short of covering `base` or `annotation` in full.
        at: usize,
    },
    /// [`RubyStyle::MonoRuby`] and [`RubyStyle::JukugoRuby`] need one run per base item;
    /// [`RubyStyle::GroupRuby`] needs exactly one.
    ///
    /// JLReq: §3.3.5, §3.3.6, §3.3.7
    RunCount {
        /// How many runs `style` requires.
        expected: usize,
        /// How many the caller supplied.
        found: usize,
    },
}

/// Which of JLReq's three ways of attaching ruby to a base this is.
///
/// One type, not three: §3.3.7 states that a jukugo compound whose every base carries two
/// or fewer ruby characters *is* composed as mono-ruby, and §3.3.1's note says the two then
/// produce identical geometry and differ only in line-adjustment behavior.
///
/// JLReq: §3.3.5, §3.3.6, §3.3.7
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RubyStyle {
    /// One run per base character, so two adjacent annotated bases are *different* cl-22
    /// runs — which is what gives them §E.2 note 6's quarter-em expansion opportunity, with
    /// no special case. §3.3.1's note names the pairs: in Figure 107 "the inter-character
    /// spacing between 鬼 and 門, or, 方 and 角 can be expanded".
    ///
    /// That expansion opportunity is not the same fact as the quarter em §3.3.1's other
    /// note reports between 凝 and 視. That one is *natural advance*: 凝 carries three ruby
    /// characters, §3.3.8 rule 1 forbids ruby from overhanging an adjacent cl-19 character,
    /// so the bases are forced apart before composition begins — which is why the note
    /// concludes that such a line "needs some line adjustment processing" rather than that
    /// it offers some. [`crate::lower`] emits it as extent (`Contribution::separations`);
    /// conflating the two composes every mono-ruby line short.
    ///
    /// JLReq: §3.3.5, §3.3.1, §3.3.8, `decision:mono-ruby-separation-split`
    MonoRuby,
    /// One run over the whole base: internally unbreakable and unexpandable, from the same
    /// same-run predicate. §3.3.6's own distribution of the surplus
    /// between base characters
    /// (`Question::GROUP_RUBY_DISTRIBUTION`) is real geometry now for the ruby-not-longer-
    /// than-base half — `crate::place`'s own module doc states the arithmetic in full, over
    /// both of the question's `jis` and `flush` answers — computed once a composed line's
    /// own placements exist, so `lower` itself still gives a group-ruby run only its
    /// identity and its block demand, and still produces no `Separation`. The
    /// ruby-longer-than-base half declines instead: its own method spreads the *base*
    /// characters apart, which `crate::place` cannot do
    /// (`crate::place::Attachments::declined`'s own doc states why).
    ///
    /// JLReq: §3.3.6
    GroupRuby,
    /// One run per base character, but the compound is one object that may split between
    /// base characters and not inside one base character's ruby. §3.3.7's own layout is
    /// real geometry now, in `crate::place`: paragraph 1 delegates each ≤2-character run to
    /// §3.3.5's own method, and paragraph 2's own `group` answer
    /// (`Question::JUKUGO_RUBY_LAYOUT`) reuses §3.3.6's own geometry, forced to `jis`
    /// (`decision:jukugo-group-layout-distribution`). §F's own `phonetic` answer stays an
    /// unfilled slot this round declines outright rather than implementing any part of.
    ///
    /// JLReq: §3.3.7, §C.2#8
    JukugoRuby,
}

/// Which side of the base a mono-ruby character's virtual body aligns to.
///
/// JLReq: §3.3.5
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RubyAlignment {
    /// 中付き: inline-axis center alignment. Permitted in both directions.
    ///
    /// JLReq: §3.3.5
    Nakatsuki,
    /// 肩付き: inline-start alignment. §3.3.5 says it "should not be adopted" in horizontal
    /// writing, so this is a caller choice ([`jlreq_spec::Question::RUBY_ALIGNMENT`]) whose
    /// `Policy::JLREQ` value follows the recommendation, not a hard error.
    ///
    /// JLReq: §3.3.5
    Katatsuki,
}

// There is deliberately no ruby-size type. §3.3.3 names half the base size as the principle
// and one-third ruby (三分ルビ) as a variant, and then says that for headings at twelve
// points or more the ruby "is generally smaller than half the size of the base characters"
// with no ratio given — so the set is not closed and no enumeration states it. The caller
// shaped the reading at some size and measured it there, and ADR-0002 makes that
// measurement the carrier: the ruby em is `Annotation::size_of`, full stop (ADR-0019).

/// One run of ruby characters against the base characters it reads.
///
/// `base` indexes the annotated stream and `annotation` the ruby's own, and the two are
/// *different types*, so the pairing §3.3.7 and §C.2 note 8 turn on cannot be written
/// backwards: a break is permitted between two base characters of a jukugo complex and
/// never inside one base character's reading (ADR-0016).
///
/// JLReq: §3.3.3, §3.3.7, §C.2#8
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RubyRun {
    base_first: ItemIndex,
    base_past: ItemIndex,
    annotation_first: AnnotationIndex,
    annotation_past: AnnotationIndex,
}

impl RubyRun {
    /// One run: `base`, read by `annotation`.
    ///
    /// A caller supplies these, so it can build them: the previous revision left this type
    /// with private fields and no constructor, which made ruby undeclarable — the failure
    /// ADR-0012's constructor check exists to catch.
    ///
    /// JLReq: §3.3.3, §3.3.7
    #[must_use]
    pub const fn new(base: Range<ItemIndex>, annotation: Range<AnnotationIndex>) -> Self {
        Self {
            base_first: base.start,
            base_past: base.end,
            annotation_first: annotation.start,
            annotation_past: annotation.end,
        }
    }

    /// The base range this run reads.
    ///
    /// JLReq: §3.3.3
    #[must_use]
    pub const fn base(self) -> Range<ItemIndex> {
        self.base_first..self.base_past
    }

    /// The annotation range that reads it.
    ///
    /// JLReq: §3.3.3
    #[must_use]
    pub const fn annotation(self) -> Range<AnnotationIndex> {
        self.annotation_first..self.annotation_past
    }
}

/// The ordinal `len` items past the head, as an [`ItemIndex`] an error can name.
fn item_ordinal(len: usize) -> ItemIndex {
    ItemIndex::new(u32::try_from(len).unwrap_or(u32::MAX))
}

/// The ordinal `len` annotation items past the head, as an [`AnnotationIndex`] an error can
/// name.
fn annotation_ordinal(len: usize) -> AnnotationIndex {
    AnnotationIndex::new(u32::try_from(len).unwrap_or(u32::MAX))
}

/// `base` lies inside `text`.
fn check_base_in_text(text: Text<'_>, base: Range<ItemIndex>) -> Result<(), RubyError> {
    let text_len = item_ordinal(text.items().len());
    if base.start > base.end || base.end > text_len {
        return Err(RubyError::BaseOutOfRange { at: base.end });
    }
    Ok(())
}

/// The run count `style` requires matches what the caller supplied.
fn check_run_count(
    base: Range<ItemIndex>,
    style: RubyStyle,
    found: usize,
) -> Result<(), RubyError> {
    let items = base.end.get().saturating_sub(base.start.get());
    let expected = match style {
        RubyStyle::GroupRuby => 1,
        RubyStyle::MonoRuby | RubyStyle::JukugoRuby => usize::try_from(items).unwrap_or(usize::MAX),
    };
    if found == expected {
        Ok(())
    } else {
        Err(RubyError::RunCount { expected, found })
    }
}

/// Every run's base lies inside `base`, every run's annotation lies inside `annotation`, and
/// the runs cover both in order without overlap.
fn check_runs(
    base: Range<ItemIndex>,
    annotation: Annotation<'_>,
    runs: &[RubyRun],
) -> Result<(), RubyError> {
    let annotation_len = annotation_ordinal(annotation.items().len());
    let mut next_base = base.start;
    let mut next_annotation = AnnotationIndex::new(0);
    for (index, run) in runs.iter().enumerate() {
        let run_base = run.base();
        let run_annotation = run.annotation();
        if run_base.start < base.start || run_base.end > base.end {
            return Err(RubyError::BaseOutOfRange { at: run_base.end });
        }
        if run_annotation.start > annotation_len || run_annotation.end > annotation_len {
            return Err(RubyError::AnnotationOutOfRange {
                at: run_annotation.end,
            });
        }
        if run_base.start != next_base || run_base.start >= run_base.end {
            return Err(RubyError::RunsNotContiguous { at: index });
        }
        if run_annotation.start != next_annotation || run_annotation.start >= run_annotation.end {
            return Err(RubyError::RunsNotContiguous { at: index });
        }
        next_base = run_base.end;
        next_annotation = run_annotation.end;
    }
    if next_base != base.end || next_annotation != annotation_len {
        return Err(RubyError::RunsNotContiguous { at: runs.len() });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use jlreq_class::{Annotation, AnnotationIndex, Text};
    use jlreq_unit::{Advance, ByteOffset, InlineExtent, Item, ItemIndex, Scale, ScaleId};

    use super::{Ruby, RubyError, RubyRun, RubyStyle};

    /// A one-em square size.
    fn base_scale() -> Scale {
        Scale::square(Advance::new(1000).unwrap()).expect("a positive em")
    }

    /// A ruby-sized square size.
    fn ruby_scale() -> Scale {
        Scale::square(Advance::new(500).unwrap()).expect("a positive em")
    }

    /// One item at `start`, one em wide, at the base size.
    fn item(start: u32) -> Item {
        Item::new(
            ByteOffset::new(start),
            InlineExtent::new(1000).unwrap(),
            ScaleId::BASE,
        )
    }

    /// Three ideographs, one item each.
    fn base_items() -> [Item; 3] {
        [item(0), item(3), item(6)]
    }

    /// Three ruby-sized reading items, one per base character.
    fn reading_items() -> [Item; 3] {
        [
            Item::new(
                ByteOffset::new(0),
                InlineExtent::new(500).unwrap(),
                ScaleId::BASE,
            ),
            Item::new(
                ByteOffset::new(3),
                InlineExtent::new(500).unwrap(),
                ScaleId::BASE,
            ),
            Item::new(
                ByteOffset::new(6),
                InlineExtent::new(500).unwrap(),
                ScaleId::BASE,
            ),
        ]
    }

    #[test]
    fn mono_ruby_needs_one_run_per_base_item() {
        let base = base_items();
        let scales = [base_scale()];
        let text = Text::new("鬼門方", &base, &scales).expect("three ideographs");
        let reading = reading_items();
        let reading_scales = [ruby_scale()];
        let annotation = Annotation::new("き", &reading[..1], &reading_scales).expect("one kana");
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(1),
        )];
        let refused = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(3),
            annotation,
            &runs,
            RubyStyle::MonoRuby,
        )
        .expect_err("three base items need three runs, not one");
        assert_eq!(
            refused,
            RubyError::RunCount {
                expected: 3,
                found: 1
            }
        );
    }

    #[test]
    fn mono_ruby_with_one_run_per_base_item_is_well_formed() {
        let base = base_items();
        let scales = [base_scale()];
        let text = Text::new("鬼門方", &base, &scales).expect("three ideographs");
        let reading = reading_items();
        let reading_scales = [ruby_scale()];
        let annotation = Annotation::new("きもか", &reading, &reading_scales)
            .expect("three kana, one per base item");
        let runs = [
            RubyRun::new(
                ItemIndex::new(0)..ItemIndex::new(1),
                AnnotationIndex::new(0)..AnnotationIndex::new(1),
            ),
            RubyRun::new(
                ItemIndex::new(1)..ItemIndex::new(2),
                AnnotationIndex::new(1)..AnnotationIndex::new(2),
            ),
            RubyRun::new(
                ItemIndex::new(2)..ItemIndex::new(3),
                AnnotationIndex::new(2)..AnnotationIndex::new(3),
            ),
        ];
        assert!(
            Ruby::new(
                text,
                ItemIndex::new(0)..ItemIndex::new(3),
                annotation,
                &runs,
                RubyStyle::MonoRuby,
            )
            .is_ok()
        );
    }

    #[test]
    fn group_ruby_needs_exactly_one_run() {
        let base = base_items();
        let scales = [base_scale()];
        let text = Text::new("鬼門方", &base, &scales).expect("three ideographs");
        let reading = reading_items();
        let reading_scales = [ruby_scale()];
        let annotation = Annotation::new("きも", &reading[..2], &reading_scales).expect("two kana");
        let runs = [
            RubyRun::new(
                ItemIndex::new(0)..ItemIndex::new(1),
                AnnotationIndex::new(0)..AnnotationIndex::new(1),
            ),
            RubyRun::new(
                ItemIndex::new(1)..ItemIndex::new(3),
                AnnotationIndex::new(1)..AnnotationIndex::new(2),
            ),
        ];
        let refused = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(3),
            annotation,
            &runs,
            RubyStyle::GroupRuby,
        )
        .expect_err("group-ruby needs exactly one run over the whole base");
        assert_eq!(
            refused,
            RubyError::RunCount {
                expected: 1,
                found: 2
            }
        );
    }

    #[test]
    fn a_base_range_outside_the_text_is_refused() {
        let base = base_items();
        let scales = [base_scale()];
        let text = Text::new("鬼門方", &base, &scales).expect("three ideographs");
        let reading = reading_items();
        let reading_scales = [ruby_scale()];
        let annotation = Annotation::new("き", &reading[..1], &reading_scales).expect("one kana");
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(4),
            AnnotationIndex::new(0)..AnnotationIndex::new(1),
        )];
        let refused = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(4),
            annotation,
            &runs,
            RubyStyle::GroupRuby,
        )
        .expect_err("the text has three items, not four");
        assert_eq!(
            refused,
            RubyError::BaseOutOfRange {
                at: ItemIndex::new(4)
            }
        );
    }

    #[test]
    fn a_run_whose_annotation_reaches_past_the_reading_is_refused() {
        let base = base_items();
        let scales = [base_scale()];
        let text = Text::new("鬼門方", &base, &scales).expect("three ideographs");
        let reading = reading_items();
        let reading_scales = [ruby_scale()];
        let annotation = Annotation::new("き", &reading[..1], &reading_scales).expect("one kana");
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(2),
        )];
        let refused = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(1),
            annotation,
            &runs,
            RubyStyle::MonoRuby,
        )
        .expect_err("the reading has one item, not two");
        assert_eq!(
            refused,
            RubyError::AnnotationOutOfRange {
                at: AnnotationIndex::new(2)
            }
        );
    }

    #[test]
    fn a_single_run_that_falls_short_of_the_whole_base_is_refused() {
        // `expected == found` (one run, exactly what `RubyStyle::GroupRuby` requires) is not
        // enough: the one run's own base still has to cover the whole declared base range.
        let base = base_items();
        let scales = [base_scale()];
        let text = Text::new("鬼門方", &base, &scales).expect("three ideographs");
        let reading = reading_items();
        let reading_scales = [ruby_scale()];
        let annotation = Annotation::new("きも", &reading[..2], &reading_scales).expect("two kana");
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(2),
            AnnotationIndex::new(0)..AnnotationIndex::new(2),
        )];
        let refused = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(3),
            annotation,
            &runs,
            RubyStyle::GroupRuby,
        )
        .expect_err("the one run covers only two of the three base items");
        assert_eq!(refused, RubyError::RunsNotContiguous { at: 1 });
    }

    #[test]
    fn a_with_alignment_override_is_read_back_by_lower() {
        // `alignment()` is crate-visible, so this exercises `with_alignment` through the
        // same accessor `lower` reads rather than through a public one this type does not
        // have.
        let base = base_items();
        let scales = [base_scale()];
        let text = Text::new("鬼門方", &base, &scales).expect("three ideographs");
        let reading = reading_items();
        let reading_scales = [ruby_scale()];
        let annotation = Annotation::new("き", &reading[..1], &reading_scales).expect("one kana");
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(1),
        )];
        let ruby = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(1),
            annotation,
            &runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run, one base item")
        .with_alignment(super::RubyAlignment::Katatsuki);
        assert_eq!(ruby.alignment(), Some(super::RubyAlignment::Katatsuki));
    }

    #[test]
    fn text_reports_the_stream_it_was_built_over() {
        // Otherwise crate-visible and unread by anything else in this crate but
        // `crate::lower::check_ruby_bounds`'s own consistency check.
        let base = base_items();
        let scales = [base_scale()];
        let text = Text::new("鬼門方", &base, &scales).expect("three ideographs");
        let reading = reading_items();
        let reading_scales = [ruby_scale()];
        let annotation = Annotation::new("き", &reading[..1], &reading_scales).expect("one kana");
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(1),
        )];
        let ruby = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(1),
            annotation,
            &runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run, one base item");
        assert_eq!(ruby.text(), text);
    }
}
