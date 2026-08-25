// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

/// A finite resource consumed while composing a paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompositionResource {
    /// Shaped clusters in the paragraph.
    Clusters,
    /// Caller-visible break candidates, including the implicit paragraph end.
    BreakCandidates,
    /// Inline constructs in the paragraph.
    Constructs,
    /// Tab stops available to each line.
    TabStops,
    /// Dynamic-programming transitions and special-element inspections.
    SearchTransitions,
}

impl CompositionResource {
    const fn error_code(self) -> &'static str {
        match self {
            Self::Clusters => "compose.cluster-limit",
            Self::BreakCandidates => "compose.break-candidate-limit",
            Self::Constructs => "compose.construct-limit",
            Self::TabStops => "compose.tab-stop-limit",
            Self::SearchTransitions => "compose.transition-limit",
        }
    }

    const fn description(self) -> &'static str {
        match self {
            Self::Clusters => "cluster limit exceeded",
            Self::BreakCandidates => "break-candidate limit exceeded",
            Self::Constructs => "construct limit exceeded",
            Self::TabStops => "tab-stop limit exceeded",
            Self::SearchTransitions => "composition search transition limit exceeded",
        }
    }
}

/// Deterministic resource limits for one composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct CompositionLimits {
    clusters: usize,
    break_candidates: usize,
    constructs: usize,
    tab_stops: usize,
    search_transitions: usize,
}

impl CompositionLimits {
    /// The default maximum number of shaped clusters.
    pub const DEFAULT_MAX_CLUSTERS: usize = 65_536;
    /// The default maximum number of break candidates.
    pub const DEFAULT_MAX_BREAK_CANDIDATES: usize = 65_536;
    /// The default maximum number of inline constructs.
    pub const DEFAULT_MAX_CONSTRUCTS: usize = 4_096;
    /// The default maximum number of tab stops.
    pub const DEFAULT_MAX_TAB_STOPS: usize = 4_096;
    /// The default maximum number of search transitions and special inspections.
    pub const DEFAULT_MAX_SEARCH_TRANSITIONS: usize = 8_000_000;

    /// The release defaults as a value usable in constants.
    pub const DEFAULT: Self = Self {
        clusters: Self::DEFAULT_MAX_CLUSTERS,
        break_candidates: Self::DEFAULT_MAX_BREAK_CANDIDATES,
        constructs: Self::DEFAULT_MAX_CONSTRUCTS,
        tab_stops: Self::DEFAULT_MAX_TAB_STOPS,
        search_transitions: Self::DEFAULT_MAX_SEARCH_TRANSITIONS,
    };

    /// The shaped-cluster limit.
    #[must_use]
    pub const fn max_clusters(self) -> usize {
        self.clusters
    }

    /// The break-candidate limit.
    #[must_use]
    pub const fn max_break_candidates(self) -> usize {
        self.break_candidates
    }

    /// The inline-construct limit.
    #[must_use]
    pub const fn max_constructs(self) -> usize {
        self.constructs
    }

    /// The tab-stop limit.
    #[must_use]
    pub const fn max_tab_stops(self) -> usize {
        self.tab_stops
    }

    /// The composition-search work limit.
    #[must_use]
    pub const fn max_search_transitions(self) -> usize {
        self.search_transitions
    }

    /// Return limits with a different shaped-cluster maximum.
    #[must_use]
    pub const fn with_max_clusters(mut self, maximum: usize) -> Self {
        self.clusters = maximum;
        self
    }

    /// Return limits with a different break-candidate maximum.
    #[must_use]
    pub const fn with_max_break_candidates(mut self, maximum: usize) -> Self {
        self.break_candidates = maximum;
        self
    }

    /// Return limits with a different inline-construct maximum.
    #[must_use]
    pub const fn with_max_constructs(mut self, maximum: usize) -> Self {
        self.constructs = maximum;
        self
    }

    /// Return limits with a different tab-stop maximum.
    #[must_use]
    pub const fn with_max_tab_stops(mut self, maximum: usize) -> Self {
        self.tab_stops = maximum;
        self
    }

    /// Return limits with a different composition-search work maximum.
    #[must_use]
    pub const fn with_max_search_transitions(mut self, maximum: usize) -> Self {
        self.search_transitions = maximum;
        self
    }
}

impl Default for CompositionLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Composition stopped before producing a layout because a declared resource limit was hit.
///
/// No partial or approximate layout is returned. The same [`crate::Composer`] can be reused
/// immediately after this error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ComposeError {
    resource: CompositionResource,
    limit: usize,
    observed: usize,
}

impl ComposeError {
    pub(crate) const fn new(resource: CompositionResource, limit: usize, observed: usize) -> Self {
        Self {
            resource,
            limit,
            observed,
        }
    }

    /// A stable, language-independent error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.resource.error_code()
    }

    /// The resource whose limit was exceeded.
    #[must_use]
    pub const fn resource(self) -> CompositionResource {
        self.resource
    }

    /// The configured inclusive maximum.
    #[must_use]
    pub const fn limit(self) -> usize {
        self.limit
    }

    /// The amount required or observed when composition stopped.
    #[must_use]
    pub const fn observed(self) -> usize {
        self.observed
    }
}

impl core::fmt::Display for ComposeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.resource.description())
    }
}

impl core::error::Error for ComposeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn every_resource_has_a_stable_code_description_and_display() {
        let cases = [
            (
                CompositionResource::Clusters,
                "compose.cluster-limit",
                "cluster limit exceeded",
            ),
            (
                CompositionResource::BreakCandidates,
                "compose.break-candidate-limit",
                "break-candidate limit exceeded",
            ),
            (
                CompositionResource::Constructs,
                "compose.construct-limit",
                "construct limit exceeded",
            ),
            (
                CompositionResource::TabStops,
                "compose.tab-stop-limit",
                "tab-stop limit exceeded",
            ),
            (
                CompositionResource::SearchTransitions,
                "compose.transition-limit",
                "composition search transition limit exceeded",
            ),
        ];
        for (resource, code, message) in cases {
            let error = ComposeError::new(resource, 13, 17);
            assert_eq!(error.code(), code);
            assert_eq!(resource.description(), message);
            assert_eq!(format!("{error}"), message);
            assert_eq!(error.resource(), resource);
            assert_eq!(error.limit(), 13);
            assert_eq!(error.observed(), 17);
        }
    }
}
