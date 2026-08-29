// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

#[derive(Debug)]
struct CallState {
    runs: usize,
    glyphs: usize,
    max_runs: usize,
    max_glyphs: usize,
    used_fonts: BTreeSet<FontId>,
    diagnostics: Vec<Diagnostic>,
    font_candidates: BTreeMap<FontCandidateKey, Arc<[FontId]>>,
    font_selections: BTreeMap<FontSelectionKey, (FontId, bool)>,
    #[cfg(test)]
    shape_calls: usize,
}

impl CallState {
    fn new(options: &LayoutOptions) -> Self {
        Self {
            runs: 0,
            glyphs: 0,
            max_runs: options.limits.runs,
            max_glyphs: options.limits.glyphs,
            used_fonts: BTreeSet::new(),
            diagnostics: Vec::new(),
            font_candidates: BTreeMap::new(),
            font_selections: BTreeMap::new(),
            #[cfg(test)]
            shape_calls: 0,
        }
    }

    fn charge_run(&mut self) -> Result<(), LayoutError> {
        self.runs = self.runs.saturating_add(1);
        check_limit(Resource::Runs, self.max_runs, self.runs)
    }

    fn charge_glyphs(&mut self, amount: usize) -> Result<(), LayoutError> {
        self.glyphs = self.glyphs.saturating_add(amount);
        check_limit(Resource::Glyphs, self.max_glyphs, self.glyphs)
    }

