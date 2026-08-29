// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt;
use std::sync::Arc;

use crate::LayoutError;

/// Stable identifier assigned by a [`FontLibrary`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontId(u32);

impl FontId {
    /// Numeric value suitable for renderer-side maps.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
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

/// Font bytes retained by a completed layout for renderer use.
#[derive(Clone)]
#[non_exhaustive]
pub struct FontResource {
    pub(crate) id: FontId,
    pub(crate) bytes: Arc<[u8]>,
    pub(crate) face_index: u32,
    pub(crate) family: String,
    pub(crate) style: FontStyle,
    pub(crate) shaper_data: Arc<harfrust::ShaperData>,
}

impl PartialEq for FontResource {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.face_index == other.face_index
            && self.bytes == other.bytes
            && self.family == other.family
            && self.style == other.style
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
}

/// Ordered, in-memory font faces used for primary selection and fallback.
#[derive(Clone, Default)]
pub struct FontLibrary {
    fonts: Vec<FontResource>,
    primary: Option<FontId>,
    fallback_order: Vec<FontId>,
}

impl fmt::Debug for FontLibrary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FontLibrary")
            .field("fonts", &self.fonts)
            .field("primary", &self.primary)
            .field("fallback_order", &self.fallback_order)
            .finish()
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
        }
    }

    /// Register face zero with default family/style metadata.
    pub fn register_font<B>(&mut self, bytes: B) -> Result<FontId, LayoutError>
    where
        B: Into<Arc<[u8]>>,
    {
        self.register_face(bytes, 0, "", FontStyle::default())
    }

    /// Alias for [`register_font`](Self::register_font), convenient in builder-style setup.
    pub fn add_font<B>(&mut self, bytes: B) -> Result<FontId, LayoutError>
    where
        B: Into<Arc<[u8]>>,
    {
        self.register_font(bytes)
    }

    /// Register one face from TTF, OTF, or TTC bytes.
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
        let font = harfrust::FontRef::from_index(&bytes, face_index)
            .map_err(|_| LayoutError::invalid_font(face_index))?;
        // Parse shaping-critical tables exactly once. Layout engines retain this immutable
        // cache through the resource instead of reconstructing it on first use.
        let shaper_data = Arc::new(harfrust::ShaperData::new(&font));

        let ordinal = u32::try_from(self.fonts.len())
            .map_err(|_| LayoutError::invalid_document("font.too-many-ids", None))?;
        let id = FontId(ordinal);
        self.fonts.push(FontResource {
            id,
            bytes,
            face_index,
            family: family.into(),
            style,
            shaper_data,
        });
        self.fallback_order.push(id);
        if self.primary.is_none() {
            self.primary = Some(id);
        }
        Ok(id)
    }

    /// Alias for [`register_face`](Self::register_face).
    pub fn add_face<B>(
        &mut self,
        bytes: B,
        face_index: u32,
        family: impl Into<String>,
        style: FontStyle,
    ) -> Result<FontId, LayoutError>
    where
        B: Into<Arc<[u8]>>,
    {
        self.register_face(bytes, face_index, family, style)
    }

    /// Make a registered face the `.notdef` source and first fallback candidate.
    pub fn set_primary(&mut self, id: FontId) -> Result<(), LayoutError> {
        self.get(id)
            .ok_or_else(|| LayoutError::invalid_document("font.unknown-id", None))?;
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
                return Err(LayoutError::invalid_document("font.unknown-id", None));
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
    #[must_use]
    pub fn get(&self, id: FontId) -> Option<&FontResource> {
        self.fonts.get(id.0 as usize).filter(|font| font.id == id)
    }

    #[cfg(feature = "system-fonts")]
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
            query.matches_with(|font| {
                selected = Some((font.blob.as_ref().to_vec(), font.index));
                QueryStatus::Stop
            });
        }
        let (bytes, index) = selected
            .ok_or_else(|| LayoutError::invalid_document("font.system-family-not-found", None))?;
        self.register_face(bytes, index, family, style)
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

    fn resource(id: u32, face_index: u32, family: &str, style: FontStyle) -> FontResource {
        let bytes = Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF);
        let font = harfrust::FontRef::from_index(&bytes, 0).unwrap();
        let shaper_data = Arc::new(harfrust::ShaperData::new(&font));
        FontResource {
            id: FontId(id),
            bytes,
            face_index,
            family: family.to_owned(),
            style,
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
            primary: Some(FontId(0)),
            fallback_order: vec![FontId(0), FontId(1), FontId(2)],
        }
    }

    #[test]
    fn identifiers_resources_and_libraries_have_exact_observable_metadata() {
        assert_eq!(FontId(42).get(), 42);
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
    fn fallback_order_deduplicates_and_appends_every_omitted_face() {
        let mut library = fixture_library();
        library.set_fallback_order([FontId(2), FontId(2)]).unwrap();
        assert_eq!(library.fallback_order, [FontId(2), FontId(0), FontId(1)]);
    }

    #[test]
    fn candidate_matching_prefers_style_without_repeating_primary_or_fallback() {
        let library = fixture_library();
        let requested = FontStyle::new(500, 90, FontSlant::Italic);
        assert_eq!(
            library.ordered_candidates(&["family".into(), "FAMILY".into()], requested),
            [FontId(1), FontId(0), FontId(2)]
        );
        assert_eq!(
            library.ordered_candidates(&["Fallback".into()], FontStyle::default()),
            [FontId(2), FontId(0), FontId(1)]
        );
        assert_eq!(
            library.ordered_candidates(&[], FontStyle::default()),
            [FontId(0), FontId(1), FontId(2)]
        );
    }
}
