// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The two streams, and the well-formedness check that is why they live here.
//!
//! A [`Text`] is one running-text stream and an [`Annotation`] is one annotation stream;
//! they differ in the ordinal that indexes them and in nothing else (ADR-0016). Both are
//! validated by one routine, because annotation characters are classified too.
//!
//! `Text` is in `jlreq-class` rather than in `jlreq-unit` because its validity is a
//! statement about Appendix A, and a constructor that cannot read the table it checks
//! against is a constructor that documents its invariant instead of holding it
//! (ADR-0018).
//!
//! # What a well-formed stream is
//!
//! Six things, and the last three are the substance of ADR 0018.
//!
//! The scale table is non-empty and no longer than [`Carry::SIZES`], and every item names a
//! size in it, so [`Text::size_of`] answers without an error channel. Byte offsets are
//! strictly increasing, land on character boundaries, and stay in range, so no downstream
//! slice can panic — this is a `no_std` library that may run in an interrupt handler.
//!
//! Every item is exactly one Appendix A key, with the one exception a Western ligature is.
//! No key begins inside one item and ends in the next. And every item whose key Appendix A
//! names under one of §3.1.2's five classes declares a frame (字幅), because there the frame
//! decides a geometry rather than a class and an unstated geometry has no answer to report.
//!
//! # What is deliberately not checked
//!
//! Bytes before the first item's offset belong to no occurrence. ADR 0018 states three
//! refusals and this is not one of them: an item names where its own occurrence begins, and
//! a stream whose first item starts past the head of the text is a segmentation the caller
//! chose rather than a claim about Appendix A. It is stated here rather than discovered,
//! because [`Text::cluster`] answers over the caller's own segmentation and a reader is
//! owed the boundary.

use jlreq_unit::{Carry, Frame, Item, ItemIndex, Scale, ScaleId, Size};

use crate::class::Class;
use crate::classify::{listed_classes, literally_listed_classes};
use crate::member::{Member, members};

/// Text, its items, and its scale table: the single carrier of what the caller knows
/// about **one running-text stream**.
///
/// A stream is one string in reading order. A paragraph is one; each annotation — each
/// ruby run's reading, each interlinear reference mark — is an [`Annotation`] (ADR-0016).
/// Tate-chu-yoko (縦中横), warichu (割注), furiwake (振り分け) and the ornamented complex
/// are *not* annotations: their characters are running text and live in the stream they
/// read in.
///
/// Deliberately not called a *run*: JLReq spends that word on construct instances — "two
/// adjacent characters of the same ornamented character complex (cl-21) run" — and
/// `jlreq_unit::RunId` is that sense. Nothing else in this workspace may take the word.
///
/// Construction validates six things, and the last three are why this type is here. Byte
/// offsets are strictly increasing, land on character boundaries, and stay in range, so no
/// downstream slice can panic — this is a `no_std` library that may run in an interrupt
/// handler. Every item names a declared scale, and the table is non-empty and no longer
/// than [`Carry::SIZES`]. Every item is exactly one Appendix A key, with the one exception
/// [`Item`] states. And every item whose key Appendix A names under any of the five classes
/// of §3.1.2 declares a frame, because there the frame decides a geometry and an unstated
/// geometry has no answer to report.
///
/// JLReq: §A, §3.1.2, ADR-0002, ADR-0016, ADR-0018
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Text<'t> {
    /// The stream, in reading order.
    stream: &'t str,
    /// One item per occurrence, in reading order.
    items: &'t [Item],
    /// The character sizes the stream declares.
    scales: &'t [Scale],
    /// The first declared size, which every stream has and which is what an ordinal past
    /// the end answers with. Held rather than looked up, so `size_of` is total without a
    /// fabricated value standing in for the arm construction already ruled out.
    base: Size,
}

impl<'t> Text<'t> {
    /// The stream `text`, segmented into `items`, set at the sizes `scales` declares.
    ///
    /// JLReq: §A, §3.1.2, ADR-0018
    pub fn new(text: &'t str, items: &'t [Item], scales: &'t [Scale]) -> Result<Self, TextError> {
        let base = validate(text, items, scales)?;
        Ok(Self {
            stream: text,
            items,
            scales,
            base,
        })
    }

