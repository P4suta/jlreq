// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Which writing direction a member is used in, which is a validity fact and never a class.
//!
//! Twelve of Appendix A's rows carry a writing-direction Remark: seven read "used in
//! horizontal composition" and five read "used in vertical composition". For every one of
//! those twelve the class ambiguity is resolved by frame or by role, so the direction never
//! selects a class — which is what lets this crate answer a question about the writing
//! direction without any part of classification reading one (`docs/adr/0011`).
//!
//! # The Remarks column does not state all of it
//!
//! Four of the seven horizontal rows are the quotation marks `‘ ’ “ ”`, and §3.1.1 says of
//! them that "in vertical writing mode, when Western characters (cl-27) are composed rotated
//! 90 degrees clockwise, these quotation marks are sometimes used". So their restriction is
//! conditional on a rotation policy the caller owns, and reading the Remarks cell alone would
//! publish a prohibition §3.1.1 does not state. That refinement is a rule and not a row, so
//! it is applied here, over the generated table, with the four code points §3.1.1 names.

use crate::generated::appendix_a::REMARKS;
use crate::generated::appendix_a::{USAGE_HORIZONTAL_ONLY, USAGE_UNQUALIFIED, USAGE_VERTICAL_ONLY};
use crate::member::{Member, folded, listings};

/// The four quotation marks §3.1.1 permits in vertical writing when Western characters are
/// composed rotated 90 degrees clockwise.
///
/// `‘` and `“` are cl-01; `’` and `”` are cl-02. Appendix A marks all four "used in
/// horizontal composition" exactly as it marks `;`, `.` and `,`, and §3.1.1 is what
/// separates the two groups: the three punctuation marks are the horizontal conventions it
/// enumerates for full stops (cl-06) and commas (cl-07), and the four quotation marks are
/// the ones it says are "sometimes used" in vertical writing.
///
/// JLReq: §3.1.1
const ROTATED_WESTERN: [char; 4] = ['\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}'];

/// Whether the writing system uses this member in one direction only.
///
/// A validity fact, never a class selector: for all twelve code points carrying a
/// writing-mode Remark the class ambiguity is resolved by frame or role, so the direction
/// never selects a class. `jlreq::diagnose` checks it; composition does not.
///
/// JLReq: §3.1.1, §A Remarks
#[must_use]
pub fn usage(member: Member) -> Usage {
    let literal = listings(member);
    let table = if literal.is_empty() {
        folded(member).map_or(&[][..], listings)
    } else {
        literal
    };
    let stated = table
        .iter()
        .filter_map(|listing| REMARKS.get(listing.remark as usize))
        .map(|remark| remark.usage)
        .fold(None, |found, usage| match (found, usage) {
            (found, USAGE_UNQUALIFIED) => found,
            (None, stated) => Some(stated),
            (Some(previous), _) => Some(previous),
        });
    match stated {
        Some(USAGE_VERTICAL_ONLY) => Usage::VerticalOnly,
        Some(USAGE_HORIZONTAL_ONLY) if is_rotated_western(member) => {
            Usage::HorizontalOrRotatedWestern
        },
        Some(USAGE_HORIZONTAL_ONLY) => Usage::HorizontalOnly,
        _ => Usage::Both,
    }
}

/// Whether a member is one of the four quotation marks §3.1.1 permits rotated.
fn is_rotated_western(member: Member) -> bool {
    let mut code_points = member.code_points();
    let Some(only) = code_points.next() else {
        return false;
    };
    code_points.next().is_none() && ROTATED_WESTERN.contains(&only)
}

/// Which writing directions the writing system uses a member in.
///
/// JLReq: §3.1.1, §A Remarks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Usage {
    /// Both writing directions, which is every member but twelve.
    Both,
    /// e.g. `U+002E` as a full stop — §3.1.1's three horizontal conventions.
    HorizontalOnly,
    /// e.g. `U+301D` 〝, which §3.1.1 says is "exclusively used for vertical writing
    /// mode and not to be used in horizontal writing mode".
    VerticalOnly,
    /// Horizontal, except that §3.1.1 permits `‘ ’ “ ”` in vertical writing when
    /// Western characters are rotated. The restriction is conditional on a rotation
    /// policy the caller owns.
    HorizontalOrRotatedWestern,
}

