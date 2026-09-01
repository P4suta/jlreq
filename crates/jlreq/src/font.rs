// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "system-fonts")]
use crate::OpenTypeTag;
#[cfg(feature = "system-fonts")]
use crate::units::quantize;
use crate::units::to_f32;
use crate::{FontVariation, LayoutError};

pub(crate) fn unknown_font_id() -> LayoutError {
    LayoutError::invalid_font_request(
        "font.unknown-id",
        "the font identifier does not belong to this font library",
    )
}

/// Source of the per-library provenance nonce; zero is reserved for
/// "no face registered yet".
static NEXT_LIBRARY_NONCE: AtomicU64 = AtomicU64::new(1);

/// Stable identifier assigned by a [`FontLibrary`].
///
/// Equality, ordering, hashing, and [`Debug`](fmt::Debug) identify the
/// library slot only, which keeps layouts produced from identical bytes and
/// options bit-identical across distinct libraries. Provenance is checked by
/// the lookups instead: [`FontLibrary::get`] and [`crate::TextLayout::font`]
/// return `None` for an identifier minted by a different library rather than
/// silently resolving the wrong font.
#[derive(Clone, Copy)]
pub struct FontId {
    index: u32,
    nonce: u64,
}

impl FontId {
    /// Numeric value suitable for renderer-side maps.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.index
    }

    pub(crate) const fn same_provenance(self, other: Self) -> bool {
        self.index == other.index && self.nonce == other.nonce
    }
}

impl PartialEq for FontId {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl Eq for FontId {}

impl PartialOrd for FontId {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FontId {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

impl core::hash::Hash for FontId {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl fmt::Debug for FontId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "FontId({})", self.index)
    }
}

/// Font slant requested by a span or recorded for a face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum FontSlant {
    /// Upright design.
    #[default]
    Normal,
    /// Designed italic face.
    Italic,
    /// Mechanically or explicitly oblique face.
    Oblique,
}

/// Family matching attributes kept independent of Fontique's public types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct FontStyle {
    weight: u16,
    width: u16,
    slant: FontSlant,
}

impl FontStyle {
    /// Build a style using CSS-like weight and percentage width values.
    #[must_use]
    pub const fn new(weight: u16, width: u16, slant: FontSlant) -> Self {
        Self {
            weight,
            width,
            slant,
        }
    }

    /// CSS-like weight, normally 100 through 900.
    #[must_use]
    pub const fn weight(self) -> u16 {
        self.weight
    }

    /// Percentage width where 100 is normal.
    #[must_use]
    pub const fn width(self) -> u16 {
        self.width
    }

    /// Requested slant.
    #[must_use]
    pub const fn slant(self) -> FontSlant {
        self.slant
    }
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::new(400, 100, FontSlant::Normal)
    }
}

#[cfg(feature = "system-fonts")]
fn system_attributes(style: FontStyle) -> fontique::Attributes {
    let slant = match style.slant() {
        FontSlant::Normal => fontique::FontStyle::Normal,
        FontSlant::Italic => fontique::FontStyle::Italic,
        FontSlant::Oblique => fontique::FontStyle::Oblique(None),
    };
    fontique::Attributes::new(
        fontique::FontWidth::from_percentage(f32::from(style.width())),
        slant,
        fontique::FontWeight::new(f32::from(style.weight())),
    )
}

/// Renderer-side synthetic styling requested for a selected font face.
///
/// Variable-axis synthesis is exposed separately through
/// [`FontResource::default_variations`]. This value carries only operations a
/// renderer must apply outside normal variable-font instancing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub struct FontSynthesis {
    embolden: bool,
    skew: i32,
}

impl FontSynthesis {
    #[cfg(feature = "system-fonts")]
    pub(crate) fn new(embolden: bool, skew: Option<f32>) -> Self {
        Self {
            embolden,
            skew: skew.map_or(0, quantize),
        }
    }