    /// The stream, as the caller wrote it.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn as_str(self) -> &'t str {
        self.stream
    }

    /// The occurrences, in reading order.
    ///
    /// JLReq: §A, ADR-0018
    #[must_use]
    pub const fn items(self) -> &'t [Item] {
        self.items
    }

    /// The character sizes this stream declares.
    ///
    /// JLReq: §B.1, §3.3.3
    #[must_use]
    pub const fn scales(self) -> &'t [Scale] {
        self.scales
    }

    /// The size of one item: its [`Scale`] together with the ordinal [`Carry`] keys on.
    /// The only source of a [`Size`], which is what makes the per-size exactness claim of
    /// ADR-0007 hold by construction rather than by discipline.
    ///
    /// An ordinal past the end names no item and answers with the stream's first declared
    /// size, because a stream always declares one and there is no size of an occurrence
    /// that is not there. [`Text::items`] is what a caller bounds an ordinal against; the
    /// alternative to a stated answer here is a panic in a library whose selling point is
    /// that it has none.
    ///
    /// JLReq: §B.1, §3.3.3, ADR-0007
    #[must_use]
    pub fn size_of(self, item: ItemIndex) -> Size {
        size_of(self.items, self.scales, item.get(), self.base)
    }

    /// The size declared at `id`, or `None` when this stream declares none there.
    ///
    /// JLReq: §B.1, §3.3.3
    #[must_use]
    pub fn size(self, id: ScaleId) -> Option<Size> {
        size(self.scales, id)
    }

    /// The cluster text of one item.
    ///
    /// An ordinal past the end covers nothing and answers with the empty string, for the
    /// reason [`Text::size_of`] states.
    ///
    /// JLReq: §A, ADR-0018
    #[must_use]
    pub fn cluster(self, item: ItemIndex) -> &'t str {
        cluster(self.stream, self.items, item.get())
    }
}

/// One annotation stream: the same shape as a [`Text`], indexed by a different ordinal.
///
/// Ruby readings and interlinear reference marks are annotations. The type exists so that
/// a base range and an annotation range cannot be swapped — in a call, or inside a
/// `jlreq_inline::RubyRun` — which the previous revision left to field order and a comment
/// (ADR-0016).
///
/// Validated by exactly the same routine as [`Text`], because annotation characters are
/// classified too: ruby text has its own classes and its own boundaries. There is
/// deliberately no conversion in either direction, since one would reinstate the confusion
/// the two types exist to prevent.
///
/// JLReq: §3.3.1–§3.3.8, §4.2.3, ADR-0016, ADR-0018
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Annotation<'a> {
    /// The stream, in reading order.
    stream: &'a str,
    /// One item per occurrence, in reading order.
    items: &'a [Item],
    /// The character sizes the stream declares; the first is the ruby em.
    scales: &'a [Scale],
    /// The first declared size: the ruby em. See [`Text`]'s field of the same name.
    base: Size,
}

impl<'a> Annotation<'a> {
    /// The annotation stream `text`, segmented into `items`, set at `scales`.
    ///
    /// JLReq: §3.3.1, §A, ADR-0016, ADR-0018
    pub fn new(text: &'a str, items: &'a [Item], scales: &'a [Scale]) -> Result<Self, TextError> {
        let base = validate(text, items, scales)?;
        Ok(Self {
            stream: text,
            items,
            scales,
            base,
        })
    }

    /// The stream, as the caller wrote it.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn as_str(self) -> &'a str {
        self.stream
    }

    /// The occurrences, in reading order.
    ///
    /// JLReq: §A, ADR-0018
    #[must_use]
    pub const fn items(self) -> &'a [Item] {
        self.items
    }

    /// The character sizes this stream declares.
    ///
    /// JLReq: §3.3.3
    #[must_use]
    pub const fn scales(self) -> &'a [Scale] {
        self.scales
    }

    /// The ruby em, and the only statement of it: §3.3.8's overhang allowances and
    /// §3.3.6's distribution both read this, so they cannot disagree about what one is
    /// (ADR-0019).
    ///
    /// JLReq: §3.3.3, §3.3.6, §3.3.8
    #[must_use]
    pub fn size_of(self, item: AnnotationIndex) -> Size {
        size_of(self.items, self.scales, item.get(), self.base)
    }

    /// The size declared at `id`, or `None` when this stream declares none there.
    ///
    /// JLReq: §3.3.3
    #[must_use]
    pub fn size(self, id: ScaleId) -> Option<Size> {
        size(self.scales, id)
    }

    /// The cluster text of one item.
    ///
    /// JLReq: §A, ADR-0018
    #[must_use]
    pub fn cluster(self, item: AnnotationIndex) -> &'a str {
        cluster(self.stream, self.items, item.get())
    }
}

/// An ordinal into one [`Annotation`]'s items. See [`ItemIndex`], which it is deliberately
/// not.
///
/// JLReq: n/a (addressing)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct AnnotationIndex(u32);

