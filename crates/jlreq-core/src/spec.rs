// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Private, generated specification facts used by the composition pipeline.
//!
//! This module deliberately exposes no public classification vocabulary. The generated
//! tables retain their JLReq and Unicode provenance while the public API deals only in
//! shaped clusters, constructs, placements, and stable diagnostics.

use crate::generated::appendix_a::{LISTINGS, MAX_KEY_LEN};
use crate::generated::folding::FOLDS;
use crate::generated::ideograph::RANGES as IDEOGRAPH_RANGES;
use crate::generated::script::{HIRAGANA, KATAKANA, RANGES as SCRIPT_RANGES};
use crate::model::{ClusterRole, Frame, Size, WritingMode};

pub(crate) const UNITS_PER_EM: i32 = 720;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RawHang {
    None,
    OverSpace,
    OverCharacter,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RawTerm {
    pub(crate) trailing: bool,
    pub(crate) amount: i32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RawSpacingCell {
    pub(crate) prohibited: bool,
    pub(crate) hang: RawHang,
    pub(crate) rule: &'static str,
    pub(crate) terms: &'static [RawTerm],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RawBreakCell {
    pub(crate) prohibited: bool,
    pub(crate) levels: u8,
    pub(crate) rule: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RawRangedCell {
    pub(crate) limit: Option<i32>,
    pub(crate) two_valued: bool,
    pub(crate) residual: bool,
    pub(crate) stage: u8,
    pub(crate) rule: &'static str,
}

pub(crate) const fn em(units: i32) -> i32 {
    units
}

pub(crate) const OPENING_BRACKET: u8 = 1;
pub(crate) const CLOSING_BRACKET: u8 = 2;
pub(crate) const MIDDLE_DOT: u8 = 5;
pub(crate) const FULL_STOP: u8 = 6;
pub(crate) const COMMA: u8 = 7;
pub(crate) const INSEPARABLE: u8 = 8;
pub(crate) const MATH_SYMBOL: u8 = 17;
pub(crate) const MATH_OPERATOR: u8 = 18;
const IDEOGRAPH: u8 = 19;
const CONSTRUCT_CLASSES: u32 = class_bit(20)
    .saturating_add(class_bit(21))
    .saturating_add(class_bit(22))
    .saturating_add(class_bit(23))
    .saturating_add(class_bit(24))
    .saturating_add(class_bit(25))
    .saturating_add(class_bit(28))
    .saturating_add(class_bit(29))
    .saturating_add(class_bit(30));

pub(crate) fn class_of(
    piece: &str,
    frame: Frame,
    role: Option<ClusterRole>,
    writing_mode: WritingMode,
    unlisted_is_ideographic: bool,
    highest_ambiguous_class: bool,
    grouped_numeral_requires_role: bool,
) -> u8 {
    let mut characters = piece.chars();
    let Some(first) = characters.next() else {
        return IDEOGRAPH;
    };
    let second = characters.next();
    if characters.next().is_some() {
        return if frame == Frame::Proportional { 27 } else { 19 };
    }
    let key = [first as u32, second.map_or(0, |character| character as u32)];
    let mut candidates = candidates(key);
    if candidates == 0 {
        return if unlisted_is_ideographic || frame != Frame::Proportional {
            19
        } else {
            27
        };
    }
    candidates = narrow_by_usage(candidates, key, writing_mode);
    candidates = narrow_by_frame(candidates, key, frame);
    candidates = narrow_by_role(candidates, role, first, grouped_numeral_requires_role);
    let select = |classes| {
        if highest_ambiguous_class {
            last_class(classes)
        } else {
            first_class(classes)
        }
    };
    select(candidates & !CONSTRUCT_CLASSES)
        .or_else(|| select(candidates))
        .unwrap_or(IDEOGRAPH)
}

pub(crate) fn table_one_space(
    before: u8,
    after: u8,
    before_size: Size,
    after_size: Size,
    before_solid: bool,
    after_solid: bool,
) -> i32 {
    table_one_space_components(
        before,
        after,
        before_size,
        after_size,
        before_solid,
        after_solid,
    )
    .into_iter()
    .fold(0_i32, i32::saturating_add)
}

pub(crate) fn table_one_space_components(
    before: u8,
    after: u8,
    before_size: Size,
    after_size: Size,
    before_solid: bool,
    after_solid: bool,
) -> [i32; 2] {
    let Some(cell) = crate::generated::table1::cell(before, after) else {
        return [0, 0];
    };
    let mut components = [0_i32; 2];
    for term in cell.terms {
        if (term.trailing && after_solid) || (!term.trailing && before_solid) {
            continue;
        }
        let size = if term.trailing {
            after_size.inline()
        } else {
            before_size.inline()
        };
        let slot = usize::from(term.trailing);
        components[slot] = components[slot].saturating_add(scale_spec_units(size, term.amount));
    }
    components
}

pub(crate) fn table_two_cell(before: u8, after: u8) -> Option<RawBreakCell> {
    crate::generated::table2::cell(before, after).copied()
}

const fn class_bit(class: u8) -> u32 {
    1_u32 << class.saturating_sub(1)
}

fn insert_class(set: u32, class: u8) -> u32 {
    let bit = class_bit(class);
    if set & bit == 0 {
        set.saturating_add(bit)
    } else {
        set
    }
}

fn candidates(key: [u32; MAX_KEY_LEN]) -> u32 {
    let literal = listings(key);
    let selected = if literal.is_empty() && key[1] == 0 {
        char::from_u32(key[0])
            .and_then(fold)
            .map_or(literal, |folded| listings([folded as u32, 0]))
    } else {
        literal
    };
    let mut classes = selected
        .iter()
        .fold(0_u32, |set, listing| insert_class(set, listing.class));
    if key[1] == 0
        && char::from_u32(key[0]).is_some_and(is_ideograph)
        && classes & class_bit(IDEOGRAPH) == 0
    {
        classes = insert_class(classes, IDEOGRAPH);
    }
    classes
}

fn narrow_by_usage(classes: u32, key: [u32; MAX_KEY_LEN], mode: WritingMode) -> u32 {
    let narrowed = listings_for_candidate(key)
        .iter()
        .fold(0_u32, |set, listing| {
            let usage = crate::generated::appendix_a::REMARKS[usize::from(listing.remark)].usage;
            let permitted = usage == crate::generated::appendix_a::USAGE_UNQUALIFIED
                || (usage == crate::generated::appendix_a::USAGE_HORIZONTAL_ONLY
                    && mode == WritingMode::HorizontalTb)
                || (usage == crate::generated::appendix_a::USAGE_VERTICAL_ONLY
                    && mode == WritingMode::VerticalRl);
            if permitted {
                insert_class(set, listing.class)
            } else {
                set
            }
        });
    keep(classes, classes & narrowed)
}

fn narrow_by_frame(classes: u32, key: [u32; MAX_KEY_LEN], frame: Frame) -> u32 {
    let frame_bit = match frame {
        Frame::FullEm => crate::generated::appendix_a::FRAME_FULL_EM,
        Frame::HalfEm => crate::generated::appendix_a::FRAME_HALF_EM,
        Frame::Proportional => crate::generated::appendix_a::FRAME_PROPORTIONAL,
    };
    let permitted = listings_for_candidate(key)
        .iter()
        .fold(0_u32, |set, listing| {
            let frames = crate::generated::appendix_a::REMARKS[usize::from(listing.remark)].frames;
            if frames == crate::generated::appendix_a::FRAMES_UNSTATED || frames & frame_bit != 0 {
                insert_class(set, listing.class)
            } else {
                set
            }
        });
    let mut narrowed = keep(classes, classes & permitted);
    let explicitly_stated = listings_for_candidate(key)
        .iter()
        .fold(0_u32, |set, listing| {
            let frames = crate::generated::appendix_a::REMARKS[usize::from(listing.remark)].frames;
            let stated_by_advance = matches!(listing.class, 1 | 2 | 5 | 6 | 7)
                && matches!(frame, Frame::FullEm | Frame::HalfEm);
            if (1..20).contains(&listing.class) && (frames & frame_bit != 0 || stated_by_advance) {
                insert_class(set, listing.class)
            } else {
                set
            }
        });
    if explicitly_stated != 0 {
        narrowed = keep(
            narrowed,
            narrowed & explicitly_stated.saturating_add(CONSTRUCT_CLASSES),
        );
    }
    if frame == Frame::Proportional {
        let half_advance_classes = class_bit(1)
            .saturating_add(class_bit(2))
            .saturating_add(class_bit(5))
            .saturating_add(class_bit(6))
            .saturating_add(class_bit(7));
        let without_half_advance = narrowed & !half_advance_classes;
        narrowed = keep(narrowed, without_half_advance);
    }
    if narrowed & class_bit(IDEOGRAPH) != 0 && narrowed & class_bit(27) != 0 {
        narrowed = match frame {
            Frame::Proportional => narrowed & !class_bit(IDEOGRAPH),
            Frame::FullEm => narrowed & !class_bit(27),
            Frame::HalfEm if narrowed & class_bit(24) != 0 => narrowed & !class_bit(IDEOGRAPH),
            _ => narrowed,
        };
    }
    narrowed
}

fn narrow_by_role(
    classes: u32,
    role: Option<ClusterRole>,
    character: char,
    grouped_numeral_requires_role: bool,
) -> u32 {
    let selected = match role {
        Some(
            ClusterRole::DecimalPoint
            | ClusterRole::DigitGroupSeparator
            | ClusterRole::GroupedNumeral,
        ) => class_bit(24),
        Some(ClusterRole::SentenceMedial | ClusterRole::SentenceTerminator) => class_bit(4),
        Some(ClusterRole::UnitSymbol) => class_bit(25),
        Some(ClusterRole::WarichuBracket) if single_has_class(character, OPENING_BRACKET) => {
            class_bit(28)
        },
        Some(ClusterRole::WarichuBracket) if single_has_class(character, CLOSING_BRACKET) => {
            class_bit(29)
        },
        _ if grouped_numeral_requires_role && classes & class_bit(24) != 0 => {
            return class_bit(27);
        },
        _ => return classes,
    };
    keep(classes, classes & selected)
}

fn listings_for_candidate(
    key: [u32; MAX_KEY_LEN],
) -> &'static [crate::generated::appendix_a::Listing] {
    let literal = listings(key);
    if !literal.is_empty() || key[1] != 0 {
        return literal;
    }
    char::from_u32(key[0])
        .and_then(fold)
        .map_or(literal, |folded| listings([folded as u32, 0]))
}

const fn keep(original: u32, narrowed: u32) -> u32 {
    if narrowed == 0 { original } else { narrowed }
}

fn first_class(classes: u32) -> Option<u8> {
    (1_u8..=30).find(|class| classes & class_bit(*class) != 0)
}

fn last_class(classes: u32) -> Option<u8> {
    (1_u8..=30)
        .rev()
        .find(|class| classes & class_bit(*class) != 0)
}

pub(crate) fn scale_spec_units(size: i32, units: i32) -> i32 {
    let product = i64::from(size).saturating_mul(i64::from(units));
    let denominator = i64::from(UNITS_PER_EM);
    let whole = product.checked_div(denominator).unwrap_or(product);
    let has_remainder = product
        .checked_rem(denominator)
        .is_some_and(|remainder| remainder != 0);
    let rounded = whole.saturating_add(i64::from(has_remainder));
    i32::try_from(rounded).unwrap_or(i32::MAX)
}

/// Whether Appendix A names the single-code-point key under `class`.
///
/// Literal membership wins over compatibility folding. This is load-bearing for U+3000:
/// it is literally cl-14 and must not first fold to the cl-26 U+0020.
pub(crate) fn single_has_class(character: char, class: u8) -> bool {
    if class == IDEOGRAPH && is_ideograph(character) {
        return true;
    }
    let literal = [character as u32, 0];
    let literal_listings = listings(literal);
    if !literal_listings.is_empty() {
        return literal_listings
            .iter()
            .any(|listing| listing.class == class);
    }
    let Some(folded) = fold(character) else {
        return false;
    };
    listings([folded as u32, 0])
        .iter()
        .any(|listing| listing.class == class)
}

/// Whether two code points form one of Appendix A's indivisible keys.
pub(crate) fn is_pair(first: char, second: char) -> bool {
    !listings([first as u32, second as u32]).is_empty()
}

pub(crate) fn is_hiragana(character: char) -> bool {
    script(character) == Some(HIRAGANA)
}

pub(crate) fn is_katakana(character: char) -> bool {
    script(character) == Some(KATAKANA)
}

fn is_ideograph(character: char) -> bool {
    let code_point = character as u32;
    IDEOGRAPH_RANGES
        .binary_search_by(|range| {
            if code_point < range.first {
                core::cmp::Ordering::Greater
            } else if code_point > range.last {
                core::cmp::Ordering::Less
            } else {
                core::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

fn listings(key: [u32; MAX_KEY_LEN]) -> &'static [crate::generated::appendix_a::Listing] {
    let first = LISTINGS.partition_point(|listing| listing.key < key);
    let rest = LISTINGS.get(first..).unwrap_or_default();
    let count = rest.partition_point(|listing| listing.key == key);
    rest.get(..count).unwrap_or_default()
}

fn fold(character: char) -> Option<char> {
    let fold = FOLDS
        .binary_search_by_key(&(character as u32), |fold| fold.source)
        .ok()
        .and_then(|found| FOLDS.get(found))?;
    char::from_u32(fold.target)
}

fn script(character: char) -> Option<u8> {
    let code_point = character as u32;
    SCRIPT_RANGES
        .binary_search_by(|range| {
            if code_point < range.first {
                core::cmp::Ordering::Greater
            } else if code_point > range.last {
                core::cmp::Ordering::Less
            } else {
                core::cmp::Ordering::Equal
            }
        })
        .ok()
        .and_then(|found| SCRIPT_RANGES.get(found))
        .map(|range| range.script)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{string::String, vec::Vec};

    const ALL_CLASSES: u32 = (1_u32 << 30) - 1;
    const REFERENCE_CONSTRUCT_CLASSES: u32 = ref_bit(20)
        | ref_bit(21)
        | ref_bit(22)
        | ref_bit(23)
        | ref_bit(24)
        | ref_bit(25)
        | ref_bit(28)
        | ref_bit(29)
        | ref_bit(30);

    const fn ref_bit(class: u8) -> u32 {
        1_u32 << class.saturating_sub(1)
    }

    fn ref_literal(key: [u32; MAX_KEY_LEN]) -> Vec<&'static crate::generated::appendix_a::Listing> {
        LISTINGS
            .iter()
            .filter(|listing| listing.key == key)
            .collect()
    }

    fn ref_fold(character: char) -> Option<char> {
        FOLDS
            .iter()
            .find(|fold| fold.source == character as u32)
            .and_then(|fold| char::from_u32(fold.target))
    }

    fn ref_candidate_listings(
        key: [u32; MAX_KEY_LEN],
    ) -> Vec<&'static crate::generated::appendix_a::Listing> {
        let literal = ref_literal(key);
        if !literal.is_empty() || key[1] != 0 {
            return literal;
        }
        char::from_u32(key[0])
            .and_then(ref_fold)
            .map_or(literal, |folded| ref_literal([folded as u32, 0]))
    }

    fn ref_is_ideograph(character: char) -> bool {
        let code_point = character as u32;
        IDEOGRAPH_RANGES
            .iter()
            .any(|range| range.first <= code_point && code_point <= range.last)
    }

    fn ref_candidates(key: [u32; MAX_KEY_LEN]) -> u32 {
        let mut classes = ref_candidate_listings(key)
            .iter()
            .fold(0, |set, listing| set | ref_bit(listing.class));
        if key[1] == 0
            && char::from_u32(key[0]).is_some_and(ref_is_ideograph)
            && classes & ref_bit(IDEOGRAPH) == 0
        {
            classes |= ref_bit(IDEOGRAPH);
        }
        classes
    }

    const fn ref_keep(original: u32, narrowed: u32) -> u32 {
        if narrowed == 0 { original } else { narrowed }
    }

    fn ref_narrow_by_usage(classes: u32, key: [u32; MAX_KEY_LEN], mode: WritingMode) -> u32 {
        let narrowed = ref_candidate_listings(key).iter().fold(0, |set, listing| {
            let usage = crate::generated::appendix_a::REMARKS[usize::from(listing.remark)].usage;
            let permitted = usage == crate::generated::appendix_a::USAGE_UNQUALIFIED
                || (usage == crate::generated::appendix_a::USAGE_HORIZONTAL_ONLY
                    && mode == WritingMode::HorizontalTb)
                || (usage == crate::generated::appendix_a::USAGE_VERTICAL_ONLY
                    && mode == WritingMode::VerticalRl);
            if permitted {
                set | ref_bit(listing.class)
            } else {
                set
            }
        });
        ref_keep(classes, classes & narrowed)
    }

    fn ref_narrow_by_frame(classes: u32, key: [u32; MAX_KEY_LEN], frame: Frame) -> u32 {
        let frame_bit = match frame {
            Frame::FullEm => crate::generated::appendix_a::FRAME_FULL_EM,
            Frame::HalfEm => crate::generated::appendix_a::FRAME_HALF_EM,
            Frame::Proportional => crate::generated::appendix_a::FRAME_PROPORTIONAL,
        };
        let listings = ref_candidate_listings(key);
        let permitted = listings.iter().fold(0, |set, listing| {
            let frames = crate::generated::appendix_a::REMARKS[usize::from(listing.remark)].frames;
            if frames == crate::generated::appendix_a::FRAMES_UNSTATED || frames & frame_bit != 0 {
                set | ref_bit(listing.class)
            } else {
                set
            }
        });
        let mut narrowed = ref_keep(classes, classes & permitted);
        let explicitly_stated = listings.iter().fold(0, |set, listing| {
            let frames = crate::generated::appendix_a::REMARKS[usize::from(listing.remark)].frames;
            let stated_by_advance = matches!(listing.class, 1 | 2 | 5 | 6 | 7)
                && matches!(frame, Frame::FullEm | Frame::HalfEm);
            if listing.class < 20 && (frames & frame_bit != 0 || stated_by_advance) {
                set | ref_bit(listing.class)
            } else {
                set
            }
        });
        if explicitly_stated != 0 {
            narrowed = ref_keep(
                narrowed,
                narrowed & (explicitly_stated | REFERENCE_CONSTRUCT_CLASSES),
            );
        }
        if frame == Frame::Proportional {
            let without_half_advance =
                narrowed & !(ref_bit(1) | ref_bit(2) | ref_bit(5) | ref_bit(6) | ref_bit(7));
            narrowed = ref_keep(narrowed, without_half_advance);
        }
        if narrowed & ref_bit(IDEOGRAPH) != 0 && narrowed & ref_bit(27) != 0 {
            narrowed = match frame {
                Frame::Proportional => narrowed & !ref_bit(IDEOGRAPH),
                Frame::FullEm => narrowed & !ref_bit(27),
                Frame::HalfEm if narrowed & ref_bit(24) != 0 => narrowed & !ref_bit(IDEOGRAPH),
                _ => narrowed,
            };
        }
        narrowed
    }

    fn ref_narrow_by_role(
        classes: u32,
        role: Option<ClusterRole>,
        character: char,
        grouped_numeral_requires_role: bool,
    ) -> u32 {
        let selected = match role {
            Some(
                ClusterRole::DecimalPoint
                | ClusterRole::DigitGroupSeparator
                | ClusterRole::GroupedNumeral,
            ) => ref_bit(24),
            Some(ClusterRole::SentenceMedial | ClusterRole::SentenceTerminator) => ref_bit(4),
            Some(ClusterRole::UnitSymbol) => ref_bit(25),
            Some(ClusterRole::WarichuBracket) if single_has_class(character, OPENING_BRACKET) => {
                ref_bit(28)
            },
            Some(ClusterRole::WarichuBracket) if single_has_class(character, CLOSING_BRACKET) => {
                ref_bit(29)
            },
            _ if grouped_numeral_requires_role && classes & ref_bit(24) != 0 => {
                return ref_bit(27);
            },
            _ => return classes,
        };
        ref_keep(classes, classes & selected)
    }

    fn ref_first(classes: u32) -> Option<u8> {
        (1..=30).find(|class| classes & ref_bit(*class) != 0)
    }

    fn ref_last(classes: u32) -> Option<u8> {
        (1..=30).rev().find(|class| classes & ref_bit(*class) != 0)
    }

    fn ref_class_of(
        piece: &str,
        frame: Frame,
        role: Option<ClusterRole>,
        mode: WritingMode,
        unlisted_is_ideographic: bool,
        highest: bool,
        grouped_requires_role: bool,
    ) -> u8 {
        let mut characters = piece.chars();
        let Some(first) = characters.next() else {
            return IDEOGRAPH;
        };
        let second = characters.next();
        if characters.next().is_some() {
            return if frame == Frame::Proportional { 27 } else { 19 };
        }
        let key = [first as u32, second.map_or(0, |character| character as u32)];
        let mut classes = ref_candidates(key);
        if classes == 0 {
            return if unlisted_is_ideographic || frame != Frame::Proportional {
                19
            } else {
                27
            };
        }
        classes = ref_narrow_by_usage(classes, key, mode);
        classes = ref_narrow_by_frame(classes, key, frame);
        classes = ref_narrow_by_role(classes, role, first, grouped_requires_role);
        let select = |set| {
            if highest {
                ref_last(set)
            } else {
                ref_first(set)
            }
        };
        select(classes & !REFERENCE_CONSTRUCT_CLASSES)
            .or_else(|| select(classes))
            .unwrap_or(IDEOGRAPH)
    }

    #[test]
    fn literal_membership_precedes_wide_folding() {
        assert!(!single_has_class('\u{3000}', 26));
        assert!(single_has_class('\u{ff08}', OPENING_BRACKET));
    }

    #[test]
    fn generated_pair_and_unicode_ranges_are_queryable() {
        assert!(is_pair('\u{02e5}', '\u{02e9}'));
        assert!(is_hiragana('あ'));
        assert!(is_katakana('ア'));
        assert!(is_ideograph('𠀀'));
    }

    #[test]
    fn classification_policy_switches_are_applied_after_generated_membership() {
        assert_eq!(
            class_of(
                "1",
                Frame::HalfEm,
                None,
                WritingMode::HorizontalTb,
                false,
                false,
                true,
            ),
            27
        );
        assert_eq!(
            class_of(
                "↔",
                Frame::FullEm,
                None,
                WritingMode::HorizontalTb,
                false,
                true,
                false,
            ),
            19
        );
        assert_eq!(
            class_of(
                "🦀",
                Frame::Proportional,
                None,
                WritingMode::HorizontalTb,
                true,
                false,
                false,
            ),
            19
        );
    }

    #[test]
    fn handwritten_candidate_narrowing_matches_an_independent_table_oracle() {
        let frames = [Frame::FullEm, Frame::HalfEm, Frame::Proportional];
        let modes = [WritingMode::HorizontalTb, WritingMode::VerticalRl];
        let mut previous = None;
        for listing in LISTINGS {
            let key = listing.key;
            if previous == Some(key) {
                continue;
            }
            previous = Some(key);
            assert_eq!(
                candidates(key),
                ref_candidates(key),
                "candidates for {key:?}"
            );
            let actual = listings_for_candidate(key)
                .iter()
                .map(|listing| (listing.key, listing.class, listing.remark))
                .collect::<Vec<_>>();
            let expected = ref_candidate_listings(key)
                .iter()
                .map(|listing| (listing.key, listing.class, listing.remark))
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "listings for {key:?}");
            for mode in modes {
                assert_eq!(
                    narrow_by_usage(ALL_CLASSES, key, mode),
                    ref_narrow_by_usage(ALL_CLASSES, key, mode),
                    "usage for {key:?} {mode:?}"
                );
            }
            for frame in frames {
                assert_eq!(
                    narrow_by_frame(ALL_CLASSES, key, frame),
                    ref_narrow_by_frame(ALL_CLASSES, key, frame),
                    "frame for {key:?} {frame:?}"
                );
            }

            let mut piece = String::new();
            piece.push(char::from_u32(key[0]).expect("generated scalar"));
            if let Some(second) = char::from_u32(key[1]).filter(|_| key[1] != 0) {
                piece.push(second);
            }
            for frame in frames {
                for mode in modes {
                    for highest in [false, true] {
                        for grouped in [false, true] {
                            assert_eq!(
                                class_of(&piece, frame, None, mode, false, highest, grouped),
                                ref_class_of(&piece, frame, None, mode, false, highest, grouped),
                                "class for {key:?} {frame:?} {mode:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn role_narrowing_and_unlisted_shapes_match_the_oracle() {
        assert_eq!(CONSTRUCT_CLASSES, REFERENCE_CONSTRUCT_CLASSES);
        let ideograph_key = ['𠀀' as u32, 0];
        assert_eq!(candidates(ideograph_key), ref_bit(IDEOGRAPH));

        let synthetic = ['🦀' as u32, 0];
        let ideograph_or_western = ref_bit(IDEOGRAPH) | ref_bit(27);
        assert_eq!(
            narrow_by_frame(ideograph_or_western, synthetic, Frame::HalfEm),
            ideograph_or_western,
            "half-em without a cl-24 candidate keeps an otherwise ambiguous mask"
        );

        let unknown_pair = ['（' as u32, 'x' as u32];
        assert!(listings_for_candidate(unknown_pair).is_empty());

        let roles = [
            None,
            Some(ClusterRole::DecimalPoint),
            Some(ClusterRole::DigitGroupSeparator),
            Some(ClusterRole::SentenceMedial),
            Some(ClusterRole::SentenceTerminator),
            Some(ClusterRole::GroupedNumeral),
            Some(ClusterRole::UnitSymbol),
            Some(ClusterRole::QuantitySymbol),
            Some(ClusterRole::Formula),
            Some(ClusterRole::WarichuBracket),
        ];
        for character in ['（', '）', '.', '!', '1', 'A'] {
            for role in roles {
                for grouped in [false, true] {
                    assert_eq!(
                        narrow_by_role(ALL_CLASSES, role, character, grouped),
                        ref_narrow_by_role(ALL_CLASSES, role, character, grouped),
                        "role {role:?} for {character:?}"
                    );
                }
            }
        }
        for (piece, frame) in [
            ("", Frame::FullEm),
            ("🦀", Frame::FullEm),
            ("🦀", Frame::Proportional),
            ("abc", Frame::FullEm),
            ("abc", Frame::Proportional),
        ] {
            for unlisted in [false, true] {
                assert_eq!(
                    class_of(
                        piece,
                        frame,
                        None,
                        WritingMode::HorizontalTb,
                        unlisted,
                        false,
                        false
                    ),
                    ref_class_of(
                        piece,
                        frame,
                        None,
                        WritingMode::HorizontalTb,
                        unlisted,
                        false,
                        false
                    )
                );
            }
        }
    }

    #[test]
    fn unicode_range_searches_include_both_endpoints_only() {
        for range in IDEOGRAPH_RANGES {
            for code_point in [range.first, range.last] {
                let character = char::from_u32(code_point).expect("ideograph scalar");
                assert!(is_ideograph(character));
            }
            if let Some(character) = range
                .first
                .checked_sub(1)
                .and_then(char::from_u32)
                .filter(|character| !ref_is_ideograph(*character))
            {
                assert!(!is_ideograph(character));
            }
            if let Some(character) = range
                .last
                .checked_add(1)
                .and_then(char::from_u32)
                .filter(|character| !ref_is_ideograph(*character))
            {
                assert!(!is_ideograph(character));
            }
        }
        for range in SCRIPT_RANGES {
            for code_point in [range.first, range.last] {
                let character = char::from_u32(code_point).expect("script scalar");
                assert_eq!(script(character), Some(range.script));
            }
            if let Some(character) = range.first.checked_sub(1).and_then(char::from_u32) {
                let expected = SCRIPT_RANGES
                    .iter()
                    .find(|candidate| {
                        candidate.first <= character as u32 && character as u32 <= candidate.last
                    })
                    .map(|candidate| candidate.script);
                assert_eq!(script(character), expected);
            }
            if let Some(character) = range.last.checked_add(1).and_then(char::from_u32) {
                let expected = SCRIPT_RANGES
                    .iter()
                    .find(|candidate| {
                        candidate.first <= character as u32 && character as u32 <= candidate.last
                    })
                    .map(|candidate| candidate.script);
                assert_eq!(script(character), expected);
            }
        }
    }
}
