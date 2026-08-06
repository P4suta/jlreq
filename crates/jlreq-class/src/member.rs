// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Appendix A key, the scan that finds one in a text, and the compatibility folding.
//!
//! Appendix A is indexed by an ordered sequence of code points and not by a `char`. Most of
//! its 1133 keys are one code point; twenty-five are an ordered pair, and cl-27 lists
//! `<02E5, 02E9>` and `<02E9, 02E5>` as two distinct members, so the order matters and a
//! set would lose it (`docs/adr/0008`).
//!
//! # The two lookups, and why the literal one comes first
//!
//! Appendix A's preamble lists `U+0028` where real Japanese text carries `U+FF08`, so a
//! library that did not fold gave wrong classes on ordinary text, silently. The folding is
//! the Wide and Narrow compatibility decompositions and nothing else: full compatibility
//! folding would fold `U+2160`, a genuine cl-19 member, onto `I`.
//!
//! The literal key is looked up first and the folded one only when the literal is listed
//! nowhere. That order is load bearing rather than an optimization. `U+3000` has
//! `Decomposition_Type=Wide` onto `U+0020`; `U+3000` is cl-14 and `U+0020` is cl-26, so
//! folding first would silently reclassify the ideographic space as the Western word space.
//!
//! # This is not text segmentation
//!
//! The scan is the appendix's own key shape (ADR-0003). ICU4X grapheme clusters would fold
//! the fourteen kana pairs correctly and the tone-bar pairs incorrectly, so it cannot be
//! delegated to a segmenter.

use core::ops::Range;

use jlreq_unit::{ByteOffset, Frame};

use crate::generated::appendix_a::{FRAME_FULL_EM, LISTINGS, Listing, MAX_KEY_LEN};
use crate::generated::folding::FOLDS;
use crate::generated::ideograph::RANGES;

/// The key Appendix A is indexed by.
///
/// Twenty-five entries key on an *ordered pair* of code points, and cl-27 lists
/// `<02E5, 02E9>` and `<02E9, 02E5>` as two distinct members, so this is a sequence and
/// not a set. A `char`-keyed lookup cannot express Appendix A.
///
/// [`Member::MAX_LEN`] is generated from the table, with a compile-time assertion, so a
/// specification revision adding a three-code-point member is a build failure rather than
/// a silent truncation.
///
/// JLReq: §A
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Member {
    /// The code points, zero beyond `len`, so equality is the sequence's equality.
    key: [u32; MAX_KEY_LEN],
    /// How many of `key` are the key.
    len: u8,
}

impl Member {
    /// The longest key Appendix A enumerates, in code points.
    ///
    /// JLReq: §A
    pub const MAX_LEN: usize = MAX_KEY_LEN;

    /// The key of one code point. JLReq: §A
    #[must_use]
    pub const fn single(c: char) -> Self {
        let mut key = [0; MAX_KEY_LEN];
        key[0] = c as u32;
        Self { key, len: 1 }
    }

    /// The key of an ordered pair. JLReq: §A
    #[must_use]
    pub const fn pair(first: char, second: char) -> Self {
        let mut key = [0; MAX_KEY_LEN];
        key[0] = first as u32;
        key[1] = second as u32;
        Self { key, len: 2 }
    }

    /// How many code points this key holds. JLReq: §A
    #[must_use]
    pub const fn len(self) -> usize {
        self.len as usize
    }

    /// Whether this key holds no code point at all.
    ///
    /// It never does — both constructors take at least one — and the accessor exists
    /// because a length without one is a Clippy finding and because a caller reading
    /// [`Member::len`] should not have to know that.
    ///
    /// JLReq: §A
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// The code points, in the order Appendix A writes them. JLReq: §A
    pub fn code_points(self) -> impl Iterator<Item = char> {
        self.key
            .into_iter()
            .take(self.len())
            .filter_map(char::from_u32)
    }

    /// The key as the generated table stores one.
    const fn stored(self) -> [u32; MAX_KEY_LEN] {
        self.key
    }
}

