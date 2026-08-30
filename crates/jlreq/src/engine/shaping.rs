// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

fn aggregate_run(
    source: &str,
    range: Range<usize>,
    glyphs: Vec<RawGlyph>,
    style: &EffectiveStyle,
    bidi_level: u8,
    direction: Direction,
    variations: &Arc<[FontVariation]>,
) -> Vec<PreparedCluster> {
    let mut starts: Vec<_> = glyphs
        .iter()
        .map(|glyph| glyph.cluster)
        .filter(|start| range.contains(start))
        .collect();
    starts.push(range.start);
    starts.sort_unstable();
    starts.dedup();
    let mut result = Vec::with_capacity(starts.len());
    for (ordinal, start) in starts.iter().copied().enumerate() {
        let end = starts
            .get(ordinal.saturating_add(1))
            .copied()
            .unwrap_or(range.end);
        let piece = &source[start..end];
        result.push(PreparedCluster {
            range: start..end,
            advance: 0,
            size: style.size,
            frame: frame_for(piece),
            role: classify_role(source, start..end, style.role),
            bidi_level,
            variations: Arc::clone(variations),
            glyphs: Vec::new(),
        });
    }
    for glyph in glyphs {
        let Ok(bucket) = starts.binary_search(&glyph.cluster) else {
            continue;
        };
        if let Some(cluster) = result.get_mut(bucket) {
            cluster.advance = cluster
                .advance
                .saturating_add(glyph.inline_advance(direction));
            cluster.glyphs.push(glyph);
        }
    }
    result
}

fn frame_for(piece: &str) -> jlreq_core::Frame {
    if piece.chars().count() == 1
        && piece
            .chars()
            .next()
            .is_some_and(|character| is_japanese(character) || is_emoji(character))
    {
        jlreq_core::Frame::FullEm
    } else {
        jlreq_core::Frame::Proportional
    }
}

fn classify_role(
    source: &str,
    range: Range<usize>,
    asserted: TextRole,
) -> Option<jlreq_core::ClusterRole> {
    if asserted != TextRole::Text {
        return Some(asserted.core());
    }
    let mut characters = source[range.clone()].chars();
    let character = characters.next()?;
    if characters.next().is_some() {
        return None;
    }
    let previous = source[..range.start].chars().next_back();
    let next = source[range.end..].chars().next();
    if matches!(character, '.' | '．' | '・')
        && previous.is_some_and(char::is_numeric)
        && next.is_some_and(char::is_numeric)
    {
        return Some(jlreq_core::ClusterRole::DecimalPoint);
    }
    if matches!(character, ',' | '，' | '、')
        && previous.is_some_and(char::is_numeric)
        && next.is_some_and(char::is_numeric)
    {
        return Some(jlreq_core::ClusterRole::DigitGroupSeparator);
    }
    if matches!(character, '!' | '?' | '！' | '？') {
        return Some(if source[range.end..].trim().is_empty() {
            jlreq_core::ClusterRole::SentenceTerminator
        } else {
            jlreq_core::ClusterRole::SentenceMedial
        });
    }
    None
}

fn script_class(text: &str) -> ScriptClass {
    for character in text.chars() {
        let value = character as u32;
        if is_japanese(character) {
            return ScriptClass::Japanese;
        }
        if is_emoji(character) {
            return ScriptClass::Emoji;
        }
        if matches!(value, 0x0590..=0x08ff | 0xfb1d..=0xfdff | 0xfe70..=0xfeff) {
            return ScriptClass::Rtl;
        }
        if character.is_ascii_alphanumeric() || matches!(value, 0x00c0..=0x024f | 0x1e00..=0x1eff) {
            return ScriptClass::Latin;
        }
    }
    ScriptClass::Other
}

fn is_japanese(character: char) -> bool {
    matches!(
        character as u32,
        0x2e80..=0x2fff
            | 0x3000..=0x30ff
            | 0x31f0..=0x31ff
            | 0x3400..=0x4dbf
            | 0x4e00..=0x9fff
            | 0xf900..=0xfaff
            | 0x20000..=0x323af
    )
}

fn is_emoji(character: char) -> bool {
    matches!(character as u32, 0x1f000..=0x1faff | 0x2600..=0x27bf)
}

fn shape_direction(mode: WritingMode, level: Level, script: ScriptClass) -> Direction {
    if mode == WritingMode::VerticalRl
        && matches!(script, ScriptClass::Japanese | ScriptClass::Emoji)
    {
        Direction::TopToBottom
    } else if level.is_rtl() {
        Direction::RightToLeft
    } else {
        Direction::LeftToRight
    }
}