    /// Whether the renderer should apply synthetic emboldening.
    #[must_use]
    pub const fn embolden(self) -> bool {
        self.embolden
    }

    /// Synthetic skew angle in degrees, when one is required.
    #[must_use]
    pub fn skew(self) -> Option<f32> {
        (self.skew != 0).then(|| to_f32(self.skew))
    }

    /// Synthetic skew angle in signed 26.6 degrees.
    #[must_use]
    pub const fn skew_26_6(self) -> Option<i32> {
        if self.skew == 0 {
            None
        } else {
            Some(self.skew)
        }
    }

    /// Whether neither emboldening nor skew is required.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.embolden && self.skew == 0
    }
}

/// Em-relative design metrics for a registered face.
///
/// Values are fractions of the em — design units divided by `unitsPerEm` — so
/// multiplying by a resolved font size yields layout units. The sign
/// convention is the font's own: ascent is normally positive, descent and
/// underline position normally negative. `OS/2` typographic values are
/// preferred over `hhea` when the font carries them.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct FontMetrics {
    ascent: f32,
    descent: f32,
    line_gap: f32,
    x_height: Option<f32>,
    cap_height: Option<f32>,
    underline_position: Option<f32>,
    underline_thickness: Option<f32>,
}

impl FontMetrics {
    /// Em-relative typographic ascent.
    #[must_use]
    pub const fn ascent(self) -> f32 {
        self.ascent
    }

    /// Em-relative typographic descent, normally negative.
    #[must_use]
    pub const fn descent(self) -> f32 {
        self.descent
    }

    /// Em-relative extra distance the font recommends between lines.
    #[must_use]
    pub const fn line_gap(self) -> f32 {
        self.line_gap
    }

    /// Em-relative x-height, when the font declares one.
    #[must_use]
    pub const fn x_height(self) -> Option<f32> {
        self.x_height
    }

    /// Em-relative cap height, when the font declares one.
    #[must_use]
    pub const fn cap_height(self) -> Option<f32> {
        self.cap_height
    }

    /// Em-relative underline center position, when the font declares one.
    #[must_use]
    pub const fn underline_position(self) -> Option<f32> {
        self.underline_position
    }

    /// Em-relative underline thickness, when the font declares one.
    #[must_use]
    pub const fn underline_thickness(self) -> Option<f32> {
        self.underline_thickness
    }
}

fn em_relative_metrics(raw: crate::sfnt::RawMetrics) -> FontMetrics {
    let units_per_em = f32::from(raw.units_per_em);
    let em = move |value: i16| f32::from(value) / units_per_em;
    FontMetrics {
        ascent: em(raw.ascent),
        descent: em(raw.descent),
        line_gap: em(raw.line_gap),
        x_height: raw.x_height.map(em),
        cap_height: raw.cap_height.map(em),
        underline_position: raw.underline_position.map(em),
        underline_thickness: raw.underline_thickness.map(em),
    }
}

fn table_bytes<'a>(font: &harfrust::FontRef<'a>, tag: [u8; 4]) -> Option<&'a [u8]> {
    let data = font.table_data(harfrust::Tag::new(&tag))?;
    Some(data.as_bytes())
}

fn derived_family(font: &harfrust::FontRef<'_>) -> Option<String> {
    crate::sfnt::family_from_name_table(table_bytes(font, *b"name")?)
}

fn derived_metrics(font: &harfrust::FontRef<'_>) -> Option<FontMetrics> {
    crate::sfnt::metrics_from_tables(
        table_bytes(font, *b"head"),
        table_bytes(font, *b"hhea"),
        table_bytes(font, *b"OS/2"),
        table_bytes(font, *b"post"),
    )
    .map(em_relative_metrics)
}