impl AnnotationIndex {
    /// The item at ordinal `index` of the annotation stream.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The ordinal.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Why a stream is not one this library can answer for.
///
/// Nine refusals, and each names the item it fired on so the caller's work is directed
/// rather than guessed at. The ordinal is an [`ItemIndex`] in both streams, because an
/// annotation is validated by the same routine and a second error type would be one more
/// place for the two to disagree about what "the third item" means.
///
/// JLReq: §A, §3.1.2, §3.2.6, ADR-0018
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextError {
    /// Offsets are not strictly increasing.
    OffsetsNotMonotonic {
        /// The item whose offset does not follow the one before it.
        at: ItemIndex,
    },
    /// An offset does not land on a character boundary.
    OffsetNotOnBoundary {
        /// The item whose offset falls inside a code point.
        at: ItemIndex,
    },
    /// An offset lies outside the text.
    OffsetOutOfRange {
        /// The item whose offset is past the end of the stream.
        at: ItemIndex,
    },
    /// An item names a scale the table does not have.
    UnknownScale {
        /// The item naming a size the stream never declared.
        at: ItemIndex,
    },
    /// The scale table is empty, or longer than [`Carry::SIZES`].
    ScaleCount {
        /// How many sizes the stream declared.
        declared: usize,
    },
    /// An Appendix A key begins inside this item and ends in the next. Merge them: the
    /// pair is one occurrence and no matrix is indexed inside one. JLReq: §A
    MemberCrossesItem {
        /// The item the key begins in.
        at: ItemIndex,
        /// The key that crosses the boundary.
        key: Member,
    },
    /// This item covers several Appendix A keys and is not a Western ligature — that is,
    /// it does not declare [`Frame::Proportional`] with every key in cl-27.
    ///
    /// JLReq: §A, §3.2.6
    ItemCoversSeveralMembers {
        /// The item covering more than one key.
        at: ItemIndex,
        /// How many keys it covers.
        keys: u16,
    },
    /// The frame is unstated on an item Appendix A names under one of the five classes
    /// whose advance §3.1.2 states as half-width. There is no answer to report instead:
    /// on the ideographic frame the conditional space is inside the supplied advance and
    /// on every other it is added, and a default would put a half-em guess at the
    /// commonest adjacency in Japanese. JLReq: §3.1.2
    FrameRequired {
        /// The item with no declared frame.
        at: ItemIndex,
        /// The class of §3.1.2's five that Appendix A names its key under.
        class: Class,
    },
}

impl TextError {
    /// Frozen projection (ADR-0012): whether the caller fixes this by cutting the text into
    /// items differently, rather than by changing what an item or the scale table declares.
    ///
    /// `true` for the five refusals about where the item boundaries fall — the three about
    /// offsets, the key that crosses one, and the item that covers several — and `false` for
    /// the three about a declaration: an unknown scale, an unusable scale table, and the
    /// frame §3.1.2 requires. The two answers are the two kinds of work, so a refusal added
    /// later is detail: a caller who branched on this keeps meaning what they meant, where a
    /// `match` with a catch-all arm would silently treat a new segmentation refusal as a
    /// declaration to change.
    ///
    /// JLReq: §A, §3.1.2, ADR-0018
    #[must_use]
    pub const fn is_segmentation(self) -> bool {
        matches!(
            self,
            Self::OffsetsNotMonotonic { .. }
                | Self::OffsetNotOnBoundary { .. }
                | Self::OffsetOutOfRange { .. }
                | Self::MemberCrossesItem { .. }
                | Self::ItemCoversSeveralMembers { .. }
        )
    }