/// Longest-match scan over Appendix A's key structure, yielding each member with its
/// byte range in the original text.
///
/// This is not text segmentation (ADR-0003): it is the appendix's own key shape, which no
/// other library knows. ICU4X grapheme clusters would fold the fourteen kana pairs
/// correctly and the tone-bar pairs incorrectly, so it cannot be delegated.
///
/// A code point Appendix A lists nowhere is yielded on its own, because the scan reports
/// what the text holds and [`crate::classify`] reports what the table says about it.
///
/// JLReq: §A
#[must_use]
pub fn members(text: &str) -> Members<'_> {
    Members { text, position: 0 }
}

/// The members of one text, in reading order. See [`members`].
///
/// JLReq: §A
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Members<'t> {
    /// The text being scanned.
    text: &'t str,
    /// Where the next member starts, in bytes of `text`.
    position: usize,
}

impl Iterator for Members<'_> {
    /// The byte range first, and typed: a caller compares this range against
    /// [`jlreq_unit::Item::start`], which is a [`ByteOffset`], so a raw `usize` here would
    /// be the untyped channel ADR 0011 narrows everywhere else.
    type Item = (Range<ByteOffset>, Member);

    fn next(&mut self) -> Option<Self::Item> {
        let rest = self.text.get(self.position..)?;
        let mut characters = rest.chars();
        let first = characters.next()?;
        let start = self.position;
        if let Some(second) = characters.next() {
            let paired = Member::pair(first, second);
            if is_key(paired) {
                let end = start
                    .saturating_add(first.len_utf8())
                    .saturating_add(second.len_utf8());
                self.position = end;
                return Some((offset(start)..offset(end), paired));
            }
        }
        let end = start.saturating_add(first.len_utf8());
        self.position = end;
        Some((offset(start)..offset(end), Member::single(first)))
    }
}

/// One byte position of the scanned text, as the item vocabulary spells one.
///
/// [`ByteOffset`] holds a `u32`, which is also what a `jlreq_unit::Item` holds, so a text
/// longer than four gibibytes has no items to be scanned against in the first place and the
/// saturation below is unreachable rather than approximate.
fn offset(position: usize) -> ByteOffset {
    ByteOffset::new(u32::try_from(position).unwrap_or(u32::MAX))
}

/// Fold a compatibility code point onto the code point Appendix A keys, reporting the
/// frame that code point itself asserts.
///
/// Appendix A's preamble states that it lists `U+0028` while real Japanese text uses
/// `U+FF08`, so a library that did not fold would give wrong classes on ordinary text,
/// silently. Only the Wide and Narrow decomposition mapping is used: full compatibility
/// folding would fold `U+2160`, a genuine cl-19 member, onto `I`.
///
/// A caller who passed `U+FF08` has thereby stated the frame is full-width; if they also
/// declared [`Frame::Proportional`] that is a diagnostic, not a silent choice.
///
/// JLReq: §A preamble
#[must_use]
pub fn fold_compatibility(c: char) -> Option<(char, Frame)> {
    let fold = FOLDS
        .binary_search_by_key(&(c as u32), |fold| fold.source)
        .ok()
        .and_then(|found| FOLDS.get(found))?;
    let target = char::from_u32(fold.target)?;
    let frame = if fold.frame == FRAME_FULL_EM {
        Frame::FullEm
    } else {
        Frame::HalfEm
    };
    Some((target, frame))
}

/// Every listing of one key, or the empty slice when Appendix A lists it nowhere.
///
/// The generated table is sorted by key and then by class, so one key's listings are one
/// contiguous run and the run is found by two binary searches over the same order.
pub(crate) fn listings(member: Member) -> &'static [Listing] {
    let key = member.stored();
    let first = LISTINGS.partition_point(|listing| listing.key < key);
    let rest = LISTINGS.get(first..).unwrap_or_default();
    let count = rest.partition_point(|listing| listing.key == key);
    rest.get(..count).unwrap_or_default()
}

/// Whether Appendix A lists this key, literally or through the compatibility folding.
pub(crate) fn is_key(member: Member) -> bool {
    !listings(member).is_empty()
        || folded(member).is_some_and(|folded| !listings(folded).is_empty())
}

/// The key `member` folds onto, when every code point of it folds and the result differs.
///
/// A pair folds only when both halves do, because a half-folded key is a sequence Appendix
/// A never wrote and looking one up would be asking the table a question about a text that
/// does not exist.
pub(crate) fn folded(member: Member) -> Option<Member> {
    let mut key = member.stored();
    let mut changed = false;
    let mut position = 0;
    while position < member.len() {
        let code_point = *key.get(position)?;
        let (target, _) = fold_compatibility(char::from_u32(code_point)?)?;
        if target as u32 != code_point {
            changed = true;
        }
        *key.get_mut(position)? = target as u32;
        position = position.saturating_add(1);
    }
    changed.then_some(Member {
        key,
        len: member.len,
    })
}

