// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Minimal, total readers for the SFNT tables the facade consumes directly.
//!
//! HarfRust exposes raw table bytes but not parsed `name` or metrics views, so
//! these readers exist to keep the dependency surface closed. Every read is
//! bounds-checked and every arithmetic step is explicit: font bytes are
//! attacker-controlled threat-model input, so a malformed table must yield
//! `None`, never a panic.

/// Windows platform identifier in the `name` table.
const PLATFORM_WINDOWS: u16 = 3;
/// Unicode platform identifier in the `name` table.
const PLATFORM_UNICODE: u16 = 0;
/// Macintosh platform identifier in the `name` table.
const PLATFORM_MACINTOSH: u16 = 1;
/// Windows US-English language identifier.
const LANGUAGE_WINDOWS_ENGLISH_US: u16 = 0x0409;
/// Typographic family name identifier.
const NAME_TYPOGRAPHIC_FAMILY: u16 = 16;
/// Legacy family name identifier.
const NAME_FAMILY: u16 = 1;

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    let high = *data.get(offset)?;
    let low = *data.get(offset.checked_add(1)?)?;
    Some(u16::from_be_bytes([high, low]))
}

fn read_i16(data: &[u8], offset: usize) -> Option<i16> {
    read_u16(data, offset).map(|value| i16::from_be_bytes(value.to_be_bytes()))
}

fn decode_utf16_be(bytes: &[u8]) -> Option<String> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let units = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_be_bytes(*pair));
    char::decode_utf16(units)
        .collect::<Result<String, _>>()
        .ok()
}

fn decode_ascii(bytes: &[u8]) -> Option<String> {
    if bytes.iter().all(u8::is_ascii) {
        String::from_utf8(bytes.to_vec()).ok()
    } else {
        None
    }
}

/// How preferable one `name` record is; lower ranks win.
fn record_rank(name_id: u16, platform: u16, language: u16) -> Option<(u8, u8)> {
    let name_rank = match name_id {
        NAME_TYPOGRAPHIC_FAMILY => 0,
        NAME_FAMILY => 1,
        _ => return None,
    };
    let platform_rank = match platform {
        PLATFORM_WINDOWS if language == LANGUAGE_WINDOWS_ENGLISH_US => 0,
        PLATFORM_WINDOWS => 1,
        PLATFORM_UNICODE => 2,
        PLATFORM_MACINTOSH => 3,
        _ => return None,
    };
    Some((name_rank, platform_rank))
}

/// Extract a family name from raw `name` table bytes.
///
/// Preference order: typographic family (ID 16) over legacy family (ID 1);
/// within one ID, Windows US-English, then any Windows language, then the
/// Unicode platform (all UTF-16BE), then Macintosh Roman restricted to ASCII.
/// Empty and whitespace-only candidates are rejected.
pub(crate) fn family_from_name_table(data: &[u8]) -> Option<String> {
    let count = read_u16(data, 2)?;
    let storage_offset = usize::from(read_u16(data, 4)?);
    let mut best: Option<((u8, u8), String)> = None;
    for index in 0..usize::from(count) {
        let record = index.checked_mul(12)?.checked_add(6)?;
        let platform = read_u16(data, record)?;
        let encoding = read_u16(data, record.checked_add(2)?)?;
        let language = read_u16(data, record.checked_add(4)?)?;
        let name_id = read_u16(data, record.checked_add(6)?)?;
        let length = usize::from(read_u16(data, record.checked_add(8)?)?);
        let offset = usize::from(read_u16(data, record.checked_add(10)?)?);
        let Some(rank) = record_rank(name_id, platform, language) else {
            continue;
        };
        if best.as_ref().is_some_and(|(kept, _)| *kept <= rank) {
            continue;
        }
        let start = storage_offset.checked_add(offset)?;
        let end = start.checked_add(length)?;
        let Some(bytes) = data.get(start..end) else {
            continue;
        };
        let decoded = match platform {
            PLATFORM_WINDOWS | PLATFORM_UNICODE => decode_utf16_be(bytes),
            PLATFORM_MACINTOSH if encoding == 0 => decode_ascii(bytes),
            _ => None,
        };
        let Some(candidate) = decoded else {
            continue;
        };
        if candidate.trim().is_empty() {
            continue;
        }
        best = Some((rank, candidate));
    }
    best.map(|(_, family)| family)
}