/// Font bytes retained by a completed layout for renderer use.
#[derive(Clone)]
#[non_exhaustive]
pub struct FontResource {
    pub(crate) id: FontId,
    pub(crate) bytes: Arc<[u8]>,
    pub(crate) face_index: u32,
    pub(crate) family: String,
    pub(crate) style: FontStyle,
    pub(crate) default_variations: Vec<FontVariation>,
    pub(crate) synthesis: FontSynthesis,
    pub(crate) metrics: Option<FontMetrics>,
    pub(crate) shaper_data: Arc<harfrust::ShaperData>,
}

impl PartialEq for FontResource {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.face_index == other.face_index
            && self.bytes == other.bytes
            && self.family == other.family
            && self.style == other.style
            && self.default_variations == other.default_variations
            && self.synthesis == other.synthesis
    }
}

impl Eq for FontResource {}

impl fmt::Debug for FontResource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FontResource")
            .field("id", &self.id)
            .field("byte_len", &self.bytes.len())
            .field("face_index", &self.face_index)
            .field("family", &self.family)
            .field("style", &self.style)
            .field("default_variations", &self.default_variations)
            .field("synthesis", &self.synthesis)
            .finish_non_exhaustive()
    }
}

impl FontResource {
    /// Library identifier referenced by glyph placements.
    #[must_use]
    pub const fn id(&self) -> FontId {
        self.id
    }

    /// Original TTF, OTF, or TTC bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Face index inside the original bytes.
    #[must_use]
    pub const fn face_index(&self) -> u32 {
        self.face_index
    }

    /// Registered family label.
    #[must_use]
    pub fn family(&self) -> &str {
        &self.family
    }

    /// Registered style label.
    #[must_use]
    pub const fn style(&self) -> FontStyle {
        self.style
    }

    /// Variable-axis defaults selected with this face by the system matcher.
    ///
    /// Explicitly registered faces return an empty slice. During layout these
    /// values override global settings with the same tag and are in turn
    /// overridden by span-specific settings.
    #[must_use]
    pub fn default_variations(&self) -> &[FontVariation] {
        &self.default_variations
    }

    /// Synthetic emboldening or skew required to reproduce the selected face.
    #[must_use]
    pub const fn synthesis(&self) -> FontSynthesis {
        self.synthesis
    }

    /// Em-relative design metrics, when the face carries well-formed
    /// `head` and `hhea` tables.
    ///
    /// Renderers use these for underline, strikethrough, and baseline
    /// alignment; composition itself never depends on them.
    #[must_use]
    pub const fn metrics(&self) -> Option<FontMetrics> {
        self.metrics
    }
}

/// Ordered, in-memory font faces used for primary selection and fallback.
#[derive(Clone, Default)]
pub struct FontLibrary {
    fonts: Vec<FontResource>,
    primary: Option<FontId>,
    fallback_order: Vec<FontId>,
    nonce: u64,
}

impl fmt::Debug for FontLibrary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FontLibrary")
            .field("fonts", &self.fonts)
            .field("primary", &self.primary)
            .field("fallback_order", &self.fallback_order)
            .finish_non_exhaustive()
    }
}