    /// The item the caller must fix, where one item is what is wrong.
    ///
    /// `None` for [`TextError::ScaleCount`] alone, which is about the scale table rather
    /// than about any one item. An ordinary accessor and not the frozen projection above:
    /// its answer set is the stream's ordinals, which grow with the stream.
    ///
    /// JLReq: §A, §3.1.2, ADR-0018
    #[must_use]
    pub const fn at(self) -> Option<ItemIndex> {
        match self {
            Self::OffsetsNotMonotonic { at }
            | Self::OffsetNotOnBoundary { at }
            | Self::OffsetOutOfRange { at }
            | Self::UnknownScale { at }
            | Self::MemberCrossesItem { at, .. }
            | Self::ItemCoversSeveralMembers { at, .. }
            | Self::FrameRequired { at, .. } => Some(at),
            Self::ScaleCount { .. } => None,
        }
    }
}

/// The ordinal `index`, as an error names one.
fn ordinal(index: usize) -> ItemIndex {
    ItemIndex::new(u32::try_from(index).unwrap_or(u32::MAX))
}

/// The size of the item at `index`, or the stream's first declared size when no item is
/// there.
fn size_of(items: &[Item], scales: &[Scale], index: u32, base: Size) -> Size {
    let Some(item) = items.get(index as usize) else {
        return base;
    };
    size(scales, item.scale()).unwrap_or(base)
}

/// The size declared at `id`, or `None` when the table declares none there.
fn size(scales: &[Scale], id: ScaleId) -> Option<Size> {
    scales
        .get(id.index() as usize)
        .map(|scale| Size::new(id, *scale))
}

/// The text of the item at `index`, which runs to the next item's offset or to the end.
fn cluster<'t>(text: &'t str, items: &[Item], index: u32) -> &'t str {
    let position = index as usize;
    let Some(item) = items.get(position) else {
        return "";
    };
    let start = item.start().get() as usize;
    let end = items
        .get(position.saturating_add(1))
        .map_or(text.len(), |next| next.start().get() as usize);
    text.get(start..end).unwrap_or_default()
}

/// Whether a stream is one this library can answer for, and which refusal it meets first.
///
/// One routine for both streams, because annotation text is classified by the same tables
/// and a second copy is a second place for the two to disagree (ADR-0016, ADR-0018).
fn validate(text: &str, items: &[Item], scales: &[Scale]) -> Result<Size, TextError> {
    let base = check_scales(items, scales)?;
    check_offsets(text, items)?;
    check_members(text, items)?;
    check_frames(text, items)?;
    Ok(base)
}

/// The scale table is usable and every item names a size in it.
fn check_scales(items: &[Item], scales: &[Scale]) -> Result<Size, TextError> {
    let declared = scales.len();
    let Some(first) = scales.first().filter(|_| declared <= Carry::SIZES) else {
        return Err(TextError::ScaleCount { declared });
    };
    for (index, item) in items.iter().enumerate() {
        if size(scales, item.scale()).is_none() {
            return Err(TextError::UnknownScale { at: ordinal(index) });
        }
    }
    Ok(Size::new(ScaleId::BASE, *first))
}

/// Offsets are strictly increasing, land on character boundaries, and stay in range.
fn check_offsets(text: &str, items: &[Item]) -> Result<(), TextError> {
    let mut previous: Option<usize> = None;
    for (index, item) in items.iter().enumerate() {
        let start = item.start().get() as usize;
        if previous.is_some_and(|before| start <= before) {
            return Err(TextError::OffsetsNotMonotonic { at: ordinal(index) });
        }
        if start >= text.len() {
            return Err(TextError::OffsetOutOfRange { at: ordinal(index) });
        }
        if !text.is_char_boundary(start) {
            return Err(TextError::OffsetNotOnBoundary { at: ordinal(index) });
        }
        previous = Some(start);
    }
    Ok(())
}

/// Every item is exactly one Appendix A key, and no key crosses an item boundary.
///
/// The scan runs over the whole text rather than over each cluster, which is what lets the
/// crossing case be seen at all: a pair whose halves the caller split into two items is one
/// member of the global scan that ends past its own item's end.
///
/// It begins at the first item's own offset rather than at byte zero, because the bytes
/// before it belong to no occurrence — the one thing this module states it does not check.
/// Scanning from zero and skipping what began earlier let a key that starts before the first
/// item swallow it, and the item was then reported as covering *no* key under a variant
/// whose name says it covers several.
fn check_members(text: &str, items: &[Item]) -> Result<(), TextError> {
    let Some(first) = items.first() else {
        return Ok(());
    };
    let head = first.start().get() as usize;
    let Some(tail) = text.get(head..) else {
        return Ok(());
    };
    let mut covered = Coverage::Empty;
    let mut current: usize = 0;
    for (range, key) in members(tail) {
        let start = head.saturating_add(range.start.get() as usize);
        let finish = head.saturating_add(range.end.get() as usize);
        while items
            .get(current.saturating_add(1))
            .is_some_and(|next| (next.start().get() as usize) <= start)
        {
            check_covered(items, current, covered)?;
            covered = Coverage::Empty;
            current = current.saturating_add(1);
        }
        let end = items
            .get(current.saturating_add(1))
            .map_or(text.len(), |next| next.start().get() as usize);
        if finish > end {
            return Err(TextError::MemberCrossesItem {
                at: ordinal(current),
                key,
            });
        }
        covered = covered.and(key);
    }
    check_covered(items, current, covered)
}

