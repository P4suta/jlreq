// SPDX-FileCopyrightText: 2026 kumihan contributors
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

pub(crate) const OPENING_BRACKET: u8 = 1;
pub(crate) const CLOSING_BRACKET: u8 = 2;
pub(crate) const DIVIDING_PUNCTUATION: u8 = 4;
pub(crate) const MIDDLE_DOT: u8 = 5;
pub(crate) const FULL_STOP: u8 = 6;
pub(crate) const COMMA: u8 = 7;
pub(crate) const INSEPARABLE: u8 = 8;
pub(crate) const MATH_SYMBOL: u8 = 17;
pub(crate) const MATH_OPERATOR: u8 = 18;
const IDEOGRAPH: u8 = 19;

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
}