    #[cfg(test)]
    fn charge_shape(&mut self) {
        self.shape_calls = self.shape_calls.saturating_add(1);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FontCandidateKey {
    families: Vec<String>,
    weight: u16,
    width: u16,
    slant: u8,
}

impl FontCandidateKey {
    fn new(style: &EffectiveStyle) -> Self {
        let slant = match style.font_style.slant() {
            FontSlant::Normal => 0,
            FontSlant::Italic => 1,
            FontSlant::Oblique => 2,
        };
        Self {
            families: style.families.clone(),
            weight: style.font_style.weight(),
            width: style.font_style.width(),
            slant,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FontSelectionKey {
    text: String,
    candidates: FontCandidateKey,
    size: i32,
    direction: u8,
    language: String,
    features: Vec<([u8; 4], u32)>,
    variations: Vec<([u8; 4], u32)>,
}

impl FontSelectionKey {
    fn new(
        text: &str,
        style: &EffectiveStyle,
        direction: Direction,
        candidates: FontCandidateKey,
    ) -> Self {
        let direction = match direction {
            Direction::Invalid => 0,
            Direction::LeftToRight => 1,
            Direction::RightToLeft => 2,
            Direction::TopToBottom => 3,
            Direction::BottomToTop => 4,
        };
        Self {
            text: text.to_owned(),
            candidates,
            size: style.size,
            direction,
            language: style.language.clone(),
            features: style
                .features
                .iter()
                .map(|feature| (feature.tag().bytes(), feature.value()))
                .collect(),
            variations: style
                .variations
                .iter()
                .map(|variation| (variation.tag().bytes(), variation.value().to_bits()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct EffectiveStyle {
    families: Vec<String>,
    font_style: FontStyle,
    size: i32,
    language: String,
    features: Vec<OpenTypeFeature>,
    variations: Vec<FontVariation>,
    role: TextRole,
}

struct StyleResolver<'a> {
    spans: &'a [(Range<usize>, SpanStyle)],
    next: usize,
    base: Arc<EffectiveStyle>,
    selected: Option<(usize, Arc<EffectiveStyle>)>,
}

impl<'a> StyleResolver<'a> {
    fn new(
        spans: &'a [(Range<usize>, SpanStyle)],
        options: &LayoutOptions,
        global_offset: usize,
    ) -> Self {
        Self {
            spans,
            next: spans.partition_point(|(range, _)| range.end <= global_offset),
            base: Arc::new(base_effective_style(options)),
            selected: None,
        }
    }

    fn resolve(&mut self, global: &Range<usize>) -> Result<Arc<EffectiveStyle>, LayoutError> {
        while self
            .spans
            .get(self.next)
            .is_some_and(|(range, _)| range.end <= global.start)
        {
            self.next = self.next.saturating_add(1);
            self.selected = None;
        }
        let Some((range, style)) = self.spans.get(self.next) else {
            return Ok(Arc::clone(&self.base));
        };
        if !ranges_overlap(range, global) {
            return Ok(Arc::clone(&self.base));
        }
        if range.start > global.start || range.end < global.end {
            return Err(LayoutError::invalid_document(
                "document.span-splits-grapheme",
                Some(global.clone()),
            ));
        }
        if let Some((index, selected)) = &self.selected
            && *index == self.next
        {
            return Ok(Arc::clone(selected));
        }
        let selected = Arc::new(span_effective_style(&self.base, style));
        self.selected = Some((self.next, Arc::clone(&selected)));
        Ok(selected)
    }
}

#[derive(Debug)]
struct GraphemeItem {
    range: Range<usize>,
    level: Level,
    script: ScriptClass,
    direction: Direction,
    font_id: FontId,
    effective: Arc<EffectiveStyle>,
    is_tab: bool,
}

impl GraphemeItem {
    fn same_run(&self, other: &Self) -> bool {
        self.font_id == other.font_id
            && self.level.is_rtl() == other.level.is_rtl()
            && self.script == other.script
            && self.direction == other.direction
            && (Arc::ptr_eq(&self.effective, &other.effective) || self.effective == other.effective)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptClass {
    Japanese,
    Latin,
    Rtl,
    Emoji,
    Other,
}

#[derive(Debug, Clone)]
struct RawGlyph {
    font_id: FontId,
    glyph_id: u32,
    cluster: usize,
    x_advance: i32,
    y_advance: i32,
    x_offset: i32,
    y_offset: i32,
}

impl RawGlyph {
    fn inline_advance(&self, direction: Direction) -> i32 {
        match direction {
            Direction::TopToBottom | Direction::BottomToTop => self.y_advance.abs(),
            _ => self.x_advance.abs(),
        }
    }
}

#[derive(Debug, Clone)]
struct PreparedCluster {
    range: Range<usize>,
    advance: i32,
    size: i32,
    frame: jlreq_core::Frame,
    role: Option<jlreq_core::ClusterRole>,
    bidi_level: u8,
    glyphs: Vec<RawGlyph>,
}

#[derive(Debug, Clone)]
struct PreparedText {
    clusters: Vec<PreparedCluster>,
}

impl PreparedText {
    fn to_core(
        &self,
        source: &str,
        default_size: i32,
    ) -> Result<jlreq_core::ShapedText, LayoutError> {
        let size = jlreq_core::Size::square(default_size)?;
        let mut clusters = Vec::with_capacity(self.clusters.len());
        for cluster in &self.clusters {
            let local_size = jlreq_core::Size::square(cluster.size)?;
            let mut core = jlreq_core::Cluster::new(cluster.range.clone(), cluster.advance)
                .with_size(local_size)
                .with_frame(cluster.frame);
            if let Some(role) = cluster.role {
                core = core.with_role(role);
            }
            clusters.push(core);
        }
        Ok(jlreq_core::ShapedText::new(
            source,
            size,
            jlreq_core::Frame::FullEm,
            clusters,
        )?)
    }

    fn is_boundary(&self, offset: usize, source_len: usize) -> bool {
        offset == 0
            || offset == source_len
            || self
                .clusters
                .binary_search_by_key(&offset, |cluster| cluster.range.start)
                .is_ok()
    }

    fn cluster_range(&self, range: &Range<usize>) -> Range<usize> {
        let start = self
            .clusters
            .partition_point(|cluster| cluster.range.end <= range.start);
        let end = self
            .clusters
            .partition_point(|cluster| cluster.range.start < range.end);
        start..end
    }
}

#[derive(Debug, Clone)]
struct AttachmentShape {
    global_ordinal: usize,
    base: Range<usize>,
    prepared: PreparedText,
}

#[derive(Debug, Clone)]
struct ParagraphSegment {
    content: Range<usize>,
}

fn validate_call(
    document: &Document,
    fonts: &FontLibrary,
    options: &LayoutOptions,
) -> Result<(), LayoutError> {
    if fonts.is_empty() {
        return Err(LayoutError::NoFonts);
    }
    check_limit(
        Resource::InputBytes,
        options.limits.input_bytes,
        document.text.len(),
    )?;
    check_limit(Resource::Fonts, options.limits.fonts, fonts.len())?;
    let font_bytes = fonts
        .fonts()
        .iter()
        .fold(0_usize, |sum, font| sum.saturating_add(font.bytes().len()));
    check_limit(Resource::FontBytes, options.limits.font_bytes, font_bytes)?;
    check_limit(
        Resource::Constructs,
        options.limits.constructs,
        document.constructs.len(),
    )
}

fn check_limit(resource: Resource, limit: usize, observed: usize) -> Result<(), LayoutError> {
    if observed > limit {
        Err(LayoutError::resource(resource, limit, observed))
    } else {
        Ok(())
    }
}

fn map_core_resource_error(error: jlreq_core::ComposeError) -> LayoutError {
    let Some(resource) = high_level_resource(error.resource()) else {
        return LayoutError::CoreComposition(error);
    };
    LayoutError::resource(resource, error.limit(), error.observed())
}

fn high_level_resource(resource: jlreq_core::CompositionResource) -> Option<Resource> {
    match resource {
        jlreq_core::CompositionResource::Clusters => Some(Resource::Glyphs),
        jlreq_core::CompositionResource::BreakCandidates => Some(Resource::Runs),
        jlreq_core::CompositionResource::Constructs | jlreq_core::CompositionResource::TabStops => {
            Some(Resource::Constructs)
        },
        jlreq_core::CompositionResource::SearchTransitions => Some(Resource::CoreOperations),
        _ => None,
    }
}

#[cfg(test)]
fn effective_style(
    global: &Range<usize>,
    spans: &[(Range<usize>, SpanStyle)],
    options: &LayoutOptions,
) -> Result<EffectiveStyle, LayoutError> {
    let mut selected = None;
    for (range, style) in spans {
        if ranges_overlap(range, global) {
            if range.start > global.start || range.end < global.end {
                return Err(LayoutError::invalid_document(
                    "document.span-splits-grapheme",
                    Some(global.clone()),
                ));
            }
            selected = Some(style);
            break;
        }
    }
    let base = base_effective_style(options);
    Ok(selected.map_or(base.clone(), |style| span_effective_style(&base, style)))
}

fn base_effective_style(options: &LayoutOptions) -> EffectiveStyle {
    EffectiveStyle {
        families: Vec::new(),
        font_style: FontStyle::default(),
        size: options.font_size,
        language: options.language.clone(),
        features: options.features.clone(),
        variations: options.variations.clone(),
        role: TextRole::Text,
    }
}

fn span_effective_style(base: &EffectiveStyle, style: &SpanStyle) -> EffectiveStyle {
    let mut result = base.clone();
    result.families.clone_from(&style.families);
    result.font_style = style.font_style;
    result.size = style.font_size.unwrap_or(base.size);
    if let Some(language) = &style.language {
        result.language.clone_from(language);
    }
    result.features.extend_from_slice(&style.features);
    result.variations.extend_from_slice(&style.variations);
    result.role = style.role;
    result
}