/// What one item was found to cover.
///
/// `Empty` is a state and not a count, so no code path can report "this item covers several
/// keys" about an item that covers none.
#[derive(Debug, Clone, Copy)]
enum Coverage {
    /// No Appendix A key begins in this item.
    Empty,
    /// At least one does: how many, and whether cl-27 lists every one of them.
    Found {
        /// How many keys the item covers.
        keys: u16,
        /// Whether Appendix A lists every one of them under cl-27.
        western: bool,
    },
}

impl Coverage {
    /// This coverage with one more key found in the same item.
    ///
    /// The cl-27 test is the *literal* listing and never the folded one. ADR 0018's
    /// exception is that a proportional multi-code-point cluster "is a Western ligature and
    /// nothing else"; running the test through the compatibility folding passed `U+FF21` for
    /// a test written about `U+0041`, so two full-width Latin letters in one item were
    /// accepted as a ligature and then answered cl-27 with the specification's own standing,
    /// where §3.2.4 puts full-width Latin in cl-19.
    fn and(self, key: Member) -> Self {
        let western = literally_listed_classes(key).contains(Class::Western);
        match self {
            Self::Empty => Self::Found { keys: 1, western },
            Self::Found {
                keys,
                western: before,
            } => Self::Found {
                keys: keys.saturating_add(1),
                western: before && western,
            },
        }
    }
}

/// One item covers exactly one key, or is the one shape a shaper produces that covers
/// several.
///
/// §3.2.6 puts proportional Western characters in cl-27, so a proportional multi-code-point
/// cluster is a Western ligature and nothing else: Table 1 sets cl-27 against cl-27 solid
/// and §C.2 note 12 requires a caller-supplied hyphen before a Western word may be divided
/// at all, so there is no amount and no break inside such a cluster for the merge to have
/// destroyed (ADR-0018).
///
/// Both halves of that exception are required, and the second is not redundant: a caller
/// who declared the proportional frame over a cluster of kana has still handed one item two
/// occurrences that Table 1 puts an amount between.
///
/// [`Coverage::Empty`] is accepted, and that is not a silent pass. An item in which no key
/// begins is one whose own offset lies inside a key that began earlier, and such a key ends
/// past the previous item, so `check_members` has already refused it as
/// [`TextError::MemberCrossesItem`] before this function could be reached. Before the first
/// item there is nothing to refuse: those bytes belong to no occurrence, and the scan does
/// not begin until the first item's offset.
fn check_covered(items: &[Item], index: usize, covered: Coverage) -> Result<(), TextError> {
    let Coverage::Found { keys, western } = covered else {
        return Ok(());
    };
    if keys == 1 {
        return Ok(());
    }
    let Some(item) = items.get(index) else {
        return Ok(());
    };
    if western && item.frame() == Frame::Proportional {
        return Ok(());
    }
    Err(TextError::ItemCoversSeveralMembers {
        at: ordinal(index),
        keys,
    })
}

