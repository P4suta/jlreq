// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::{string::String, vec::Vec};

use crate::model::{Cluster, Frame, InputError, ShapedText, Size};
use crate::spec;

impl ShapedText {
    /// Validate and own a source string and its shaped cluster sequence.
    pub fn new<S, I>(source: S, size: Size, frame: Frame, clusters: I) -> Result<Self, InputError>
    where
        S: Into<String>,
        I: IntoIterator<Item = Cluster>,
    {
        let source = source.into();
        let clusters: Vec<_> = clusters.into_iter().collect();
        validate_clusters(&source, frame, &clusters)?;
        Ok(Self {
            source,
            size,
            frame,
            clusters,
        })
    }

    pub(crate) fn cluster_boundary(&self, at: usize) -> bool {
        let shaped_boundary = at == 0
            || at == self.source.len()
            || self
                .clusters
                .binary_search_by_key(&at, |cluster| cluster.range().start)
                .is_ok();
        shaped_boundary && !splits_appendix_pair(&self.source, at)
    }

    pub(crate) fn cluster_ordinal(&self, at: usize) -> Option<usize> {
        if at == self.source.len() {
            return Some(self.clusters.len());
        }
        self.clusters
            .binary_search_by_key(&at, |cluster| cluster.range().start)
            .ok()
    }
}

fn validate_clusters(
    source: &str,
    default_frame: Frame,
    clusters: &[Cluster],
) -> Result<(), InputError> {
    if source.is_empty() {
        if clusters.is_empty() {
            return Ok(());
        }
        return Err(InputError::new(
            "input.cluster-out-of-range",
            clusters.first().map(Cluster::range),
            "an empty source cannot contain shaped clusters",
        ));
    }
    if clusters.is_empty() {
        return Err(InputError::new(
            "input.uncovered-text",
            Some(0..source.len()),
            "non-empty source text must be covered by shaped clusters",
        ));
    }

    let mut cursor = 0;
    for cluster in clusters {
        let range = cluster.range();
        if range.start >= range.end || range.end > source.len() {
            return Err(InputError::new(
                "input.cluster-out-of-range",
                Some(range),
                "a cluster range is empty or outside the source",
            ));
        }
        if !source.is_char_boundary(range.start) || !source.is_char_boundary(range.end) {
            return Err(InputError::new(
                "input.invalid-utf8-boundary",
                Some(range),
                "a cluster endpoint is not a UTF-8 code-point boundary",
            ));
        }
        match range.start.cmp(&cursor) {
            core::cmp::Ordering::Less => {
                return Err(InputError::new(
                    "input.overlapping-clusters",
                    Some(range),
                    "clusters must cover the source exactly once in source order",
                ));
            },
            core::cmp::Ordering::Greater => {
                return Err(InputError::new(
                    "input.uncovered-text",
                    Some(range),
                    "clusters must cover the source exactly once in source order",
                ));
            },
            core::cmp::Ordering::Equal => {},
        }
        if cluster.advance() < 0 {
            return Err(InputError::new(
                "input.negative-advance",
                Some(range.clone()),
                "a shaped advance cannot be negative",
            ));
        }
        let piece = &source[range.clone()];
        if piece.chars().count() > 1
            && cluster.frame_override().unwrap_or(default_frame) != Frame::Proportional
            && !is_appendix_pair(piece)
        {
            return Err(InputError::new(
                "input.cluster-covers-multiple-keys",
                Some(range.clone()),
                "a non-proportional shaped cluster may cover only one Appendix A key",
            ));
        }
        cursor = range.end;
    }
    if cursor != source.len() {
        return Err(InputError::new(
            "input.uncovered-text",
            Some(cursor..source.len()),
            "clusters must cover the source exactly once",
        ));
    }
    Ok(())
}

fn splits_appendix_pair(source: &str, at: usize) -> bool {
    if !source.is_char_boundary(at) {
        return false;
    }
    let Some(before) = source[..at].chars().next_back() else {
        return false;
    };
    let Some(after) = source[at..].chars().next() else {
        return false;
    };
    spec::is_pair(before, after)
}

fn is_appendix_pair(piece: &str) -> bool {
    let mut characters = piece.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    let Some(second) = characters.next() else {
        return false;
    };
    characters.next().is_none() && spec::is_pair(first, second)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size() -> Size {
        Size::square(1_000).expect("positive size")
    }

    #[test]
    fn shaped_boundaries_include_endpoints_but_not_appendix_pair_splits() {
        let text = ShapedText::new(
            "ab",
            size(),
            Frame::Proportional,
            [Cluster::new(0..1, 500), Cluster::new(1..2, 500)],
        )
        .expect("valid text");
        assert!(text.cluster_boundary(0));
        assert!(text.cluster_boundary(1));
        assert!(text.cluster_boundary(2));
        assert!(!text.cluster_boundary(3));

        let pair = "\u{02e5}\u{02e9}";
        let split = '\u{02e5}'.len_utf8();
        let paired = ShapedText::new(
            pair,
            size(),
            Frame::Proportional,
            [
                Cluster::new(0..split, 500),
                Cluster::new(split..pair.len(), 500),
            ],
        )
        .expect("proportional pair may be separately shaped");
        assert!(!paired.cluster_boundary(split));
    }

    #[test]
    fn cluster_validation_distinguishes_range_and_advance_boundaries() {
        let empty = ShapedText::new("a", size(), Frame::FullEm, [Cluster::new(0..0, 0)])
            .expect_err("empty cluster range");
        assert_eq!(empty.code(), "input.cluster-out-of-range");

        let outside = ShapedText::new("a", size(), Frame::FullEm, [Cluster::new(0..2, 0)])
            .expect_err("outside cluster range");
        assert_eq!(outside.code(), "input.cluster-out-of-range");

        let negative = ShapedText::new("a", size(), Frame::FullEm, [Cluster::new(0..1, -1)])
            .expect_err("negative advance");
        assert_eq!(negative.code(), "input.negative-advance");
        assert!(
            ShapedText::new("a", size(), Frame::FullEm, [Cluster::new(0..1, 0)]).is_ok(),
            "zero advance is valid"
        );

        let overlap = ShapedText::new(
            "abc",
            size(),
            Frame::Proportional,
            [Cluster::new(0..2, 1), Cluster::new(1..3, 1)],
        )
        .expect_err("overlapping clusters");
        assert_eq!(overlap.code(), "input.overlapping-clusters");
        let gap = ShapedText::new(
            "abc",
            size(),
            Frame::Proportional,
            [Cluster::new(0..1, 1), Cluster::new(2..3, 1)],
        )
        .expect_err("uncovered text between clusters");
        assert_eq!(gap.code(), "input.uncovered-text");
    }

    #[test]
    fn appendix_pair_helpers_cover_all_guard_edges() {
        let pair = "\u{02e5}\u{02e9}";
        let split = '\u{02e5}'.len_utf8();
        assert!(is_appendix_pair(pair));
        assert!(!is_appendix_pair(""));
        assert!(!is_appendix_pair("a"));
        assert!(!is_appendix_pair("abc"));
        assert!(splits_appendix_pair(pair, split));
        assert!(!splits_appendix_pair(pair, 0));
        assert!(!splits_appendix_pair(pair, pair.len()));
        assert!(!splits_appendix_pair("é", 1));
    }
}