impl Usage {
    /// Frozen projection (ADR-0012): whether the writing system restricts this member to
    /// one direction at all.
    ///
    /// The question a caller has — `jlreq::diagnose` asks it of every occurrence, and a
    /// caller deciding whether to rotate a glyph asks it first. `false` for [`Usage::Both`]
    /// and `true` for every restriction, so a further restriction §3.1.1 turns out to state
    /// is detail: a caller who branched on this keeps meaning what they meant, where a
    /// `match` with a catch-all arm would silently treat the new one as unrestricted.
    ///
    /// JLReq: §3.1.1, §A Remarks
    #[must_use]
    pub const fn is_restricted(self) -> bool {
        !matches!(self, Self::Both)
    }
}

#[cfg(test)]
mod tests {
    use super::{ROTATED_WESTERN, Usage, usage};
    use crate::member::Member;

    #[test]
    fn an_ordinary_member_is_used_in_both_directions() {
        assert_eq!(usage(Member::single('あ')), Usage::Both);
        assert_eq!(usage(Member::single('漢')), Usage::Both);
        assert_eq!(
            usage(Member::single('\u{1F600}')),
            Usage::Both,
            "a member Appendix A lists nowhere carries no Remark, so nothing restricts it"
        );
    }

    #[test]
    fn the_vertical_quotation_marks_are_vertical_only() {
        assert_eq!(
            usage(Member::single('\u{301D}')),
            Usage::VerticalOnly,
            "§3.1.1: \"exclusively used for vertical writing mode and not to be used in \
             horizontal writing mode\""
        );
        assert_eq!(usage(Member::single('\u{301F}')), Usage::VerticalOnly);
    }

    #[test]
    fn the_repeat_mark_that_takes_a_following_code_point_is_vertical_only() {
        for mark in ['\u{3033}', '\u{3034}', '\u{3035}'] {
            assert_eq!(
                usage(Member::single(mark)),
                Usage::VerticalOnly,
                "§A.8 marks the vertical kana repeat marks used in vertical composition"
            );
        }
    }

    #[test]
    fn the_three_horizontal_conventions_are_horizontal_only() {
        for convention in ['.', ',', ';'] {
            assert_eq!(
                usage(Member::single(convention)),
                Usage::HorizontalOnly,
                "§3.1.1 enumerates three conventions for full stops and commas in \
                 horizontal writing"
            );
        }
    }

    #[test]
    fn the_four_quotation_marks_are_horizontal_or_rotated_western() {
        for quotation in ROTATED_WESTERN {
            assert_eq!(
                usage(Member::single(quotation)),
                Usage::HorizontalOrRotatedWestern,
                "§3.1.1 permits them in vertical writing when Western characters (cl-27) \
                 are composed rotated 90 degrees clockwise, so reading the Remarks cell \
                 alone would publish a prohibition the section does not state"
            );
        }
    }

    #[test]
    fn the_refinement_applies_to_exactly_four_of_the_seven_horizontal_members() {
        let restricted = [
            '.', ',', ';', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}',
        ];
        let rotated = restricted
            .into_iter()
            .filter(|mark| usage(Member::single(*mark)) == Usage::HorizontalOrRotatedWestern)
            .count();
        assert_eq!(
            (restricted.len(), rotated),
            (7, 4),
            "seven rows carry the horizontal Remark and §3.1.1 refines four of them"
        );
    }

    #[test]
    fn the_direction_a_member_is_used_in_follows_the_compatibility_folding() {
        assert_eq!(
            usage(Member::single('\u{FF0E}')),
            Usage::HorizontalOnly,
            "§A lists U+002E where real text may carry the full-width form, and the \
             restriction belongs to the character rather than to its width"
        );
    }

    #[test]
    fn the_projection_answers_whether_the_writing_system_restricts_the_member_at_all() {
        assert!(!Usage::Both.is_restricted());
        for restricted in [
            Usage::HorizontalOnly,
            Usage::VerticalOnly,
            Usage::HorizontalOrRotatedWestern,
        ] {
            assert!(
                restricted.is_restricted(),
                "{restricted:?} is a restriction, and a caller who branched on this rather \
                 than on the variants keeps meaning what they meant when §3.1.1 turns out to \
                 state a fourth"
            );
        }
    }
}