/// Every item whose key Appendix A names under one of §3.1.2's five classes declares a
/// frame.
fn check_frames(text: &str, items: &[Item]) -> Result<(), TextError> {
    for (index, item) in items.iter().enumerate() {
        if item.frame() != Frame::Unstated {
            continue;
        }
        let text_of_item = cluster(text, items, u32::try_from(index).unwrap_or(u32::MAX));
        for (_, key) in members(text_of_item) {
            if let Some(class) = listed_classes(key)
                .classes()
                .find(|class| class.advance_is_stated_half_width())
            {
                return Err(TextError::FrameRequired {
                    at: ordinal(index),
                    class,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use jlreq_unit::{
        Advance, ByteOffset, Carry, Frame, InlineExtent, Item, ItemIndex, Scale, ScaleId,
    };

    use super::{Annotation, AnnotationIndex, Text, TextError};
    use crate::class::Class;
    use crate::member::Member;

    /// A one-em square size, which every stream below declares first.
    fn base() -> Scale {
        Scale::square(Advance::new(1000).unwrap()).expect("a positive em")
    }

    /// One item at `start`, one em wide, at the base size.
    fn item(start: u32) -> Item {
        Item::new(
            ByteOffset::new(start),
            InlineExtent::new(1000).unwrap(),
            ScaleId::BASE,
        )
    }

    #[test]
    fn an_ordinary_japanese_stream_is_well_formed() {
        let items = [item(0), item(3)];
        let scales = [base()];
        let stream = Text::new("あい", &items, &scales).expect("two ideographs, two items");
        assert_eq!(stream.as_str(), "あい");
        assert_eq!(stream.items().len(), 2);
        assert_eq!(stream.cluster(ItemIndex::new(1)), "い");
    }

    #[test]
    fn an_empty_stream_with_no_item_is_well_formed() {
        let scales = [base()];
        let stream = Text::new("", &[], &scales).expect("an empty paragraph is a paragraph");
        assert_eq!(stream.items().len(), 0);
        assert_eq!(stream.cluster(ItemIndex::new(0)), "");
    }

    #[test]
    fn a_stream_declaring_no_size_is_refused() {
        assert_eq!(
            Text::new("あ", &[item(0)], &[]),
            Err(TextError::ScaleCount { declared: 0 }),
            "a caller with one size writes it explicitly rather than omitting it"
        );
    }

    #[test]
    fn a_scale_table_longer_than_the_carry_is_refused() {
        let scales = [base(); Carry::SIZES + 1];
        assert_eq!(
            Text::new("あ", &[item(0)], &scales),
            Err(TextError::ScaleCount {
                declared: Carry::SIZES + 1
            }),
            "the carry keeps one remainder per em without allocating, and the bound is what \
             makes the exactness claim true rather than nearly true"
        );
    }

    #[test]
    fn an_item_naming_a_size_the_stream_never_declared_is_refused() {
        let items = [item(0).with_frame(Frame::FullEm)];
        let misnamed = [Item::new(
            ByteOffset::new(0),
            InlineExtent::new(1000).unwrap(),
            ScaleId::new(3),
        )];
        assert!(Text::new("あ", &items, &[base()]).is_ok());
        assert_eq!(
            Text::new("あ", &misnamed, &[base()]),
            Err(TextError::UnknownScale {
                at: ItemIndex::new(0)
            })
        );
    }

    #[test]
    fn offsets_that_do_not_increase_are_refused() {
        let items = [item(3), item(0)];
        assert_eq!(
            Text::new("あい", &items, &[base()]),
            Err(TextError::OffsetsNotMonotonic {
                at: ItemIndex::new(1)
            })
        );
        let repeated = [item(0), item(0)];
        assert_eq!(
            Text::new("あい", &repeated, &[base()]),
            Err(TextError::OffsetsNotMonotonic {
                at: ItemIndex::new(1)
            }),
            "strictly increasing, so two items never name one occurrence"
        );
    }

    #[test]
    fn an_offset_inside_a_code_point_is_refused() {
        let items = [item(0), item(1)];
        assert_eq!(
            Text::new("あい", &items, &[base()]),
            Err(TextError::OffsetNotOnBoundary {
                at: ItemIndex::new(1)
            }),
            "a slice taken at that offset would panic, and this crate has no panic path"
        );
    }

    #[test]
    fn an_offset_past_the_end_is_refused() {
        let items = [item(0), item(6)];
        assert_eq!(
            Text::new("あい", &items, &[base()]),
            Err(TextError::OffsetOutOfRange {
                at: ItemIndex::new(1)
            }),
            "an item at the end of the text covers nothing, and every item is one key"
        );
    }

    #[test]
    fn a_key_split_across_two_items_is_refused() {
        // `<02E5, 02E9>` is a cl-27 falling tone contour whose first code point is also
        // listed alone, so splitting it yields two plausible answers instead of one correct
        // one (ADR-0018).
        let items = [
            item(0).with_frame(Frame::Proportional),
            item(2).with_frame(Frame::Proportional),
        ];
        assert_eq!(
            Text::new("\u{02E5}\u{02E9}", &items, &[base()]),
            Err(TextError::MemberCrossesItem {
                at: ItemIndex::new(0),
                key: Member::pair('\u{02E5}', '\u{02E9}'),
            }),
            "the caller merges the two glyphs into one item whose advance is their sum"
        );
    }

    #[test]
    fn the_merged_pair_is_well_formed() {
        let items = [item(0).with_frame(Frame::Proportional)];
        assert!(
            Text::new("\u{02E5}\u{02E9}", &items, &[base()]).is_ok(),
            "no cell of any matrix is indexed inside a key, so nothing is lost by stating \
             the pair as one advance"
        );
    }

    #[test]
    fn an_item_covering_two_japanese_keys_is_refused() {
        let items = [item(0)];
        assert_eq!(
            Text::new("あい", &items, &[base()]),
            Err(TextError::ItemCoversSeveralMembers {
                at: ItemIndex::new(0),
                keys: 2,
            }),
            "an item carries one advance, one frame, one role and one scale, so it cannot \
             describe two occurrences that disagree about any of them"
        );
    }

    #[test]
    fn a_western_ligature_on_the_proportional_frame_is_well_formed() {
        let items = [item(0).with_frame(Frame::Proportional)];
        assert!(
            Text::new("ffi", &items, &[base()]).is_ok(),
            "§3.2.6 puts proportional Western characters in cl-27, so a proportional \
             multi-code-point cluster is a Western ligature and nothing else"
        );
    }

    #[test]
    fn a_western_word_may_be_one_item_or_many() {
        let whole = [item(0).with_frame(Frame::Proportional)];
        assert!(Text::new("editor", &whole, &[base()]).is_ok());
        let letters: [Item; 6] = core::array::from_fn(|index| {
            item(u32::try_from(index).unwrap()).with_frame(Frame::Proportional)
        });
        assert!(
            Text::new("editor", &letters, &[base()]).is_ok(),
            "§3.2.1's own example of Western text in Japanese is six items or one, and both \
             are well formed"
        );
    }

    #[test]
    fn a_full_width_multi_key_item_is_refused_even_though_a_proportional_one_is_not() {
        let items = [item(0).with_frame(Frame::FullEm)];
        let scales = [base()];
        assert_eq!(
            Text::new("ffi", &items, &scales),
            Err(TextError::ItemCoversSeveralMembers {
                at: ItemIndex::new(0),
                keys: 3,
            }),
            "the exception is the shaper's own output and nothing else"
        );
    }

    #[test]
    fn a_proportional_item_covering_keys_cl_27_does_not_name_is_still_refused() {
        // Both halves of ADR 0018's exception are required, and this is why the second is
        // not redundant: the frame alone would let one item carry two kana that Table 1
        // puts an amount between.
        let items = [item(0).with_frame(Frame::Proportional)];
        let scales = [base()];
        assert_eq!(
            Text::new("あい", &items, &scales),
            Err(TextError::ItemCoversSeveralMembers {
                at: ItemIndex::new(0),
                keys: 2,
            }),
            "§3.2.6's exception is a Western ligature; a declared frame does not make two \
             hiragana into one"
        );
    }

    #[test]
    fn two_full_width_latin_letters_in_one_item_are_not_a_western_ligature() {
        // ADR 0018's exception is that a proportional multi-code-point cluster "is a Western
        // ligature and nothing else", and the cl-27 test has to be the literal listing:
        // U+FF21 folds onto U+0041, so a test that ran through the folding accepted this and
        // `classify` then answered cl-27 with the specification's own standing, where §3.2.4
        // puts full-width Latin in cl-19.
        let items = [item(0).with_frame(Frame::Proportional)];
        assert_eq!(
            Text::new("ＡＢ", &items, &[base()]),
            Err(TextError::ItemCoversSeveralMembers {
                at: ItemIndex::new(0),
                keys: 2,
            }),
            "§A.27 lists U+0041 and not U+FF21"
        );
        assert!(
            Text::new("AB", &items, &[base()]).is_ok(),
            "and the shape a shaper actually produces is still well formed"
        );
    }

    #[test]
    fn an_item_beginning_inside_an_earlier_key_covers_its_own_occurrence() {
        // The bytes before the first item belong to no occurrence, so the scan begins at
        // that item's own offset. Scanning from zero and skipping what began earlier let
        // `<02E5, 02E9>` swallow this item, which was then reported as covering *no* key
        // under a variant whose name says it covers several.
        let items = [item(2).with_frame(Frame::Proportional)];
        let scales = [base()];
        let stream = Text::new("\u{02E5}\u{02E9}", &items, &scales)
            .expect("one item, one key, and a preceding byte range that is no occurrence");
        assert_eq!(stream.cluster(ItemIndex::new(0)), "\u{02E9}");
    }

    #[test]
    fn a_refusal_names_the_item_the_caller_must_fix_and_what_kind_of_work_it_is() {
        let covering = Text::new("あい", &[item(0)], &[base()]).expect_err("one item, two keys");
        assert_eq!(covering.at(), Some(ItemIndex::new(0)));
        assert!(
            covering.is_segmentation(),
            "the caller cuts the text differently"
        );
        let table = Text::new("あ", &[item(0)], &[]).expect_err("no size is declared");
        assert_eq!(
            table.at(),
            None,
            "the scale table is not any one item's, and the accessor says so rather than \
             naming an item that is not at fault"
        );
        assert!(
            !table.is_segmentation(),
            "and no re-segmentation would fix a stream that declares no size"
        );
        let frame = Text::new("。", &[item(0)], &[base()]).expect_err("§3.1.2's five");
        assert!(
            !frame.is_segmentation(),
            "an unstated frame is a declaration the caller adds, not a boundary they move"
        );
    }

    #[test]
    fn an_unstated_frame_on_one_of_section_3_1_2s_five_classes_is_refused() {
        let items = [item(0)];
        assert_eq!(
            Text::new("。", &items, &[base()]),
            Err(TextError::FrameRequired {
                at: ItemIndex::new(0),
                class: Class::FullStop,
            }),
            "there is no answer to report instead: an unstated class has candidates and a \
             separating axis, and an unstated geometry has neither"
        );
    }

    #[test]
    fn the_same_item_with_a_frame_is_well_formed_on_either_reading() {
        for frame in [Frame::FullEm, Frame::HalfEm] {
            let items = [item(0).with_frame(frame)];
            assert!(
                Text::new("。", &items, &[base()]).is_ok(),
                "both readings are correct and they are the same geometry reached from \
                 opposite directions (ADR-0017)"
            );
        }
    }

    #[test]
    fn a_full_width_bracket_needs_a_frame_through_the_folding() {
        let items = [item(0)];
        assert_eq!(
            Text::new("）", &items, &[base()]),
            Err(TextError::FrameRequired {
                at: ItemIndex::new(0),
                class: Class::ClosingBracket,
            }),
            "§A lists U+0029 where real text carries U+FF09, so the requirement has to \
             follow the folding or it never fires on the commonest punctuation in Japanese"
        );
    }

    #[test]
    fn an_ideograph_needs_no_frame() {
        let items = [item(0)];
        assert!(
            Text::new("漢", &items, &[base()]).is_ok(),
            "§3.1.2 states the advance of five classes and cl-19 is not one of them"
        );
    }

    #[test]
    fn the_size_of_an_item_is_the_one_it_names() {
        let ruby = Scale::square(Advance::new(500).unwrap()).expect("a positive em");
        let items = [item(0), {
            let mut second = Item::new(
                ByteOffset::new(3),
                InlineExtent::new(500).unwrap(),
                ScaleId::new(1),
            );
            second = second.with_frame(Frame::FullEm);
            second
        }];
        let scales = [base(), ruby];
        let stream = Text::new("あい", &items, &scales).expect("two items");
        assert_eq!(stream.size_of(ItemIndex::new(1)).scale(), ruby);
        assert_eq!(stream.size_of(ItemIndex::new(1)).id(), ScaleId::new(1));
        assert_eq!(
            stream.size(ScaleId::new(1)).map(jlreq_unit::Size::scale),
            Some(ruby)
        );
        assert_eq!(
            stream.size(ScaleId::new(2)),
            None,
            "a size the stream never declared has no value, which is why this one answers \
             with an option and `size_of` does not"
        );
    }

    #[test]
    fn an_ordinal_past_the_end_answers_with_the_streams_first_size_and_no_text() {
        let items = [item(0)];
        let scales = [base()];
        let stream = Text::new("あ", &items, &scales).expect("one item");
        assert_eq!(stream.size_of(ItemIndex::new(7)).scale(), base());
        assert_eq!(
            stream.cluster(ItemIndex::new(7)),
            "",
            "an occurrence that is not there covers nothing"
        );
    }

    #[test]
    fn bytes_before_the_first_item_belong_to_no_occurrence() {
        let items = [item(3)];
        let scales = [base()];
        let stream = Text::new("あい", &items, &scales).expect("one item, starting at い");
        assert_eq!(
            stream.cluster(ItemIndex::new(0)),
            "い",
            "an item names where its own occurrence begins; ADR 0018 states three refusals \
             and a stream that starts past the head of its text is not one of them"
        );
    }

    #[test]
    fn an_annotation_is_validated_by_the_same_routine() {
        let items = [item(0)];
        assert_eq!(
            Annotation::new("。", &items, &[base()]),
            Err(TextError::FrameRequired {
                at: ItemIndex::new(0),
                class: Class::FullStop,
            }),
            "annotation characters are classified too, so ruby text is well formed by the \
             same rule (ADR-0016)"
        );
    }

    #[test]
    fn an_annotation_reads_its_own_ordinal() {
        let ruby = Scale::square(Advance::new(500).unwrap()).expect("a positive em");
        let items = [
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
        ];
        let scales = [ruby];
        let reading = Annotation::new("かん", &items, &scales).expect("two kana");
        assert_eq!(reading.cluster(AnnotationIndex::new(1)), "ん");
        assert_eq!(reading.size_of(AnnotationIndex::new(0)).scale(), ruby);
        assert_eq!(reading.as_str(), "かん");
        assert_eq!(reading.scales().len(), 1);
        assert_eq!(reading.items().len(), 2);
    }
}