/// Em-relative design metrics assembled from `head`, `hhea`, `OS/2`, and `post`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RawMetrics {
    pub(crate) units_per_em: u16,
    pub(crate) ascent: i16,
    pub(crate) descent: i16,
    pub(crate) line_gap: i16,
    pub(crate) x_height: Option<i16>,
    pub(crate) cap_height: Option<i16>,
    pub(crate) underline_position: Option<i16>,
    pub(crate) underline_thickness: Option<i16>,
}

/// Assemble design metrics from raw table bytes.
///
/// `head` and `hhea` are required; `OS/2` typographic values replace the
/// `hhea` ascent/descent/line-gap when present, and `OS/2` version 2 or later
/// additionally supplies x-height and cap height. `post` supplies the
/// underline geometry. A zero `unitsPerEm` is rejected because every consumer
/// divides by it.
pub(crate) fn metrics_from_tables(
    head: Option<&[u8]>,
    hhea: Option<&[u8]>,
    os2: Option<&[u8]>,
    post: Option<&[u8]>,
) -> Option<RawMetrics> {
    let units_per_em = read_u16(head?, 18)?;
    if units_per_em == 0 {
        return None;
    }
    let hhea = hhea?;
    let mut metrics = RawMetrics {
        units_per_em,
        ascent: read_i16(hhea, 4)?,
        descent: read_i16(hhea, 6)?,
        line_gap: read_i16(hhea, 8)?,
        x_height: None,
        cap_height: None,
        underline_position: None,
        underline_thickness: None,
    };
    if let Some(os2) = os2
        && let (Some(version), Some(ascent), Some(descent), Some(line_gap)) = (
            read_u16(os2, 0),
            read_i16(os2, 68),
            read_i16(os2, 70),
            read_i16(os2, 72),
        )
    {
        metrics.ascent = ascent;
        metrics.descent = descent;
        metrics.line_gap = line_gap;
        if version >= 2 {
            metrics.x_height = read_i16(os2, 86);
            metrics.cap_height = read_i16(os2, 88);
        }
    }
    if let Some(post) = post {
        metrics.underline_position = read_i16(post, 8);
        metrics.underline_thickness = read_i16(post, 10);
    }
    Some(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name_table(records: &[(u16, u16, u16, u16, &[u8])]) -> Vec<u8> {
        let mut header = Vec::new();
        header.extend_from_slice(&0_u16.to_be_bytes());
        header.extend_from_slice(&u16::try_from(records.len()).unwrap().to_be_bytes());
        let storage = records
            .len()
            .checked_mul(12)
            .and_then(|r| r.checked_add(6))
            .unwrap();
        header.extend_from_slice(&u16::try_from(storage).unwrap().to_be_bytes());
        let mut storage_bytes = Vec::new();
        for (platform, encoding, language, name_id, bytes) in records {
            header.extend_from_slice(&platform.to_be_bytes());
            header.extend_from_slice(&encoding.to_be_bytes());
            header.extend_from_slice(&language.to_be_bytes());
            header.extend_from_slice(&name_id.to_be_bytes());
            header.extend_from_slice(&u16::try_from(bytes.len()).unwrap().to_be_bytes());
            header.extend_from_slice(&u16::try_from(storage_bytes.len()).unwrap().to_be_bytes());
            storage_bytes.extend_from_slice(bytes);
        }
        header.extend_from_slice(&storage_bytes);
        header
    }

    fn utf16(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_be_bytes).collect()
    }

    #[test]
    fn family_prefers_typographic_windows_english_and_falls_back_by_rank() {
        let windows_english = name_table(&[
            (3, 1, 0x0409, 1, &utf16("Legacy")),
            (3, 1, 0x0409, 16, &utf16("Typographic")),
        ]);
        assert_eq!(
            family_from_name_table(&windows_english).as_deref(),
            Some("Typographic")
        );

        let windows_japanese = name_table(&[(3, 1, 0x0411, 16, &utf16("日本語ファミリ"))]);
        assert_eq!(
            family_from_name_table(&windows_japanese).as_deref(),
            Some("日本語ファミリ")
        );

        let unicode_only = name_table(&[(0, 3, 0, 1, &utf16("Unicode Family"))]);
        assert_eq!(
            family_from_name_table(&unicode_only).as_deref(),
            Some("Unicode Family")
        );

        let mac_roman = name_table(&[(1, 0, 0, 1, b"Mac Family")]);
        assert_eq!(
            family_from_name_table(&mac_roman).as_deref(),
            Some("Mac Family")
        );

        let ranked = name_table(&[
            (1, 0, 0, 16, b"Mac Typographic"),
            (3, 1, 0x0411, 1, &utf16("Windows Legacy")),
            (0, 3, 0, 16, &utf16("Unicode Typographic")),
        ]);
        assert_eq!(
            family_from_name_table(&ranked).as_deref(),
            Some("Unicode Typographic")
        );

        // US-English outranks another Windows language for the same name ID,
        // even when the other language is recorded first.
        let languages = name_table(&[
            (3, 1, 0x0411, 16, &utf16("日本語名")),
            (3, 1, 0x0409, 16, &utf16("English Name")),
        ]);
        assert_eq!(
            family_from_name_table(&languages).as_deref(),
            Some("English Name")
        );
        assert_eq!(
            record_rank(
                NAME_TYPOGRAPHIC_FAMILY,
                PLATFORM_WINDOWS,
                LANGUAGE_WINDOWS_ENGLISH_US
            ),
            Some((0, 0))
        );
        assert_eq!(
            record_rank(NAME_TYPOGRAPHIC_FAMILY, PLATFORM_WINDOWS, 0x0411),
            Some((0, 1))
        );
        assert_eq!(record_rank(NAME_FAMILY, PLATFORM_UNICODE, 0), Some((1, 2)));
        assert_eq!(
            record_rank(NAME_FAMILY, PLATFORM_MACINTOSH, 0),
            Some((1, 3))
        );
        assert_eq!(
            record_rank(2, PLATFORM_WINDOWS, LANGUAGE_WINDOWS_ENGLISH_US),
            None
        );
        assert_eq!(record_rank(NAME_FAMILY, 7, 0), None);
    }

    #[test]
    fn family_rejects_every_malformed_or_unusable_record() {
        assert_eq!(family_from_name_table(&[]), None);
        assert_eq!(family_from_name_table(&[0, 0, 0]), None);

        // Record count larger than the table.
        let mut truncated = name_table(&[(3, 1, 0x0409, 16, &utf16("Family"))]);
        truncated[3] = 9;
        assert_eq!(family_from_name_table(&truncated), None);

        // String storage out of bounds.
        let mut out_of_bounds = name_table(&[(3, 1, 0x0409, 16, &utf16("Family"))]);
        let length_offset = 6 + 8;
        out_of_bounds[length_offset] = 0xff;
        assert_eq!(family_from_name_table(&out_of_bounds), None);

        // Odd UTF-16 payload, whitespace-only, unpaired surrogate, non-ASCII
        // Mac Roman, unknown platform, and irrelevant name IDs.
        assert_eq!(
            family_from_name_table(&name_table(&[(3, 1, 0x0409, 16, b"odd")])),
            None
        );
        assert_eq!(
            family_from_name_table(&name_table(&[(3, 1, 0x0409, 16, &utf16("  \t "))])),
            None
        );
        assert_eq!(
            family_from_name_table(&name_table(&[(3, 1, 0x0409, 16, &[0xd8, 0x00])])),
            None
        );
        assert_eq!(
            family_from_name_table(&name_table(&[(1, 0, 0, 16, &[0x83, 0x41])])),
            None
        );
        assert_eq!(
            family_from_name_table(&name_table(&[(1, 1, 0, 16, b"Mac Kanji")])),
            None
        );
        assert_eq!(
            family_from_name_table(&name_table(&[(7, 0, 0, 16, b"Custom")])),
            None
        );
        assert_eq!(
            family_from_name_table(&name_table(&[(3, 1, 0x0409, 2, &utf16("Subfamily"))])),
            None
        );

        // A later, lower-ranked record must not replace an earlier winner.
        let keeps_winner = name_table(&[
            (3, 1, 0x0409, 16, &utf16("Winner")),
            (3, 1, 0x0409, 16, &utf16("Duplicate")),
            (0, 3, 0, 16, &utf16("Unicode")),
        ]);
        assert_eq!(
            family_from_name_table(&keeps_winner).as_deref(),
            Some("Winner")
        );
    }

    fn head_table(units_per_em: u16) -> Vec<u8> {
        let mut head = vec![0_u8; 54];
        head[18..20].copy_from_slice(&units_per_em.to_be_bytes());
        head
    }

    fn hhea_table(ascent: i16, descent: i16, line_gap: i16) -> Vec<u8> {
        let mut hhea = vec![0_u8; 36];
        hhea[4..6].copy_from_slice(&ascent.to_be_bytes());
        hhea[6..8].copy_from_slice(&descent.to_be_bytes());
        hhea[8..10].copy_from_slice(&line_gap.to_be_bytes());
        hhea
    }

    fn os2_table(version: u16, typo: (i16, i16, i16), heights: Option<(i16, i16)>) -> Vec<u8> {
        let mut os2 = vec![0_u8; 96];
        os2[0..2].copy_from_slice(&version.to_be_bytes());
        os2[68..70].copy_from_slice(&typo.0.to_be_bytes());
        os2[70..72].copy_from_slice(&typo.1.to_be_bytes());
        os2[72..74].copy_from_slice(&typo.2.to_be_bytes());
        if let Some((x_height, cap_height)) = heights {
            os2[86..88].copy_from_slice(&x_height.to_be_bytes());
            os2[88..90].copy_from_slice(&cap_height.to_be_bytes());
        }
        os2
    }

    fn post_table(position: i16, thickness: i16) -> Vec<u8> {
        let mut post = vec![0_u8; 32];
        post[8..10].copy_from_slice(&position.to_be_bytes());
        post[10..12].copy_from_slice(&thickness.to_be_bytes());
        post
    }

    #[test]
    fn metrics_prefer_os2_typo_values_and_require_head_and_hhea() {
        let head = head_table(1000);
        let hhea = hhea_table(800, -200, 90);
        let os2 = os2_table(4, (760, -240, 0), Some((520, 700)));
        let post = post_table(-100, 50);

        let full = metrics_from_tables(Some(&head), Some(&hhea), Some(&os2), Some(&post)).unwrap();
        assert_eq!(
            full,
            RawMetrics {
                units_per_em: 1000,
                ascent: 760,
                descent: -240,
                line_gap: 0,
                x_height: Some(520),
                cap_height: Some(700),
                underline_position: Some(-100),
                underline_thickness: Some(50),
            }
        );

        let hhea_only = metrics_from_tables(Some(&head), Some(&hhea), None, None).unwrap();
        assert_eq!(
            hhea_only,
            RawMetrics {
                units_per_em: 1000,
                ascent: 800,
                descent: -200,
                line_gap: 90,
                x_height: None,
                cap_height: None,
                underline_position: None,
                underline_thickness: None,
            }
        );

        let old_os2 = os2_table(1, (750, -250, 10), Some((999, 999)));
        let v1 = metrics_from_tables(Some(&head), Some(&hhea), Some(&old_os2), None).unwrap();
        assert_eq!((v1.ascent, v1.descent, v1.line_gap), (750, -250, 10));
        assert_eq!((v1.x_height, v1.cap_height), (None, None));

        // A short OS/2 leaves the hhea values in place instead of failing.
        let short_os2 = os2_table(4, (760, -240, 0), None)[..60].to_vec();
        let short = metrics_from_tables(Some(&head), Some(&hhea), Some(&short_os2), None).unwrap();
        assert_eq!((short.ascent, short.descent), (800, -200));

        assert_eq!(metrics_from_tables(None, Some(&hhea), None, None), None);
        assert_eq!(metrics_from_tables(Some(&head), None, None, None), None);
        assert_eq!(
            metrics_from_tables(Some(&head_table(0)), Some(&hhea), None, None),
            None
        );
        assert_eq!(
            metrics_from_tables(Some(&head[..10]), Some(&hhea), None, None),
            None
        );
        assert_eq!(
            metrics_from_tables(Some(&head), Some(&hhea[..6]), None, None),
            None
        );

        // A truncated post table simply omits underline geometry.
        let no_post =
            metrics_from_tables(Some(&head), Some(&hhea), None, Some(&post[..6])).unwrap();
        assert_eq!(no_post.underline_position, None);
        assert_eq!(no_post.underline_thickness, None);
    }
}
