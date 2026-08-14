// SPDX-FileCopyrightText: 2026 kumihan contributors
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
                .iter()
                .any(|cluster| cluster.range().start == at || cluster.range().end == at);
        shaped_boundary && !splits_appendix_pair(&self.source, at)
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
        if range.start != cursor {
            let code = if range.start < cursor {
                "input.overlapping-clusters"
            } else {
                "input.uncovered-text"
            };
            return Err(InputError::new(
                code,
                Some(range),
                "clusters must cover the source exactly once in source order",
            ));
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
    if at == 0 || at >= source.len() || !source.is_char_boundary(at) {
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