/// The frame a compatibility key asserts about itself, when every code point of it asserts
/// the same one.
///
/// A caller who wrote `U+FF08` has stated the frame is full-width whether or not they also
/// declared one, which is what makes a contradiction between the two reportable.
pub(crate) fn asserted_frame(member: Member) -> Option<Frame> {
    let mut asserted = None;
    for code_point in member.code_points() {
        let (_, frame) = fold_compatibility(code_point)?;
        match asserted {
            None => asserted = Some(frame),
            Some(previous) if previous == frame => {},
            Some(_) => return None,
        }
    }
    asserted
}

/// Whether the Unicode Character Database gives this code point `Unified_Ideograph=Yes`.
///
/// §A.19's table lists 465 rows where `Unified_Ideograph` covers 101 996 code points,
/// because the appendix enumerates the non-ideographic members of cl-19 — the Cyrillic and
/// Greek letters the class name does not describe — and leaves the ideographs themselves to
/// the character database. A lookup that read only the table would answer `Unlisted` for
/// almost every kanji in Japanese.
pub(crate) fn is_ideograph(c: char) -> bool {
    let code_point = c as u32;
    RANGES
        .binary_search_by(|range| {
            if range.last < code_point {
                core::cmp::Ordering::Less
            } else if range.first > code_point {
                core::cmp::Ordering::Greater
            } else {
                core::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// The one code point of a key of one, or `None` for a pair.
pub(crate) fn only_code_point(member: Member) -> Option<char> {
    if member.len() != 1 {
        return None;
    }
    member.code_points().next()
}

#[cfg(test)]
mod tests {
    use core::ops::Range;

    use jlreq_unit::{ByteOffset, Frame};

    use super::{
        Member, asserted_frame, fold_compatibility, folded, is_ideograph, is_key, listings, members,
    };

    /// One byte range of a scanned text, as the scan reports one.
    fn span(start: u32, end: u32) -> Range<ByteOffset> {
        ByteOffset::new(start)..ByteOffset::new(end)
    }

    #[test]
    fn a_key_holds_the_code_points_it_was_built_with_in_order() {
        let contour = Member::pair('\u{02E5}', '\u{02E9}');
        assert_eq!(contour.len(), 2);
        assert!(contour.code_points().eq(['\u{02E5}', '\u{02E9}']));
        assert_ne!(
            contour,
            Member::pair('\u{02E9}', '\u{02E5}'),
            "cl-27 lists both orders as two distinct members, so the key is a sequence"
        );
    }

    #[test]
    fn a_key_of_one_code_point_is_not_a_key_of_two_padded() {
        assert_ne!(
            Member::single('a'),
            Member::pair('a', '\u{0}'),
            "the padding is not part of the key, and equality is the sequence's equality"
        );
        assert!(!Member::single('a').is_empty());
    }

    #[test]
    fn the_longest_key_is_what_the_generated_table_holds() {
        assert_eq!(
            Member::MAX_LEN,
            2,
            "a revision adding a three-code-point member is a build failure rather than a \
             silent truncation"
        );
    }

    #[test]
    fn the_scan_prefers_the_pair_appendix_a_lists() {
        assert!(
            members("\u{02E5}\u{02E9}").eq([(span(0, 4), Member::pair('\u{02E5}', '\u{02E9}'))]),
            "§A.27 lists the falling tone contour as one key, so the scan is longest-first"
        );
    }

    #[test]
    fn the_scan_falls_back_to_one_code_point_where_no_pair_is_listed() {
        assert!(
            members("\u{02E5}a").eq([
                (span(0, 2), Member::single('\u{02E5}')),
                (span(2, 3), Member::single('a')),
            ]),
            "the first code point of the contour is also listed alone"
        );
    }

    #[test]
    fn the_scan_reports_byte_ranges_of_the_original_text() {
        assert!(
            members("あい").eq([
                (span(0, 3), Member::single('あ')),
                (span(3, 6), Member::single('い'))
            ]),
            "a range indexes the caller's own string, so a slice of it cannot panic"
        );
    }

    #[test]
    fn the_scan_yields_a_code_point_appendix_a_lists_nowhere() {
        assert!(
            members("\u{1F600}").eq([(span(0, 4), Member::single('\u{1F600}'))]),
            "the scan reports what the text holds; the table says what class it is"
        );
    }

    #[test]
    fn an_empty_text_holds_no_member() {
        assert_eq!(members("").count(), 0);
    }

    #[test]
    fn the_small_kana_with_a_combining_mark_is_one_key() {
        assert!(
            members("\u{31F7}\u{309A}").eq([(span(0, 6), Member::pair('\u{31F7}', '\u{309A}'))]),
            "§A.11 lists the pair, and its second code point is listed nowhere alone, so \
             splitting it would answer cl-11 followed by an unlisted reading (ADR-0018)"
        );
    }

    #[test]
    fn a_full_width_bracket_folds_onto_the_one_appendix_a_lists() {
        assert_eq!(
            fold_compatibility('（'),
            Some(('(', Frame::FullEm)),
            "§A's preamble lists U+0028 where real Japanese text carries U+FF08"
        );
    }

    #[test]
    fn a_half_width_form_folds_and_says_so() {
        assert_eq!(
            fold_compatibility('\u{FF61}'),
            Some(('。', Frame::HalfEm)),
            "the half-width ideographic full stop asserts the half-em frame by being itself"
        );
    }

    #[test]
    fn a_roman_numeral_does_not_fold() {
        assert_eq!(
            fold_compatibility('\u{2160}'),
            None,
            "full compatibility folding would fold U+2160, a genuine cl-19 member, onto I"
        );
    }

    #[test]
    fn the_ideographic_space_is_listed_and_therefore_never_folded() {
        assert!(
            !listings(Member::single('\u{3000}')).is_empty(),
            "U+3000 is cl-14, so the literal lookup answers and the fold is never reached"
        );
        assert_eq!(
            fold_compatibility('\u{3000}'),
            Some((' ', Frame::FullEm)),
            "the fold exists, which is exactly why the literal key has to be tried first: \
             U+3000 is cl-14 and U+0020 is cl-26"
        );
    }

    #[test]
    fn a_pair_folds_only_when_both_halves_do() {
        assert_eq!(
            folded(Member::pair('（', '\u{02E9}')),
            None,
            "a half-folded key is a sequence Appendix A never wrote"
        );
    }

    #[test]
    fn a_compatibility_key_asserts_its_own_frame() {
        assert_eq!(asserted_frame(Member::single('（')), Some(Frame::FullEm));
        assert_eq!(
            asserted_frame(Member::single('(')),
            None,
            "an ordinary code point asserts nothing about the frame; the caller states it"
        );
    }

    #[test]
    fn a_key_appendix_a_reaches_only_by_folding_is_still_a_key() {
        assert!(
            listings(Member::single('（')).is_empty(),
            "§A lists U+0028 and not U+FF08"
        );
        assert!(
            is_key(Member::single('（')),
            "and the folding is what makes ordinary Japanese text classifiable"
        );
    }

    #[test]
    fn the_listings_of_one_key_are_one_contiguous_run() {
        let opening = listings(Member::single('('));
        assert!(
            opening.len() > 1,
            "U+0028 is named by cl-01, cl-25, cl-27 and cl-28, which is the measurement \
             `docs/adr/0008` turns on"
        );
        assert!(
            opening.windows(2).all(|pair| pair[0].class < pair[1].class),
            "one key's listings are sorted by class, so a candidate set is deterministic"
        );
    }

    #[test]
    fn a_key_appendix_a_never_wrote_has_no_listing() {
        assert!(listings(Member::single('\u{1F600}')).is_empty());
    }

    #[test]
    fn the_ideograph_predicate_answers_for_the_kanji_the_table_does_not_list() {
        assert!(
            is_ideograph('漢'),
            "§A.19 lists 465 rows and Unified_Ideograph covers 101 996 code points"
        );
        assert!(
            !is_ideograph('\u{3005}'),
            "JLReq puts the ideographic iteration mark in cl-09, and the character database \
             does not call it an ideograph either"
        );
        assert!(!is_ideograph('a'));
    }
}