impl FontLibrary {
    /// Create an empty library. Layout requires at least one registered face.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fonts: Vec::new(),
            primary: None,
            fallback_order: Vec::new(),
            nonce: 0,
        }
    }

    /// Register face zero, deriving the family name from the font itself.
    ///
    /// The family comes from the font's `name` table (typographic family
    /// preferred), so a later [`SpanStyle`](crate::SpanStyle) family request
    /// matches the face without a manual [`register_face`](Self::register_face)
    /// call. A font without a usable name keeps an empty family and is reached
    /// through primary/fallback order only.
    pub fn register_font<B>(&mut self, bytes: B) -> Result<FontId, LayoutError>
    where
        B: Into<Arc<[u8]>>,
    {
        self.register_face(bytes, 0, "", FontStyle::default())
    }

    /// Register one face from TTF, OTF, or TTC bytes.
    ///
    /// An empty `family` asks the library to derive one from the font's
    /// `name` table, exactly like [`register_font`](Self::register_font).
    pub fn register_face<B>(
        &mut self,
        bytes: B,
        face_index: u32,
        family: impl Into<String>,
        style: FontStyle,
    ) -> Result<FontId, LayoutError>
    where
        B: Into<Arc<[u8]>>,
    {
        let bytes = bytes.into();
        self.register_face_with_rendering(
            bytes,
            face_index,
            family.into(),
            style,
            Vec::new(),
            FontSynthesis::default(),
        )
    }

    fn register_face_with_rendering(
        &mut self,
        bytes: Arc<[u8]>,
        face_index: u32,
        family: String,
        style: FontStyle,
        default_variations: Vec<FontVariation>,
        synthesis: FontSynthesis,
    ) -> Result<FontId, LayoutError> {
        let font = harfrust::FontRef::from_index(&bytes, face_index)
            .map_err(|_| LayoutError::invalid_font(face_index))?;
        let family = if family.is_empty() {
            derived_family(&font).unwrap_or(family)
        } else {
            family
        };
        let metrics = derived_metrics(&font);
        let shaper_data = Arc::new(harfrust::ShaperData::new(&font));
        let ordinal = u32::try_from(self.fonts.len()).map_err(|_| {
            LayoutError::invalid_font_request(
                "font.too-many-ids",
                "the library already holds the maximum number of font identifiers",
            )
        })?;
        if self.nonce == 0 {
            self.nonce = NEXT_LIBRARY_NONCE.fetch_add(1, Ordering::Relaxed);
        }
        let id = FontId {
            index: ordinal,
            nonce: self.nonce,
        };
        self.fonts.push(FontResource {
            id,
            bytes,
            face_index,
            family,
            style,
            default_variations,
            synthesis,
            metrics,
            shaper_data,
        });
        self.fallback_order.push(id);
        if self.primary.is_none() {
            self.primary = Some(id);
        }
        Ok(id)
    }

    /// Make a registered face the `.notdef` source and first fallback candidate.
    pub fn set_primary(&mut self, id: FontId) -> Result<(), LayoutError> {
        self.get(id).ok_or_else(unknown_font_id)?;
        self.primary = Some(id);
        Ok(())
    }

    /// Replace fallback priority after the primary face.
    pub fn set_fallback_order<I>(&mut self, ids: I) -> Result<(), LayoutError>
    where
        I: IntoIterator<Item = FontId>,
    {
        let mut next = Vec::new();
        for id in ids {
            if self.get(id).is_none() {
                return Err(unknown_font_id());
            }
            if !next.contains(&id) {
                next.push(id);
            }
        }
        for font in &self.fonts {
            if !next.contains(&font.id) {
                next.push(font.id);
            }
        }
        self.fallback_order = next;
        Ok(())
    }

    /// Registered faces in identifier order.
    #[must_use]
    pub fn fonts(&self) -> &[FontResource] {
        &self.fonts
    }

    /// Number of registered faces.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.fonts.len()
    }

    /// Whether no face has been registered.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }

    /// Primary face, if present.
    #[must_use]
    pub const fn primary(&self) -> Option<FontId> {
        self.primary
    }

    /// Look up registration metadata.
    ///
    /// Returns `None` for an identifier minted by a different
    /// [`FontLibrary`], even when the slot index is in range.
    #[must_use]
    pub fn get(&self, id: FontId) -> Option<&FontResource> {
        self.fonts
            .get(id.index as usize)
            .filter(|font| font.id.same_provenance(id))
    }

    pub(crate) fn has_family(&self, family: &str) -> bool {
        self.fonts
            .iter()
            .any(|font| font.family.eq_ignore_ascii_case(family))
    }

    #[cfg(feature = "system-fonts")]
    #[cfg_attr(docsrs, doc(cfg(feature = "system-fonts")))]
    /// Resolve and copy one system family face through Fontique.
    ///
    /// System selection is intentionally opt-in and is not covered by the cross-platform
    /// determinism guarantee. Once registered, the selected bytes are owned exactly like an
    /// explicitly supplied font.
    pub fn register_system_family(
        &mut self,
        family: &str,
        style: FontStyle,
    ) -> Result<FontId, LayoutError> {
        use fontique::{
            Collection, CollectionOptions, QueryStatus, SourceCache, SourceCacheOptions,
        };

        let mut collection = Collection::new(CollectionOptions {
            shared: false,
            system_fonts: true,
        });
        let mut cache = SourceCache::new(SourceCacheOptions { shared: false });
        let mut selected = None;
        {
            let mut query = collection.query(&mut cache);
            query.set_families([family]);
            query.set_attributes(system_attributes(style));
            query.matches_with(|font| {
                selected = Some((
                    Arc::<[u8]>::from(font.blob.as_ref()),
                    font.index,
                    font.synthesis,
                ));
                QueryStatus::Stop
            });
        }
        let (bytes, index, selected_synthesis) = selected.ok_or_else(|| {
            LayoutError::invalid_font_request(
                "font.system-family-not-found",
                "no installed system font matches the requested family",
            )
        })?;
        let default_variations = selected_synthesis
            .variation_settings()
            .iter()
            .map(|(tag, value)| {
                FontVariation::try_new(OpenTypeTag::from_bytes(tag.to_be_bytes()), *value)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let synthesis =
            FontSynthesis::new(selected_synthesis.embolden(), selected_synthesis.skew());
        self.register_face_with_rendering(
            bytes,
            index,
            family.to_owned(),
            style,
            default_variations,
            synthesis,
        )
    }

    pub(crate) fn ordered_candidates(
        &self,
        families: &[String],
        requested: FontStyle,
    ) -> Vec<FontId> {
        let mut result = Vec::with_capacity(self.fonts.len());
        for family in families {
            let mut matching: Vec<_> = self
                .fonts
                .iter()
                .filter(|font| font.family.eq_ignore_ascii_case(family))
                .collect();
            matching.sort_by_key(|font| {
                let slant = usize::from(font.style.slant != requested.slant);
                let weight = font.style.weight.abs_diff(requested.weight) as usize;
                let width = font.style.width.abs_diff(requested.width) as usize;
                (slant, weight, width, font.id)
            });
            for font in matching {
                if !result.contains(&font.id) {
                    result.push(font.id);
                }
            }
        }
        if let Some(primary) = self.primary
            && !result.contains(&primary)
        {
            result.push(primary);
        }
        for id in &self.fallback_order {
            if !result.contains(id) {
                result.push(*id);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_NONCE: u64 = 7;

    const fn test_id(index: u32) -> FontId {
        FontId {
            index,
            nonce: TEST_NONCE,
        }
    }

    fn resource(id: u32, face_index: u32, family: &str, style: FontStyle) -> FontResource {
        let bytes = Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF);
        let font = harfrust::FontRef::from_index(&bytes, 0).unwrap();
        let shaper_data = Arc::new(harfrust::ShaperData::new(&font));
        FontResource {
            id: test_id(id),
            bytes,
            face_index,
            family: family.to_owned(),
            style,
            default_variations: Vec::new(),
            synthesis: FontSynthesis::default(),
            metrics: None,
            shaper_data,
        }
    }

    fn fixture_library() -> FontLibrary {
        let regular = FontStyle::new(400, 100, FontSlant::Normal);
        let italic = FontStyle::new(500, 90, FontSlant::Italic);
        FontLibrary {
            fonts: vec![
                resource(0, 4, "Family", regular),
                resource(1, 7, "Family", italic),
                resource(2, 9, "Fallback", regular),
            ],
            primary: Some(test_id(0)),
            fallback_order: vec![test_id(0), test_id(1), test_id(2)],
            nonce: TEST_NONCE,
        }
    }

    #[test]
    fn identifiers_resources_and_libraries_have_exact_observable_metadata() {
        assert_eq!(test_id(42).get(), 42);
        let font = resource(
            42,
            7,
            "Distinct Family",
            FontStyle::new(525, 91, FontSlant::Oblique),
        );
        assert_eq!(font.id().get(), 42);
        assert_eq!(font.face_index(), 7);
        assert_eq!(font.bytes(), font_test_data::NOTO_SANS_JP_CFF);
        assert_eq!(font.family(), "Distinct Family");
        let rendered = format!("{font:?}");
        assert!(rendered.contains("FontResource"));
        assert!(rendered.contains("byte_len: 3"));
        assert!(rendered.contains("face_index: 7"));
        assert!(rendered.contains("Distinct Family"));

        let library = fixture_library();
        let rendered = format!("{library:?}");
        assert!(rendered.contains("FontLibrary"));
        assert!(rendered.contains("fallback_order"));
        assert!(rendered.contains("FontId(2)"));
    }

    #[test]
    fn registration_derives_family_and_metrics_from_the_font() {
        let mut library = FontLibrary::new();
        let id = library
            .register_font(Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF))
            .unwrap();
        let resource = library.get(id).unwrap();
        assert_eq!(resource.family(), "Noto Sans CJK JP");
        assert!(library.has_family("noto sans cjk jp"));
        assert!(!library.has_family("Absent Family"));

        let metrics = resource.metrics().unwrap();
        assert_eq!(metrics.ascent().to_bits(), (880.0_f32 / 1000.0).to_bits());
        assert_eq!(metrics.descent().to_bits(), (-120.0_f32 / 1000.0).to_bits());
        assert_eq!(metrics.line_gap().to_bits(), 0.0_f32.to_bits());
        assert_eq!(
            metrics.x_height().map(f32::to_bits),
            Some((543.0_f32 / 1000.0).to_bits())
        );
        assert_eq!(
            metrics.cap_height().map(f32::to_bits),
            Some((733.0_f32 / 1000.0).to_bits())
        );
        assert_eq!(
            metrics.underline_position().map(f32::to_bits),
            Some((-125.0_f32 / 1000.0).to_bits())
        );
        assert_eq!(
            metrics.underline_thickness().map(f32::to_bits),
            Some((50.0_f32 / 1000.0).to_bits())
        );

        // An explicit family always wins over derivation.
        let explicit = library
            .register_face(
                Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF),
                0,
                "Custom",
                FontStyle::default(),
            )
            .unwrap();
        assert_eq!(library.get(explicit).unwrap().family(), "Custom");
    }

    #[test]
    fn identifiers_from_another_library_never_resolve_even_in_bounds() {
        let mut first = FontLibrary::new();
        let mut second = FontLibrary::new();
        let first_id = first
            .register_font(Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF))
            .unwrap();
        let second_id = second
            .register_font(Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF))
            .unwrap();

        // Slot equality is deliberate: identical bytes and options stay
        // bit-identical across libraries.
        assert_eq!(first_id, second_id);
        assert_eq!(first_id.get(), second_id.get());
        assert!(!first_id.same_provenance(second_id));
        assert!(first_id.same_provenance(first_id));
        assert_eq!(format!("{first_id:?}"), "FontId(0)");

        // Lookups check provenance: the in-bounds foreign identifier fails.
        assert!(first.get(first_id).is_some());
        assert!(first.get(second_id).is_none());
        assert!(second.get(first_id).is_none());
        assert_eq!(
            first
                .set_primary(second_id)
                .expect_err("a foreign identifier must be rejected")
                .code(),
            "font.unknown-id"
        );
        assert_eq!(
            first
                .set_fallback_order([second_id])
                .expect_err("a foreign identifier must be rejected")
                .code(),
            "font.unknown-id"
        );

        // A clone shares provenance with its source.
        let cloned = first.clone();
        assert!(cloned.get(first_id).is_some());
    }

    #[test]
    fn fallback_order_deduplicates_and_appends_every_omitted_face() {
        let mut library = fixture_library();
        library
            .set_fallback_order([test_id(2), test_id(2)])
            .unwrap();
        assert_eq!(library.fallback_order, [test_id(2), test_id(0), test_id(1)]);
    }

    #[test]
    fn candidate_matching_prefers_style_without_repeating_primary_or_fallback() {
        let library = fixture_library();
        let requested = FontStyle::new(500, 90, FontSlant::Italic);
        assert_eq!(
            library.ordered_candidates(&["family".into(), "FAMILY".into()], requested),
            [test_id(1), test_id(0), test_id(2)]
        );
        assert_eq!(
            library.ordered_candidates(&["Fallback".into()], FontStyle::default()),
            [test_id(2), test_id(0), test_id(1)]
        );
        assert_eq!(
            library.ordered_candidates(&[], FontStyle::default()),
            [test_id(0), test_id(1), test_id(2)]
        );
    }

    #[test]
    fn resource_equality_includes_every_rendering_field() {
        let base = resource(0, 0, "Family", FontStyle::default());
        let mut changed = base.clone();
        assert_eq!(base, changed);

        changed.id = test_id(1);
        assert_ne!(base, changed);
        changed = base.clone();
        changed.face_index = 1;
        assert_ne!(base, changed);
        changed = base.clone();
        changed.bytes = Arc::from(font_test_data::TINOS_SUBSET);
        assert_ne!(base, changed);
        changed = base.clone();
        changed.family = "Other".into();
        assert_ne!(base, changed);
        changed = base.clone();
        changed.style = FontStyle::new(700, 80, FontSlant::Italic);
        assert_ne!(base, changed);
        changed = base.clone();
        changed.default_variations.push(
            FontVariation::try_new(crate::OpenTypeTag::try_new("wght").unwrap(), 650.0).unwrap(),
        );
        assert_ne!(base, changed);
        changed = base.clone();
        changed.synthesis = FontSynthesis {
            embolden: true,
            skew: 14 * 64,
        };
        assert_ne!(base, changed);
    }

    #[test]
    fn synthesis_accessors_preserve_fixed_renderer_state() {
        let synthesis = FontSynthesis {
            embolden: true,
            skew: 14 * 64,
        };
        assert!(synthesis.embolden());
        assert_eq!(synthesis.skew(), Some(14.0));
        assert_eq!(synthesis.skew_26_6(), Some(14 * 64));
        assert!(!synthesis.is_empty());
        assert!(FontSynthesis::default().is_empty());

        let embolden_only = FontSynthesis {
            embolden: true,
            skew: 0,
        };
        assert!(embolden_only.embolden());
        assert!(!embolden_only.is_empty());
        assert_eq!(embolden_only.skew_26_6(), None);

        let skew_only = FontSynthesis {
            embolden: false,
            skew: 14 * 64,
        };
        assert!(!skew_only.embolden());
        assert!(!skew_only.is_empty());
        assert_eq!(skew_only.skew_26_6(), Some(14 * 64));
    }

    #[cfg(feature = "system-fonts")]
    #[test]
    fn fontique_attributes_receive_weight_width_and_slant_without_loss() {
        let attributes = system_attributes(FontStyle::new(625, 87, FontSlant::Oblique));
        assert_eq!(attributes.weight.value().to_bits(), 625.0_f32.to_bits());
        assert_eq!(attributes.width.percentage().to_bits(), 87.0_f32.to_bits());
        assert_eq!(attributes.style, fontique::FontStyle::Oblique(None));

        let italic = system_attributes(FontStyle::new(400, 100, FontSlant::Italic));
        assert_eq!(italic.style, fontique::FontStyle::Italic);
        let normal = system_attributes(FontStyle::default());
        assert_eq!(normal.style, fontique::FontStyle::Normal);
    }
}
