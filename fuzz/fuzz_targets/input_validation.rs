// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]

use std::hint::black_box;

use jlreq::{
    Alignment, Break, Cluster, Construct, Frame, Paragraph, Ruby, RubyKind, RubyRun, ShapedText,
    Size, Style, TabAlignment, TabStop, Widow, WritingMode,
};
use libfuzzer_sys::fuzz_target;

const SOURCES: &[&str] = &[
    "",
    "日",
    "日本語",
    "\u{31f7}\u{309a}",
    "ffi",
    "A\tB",
    "a=b+c",
    "「日」、・。",
    "１２Ａ",
];

fn take(data: &[u8], cursor: &mut usize) -> u8 {
    let value = data[*cursor % data.len()];
    *cursor = cursor.saturating_add(1);
    value
}

fn take_i32(data: &[u8], cursor: &mut usize) -> i32 {
    i32::from_le_bytes([
        take(data, cursor),
        take(data, cursor),
        take(data, cursor),
        take(data, cursor),
    ])
}

fn frame(selector: u8) -> Frame {
    match selector % 3 {
        0 => Frame::FullEm,
        1 => Frame::Proportional,
        _ => Frame::HalfEm,
    }
}

fn valid_advance(data: &[u8], cursor: &mut usize) -> i32 {
    match take(data, cursor) % 5 {
        0 => 0,
        1 => 1,
        2 => 500,
        3 => 1_000,
        _ => i32::MAX,
    }
}

fn valid_clusters(source: &str, data: &[u8], cursor: &mut usize) -> Vec<Cluster> {
    source
        .char_indices()
        .map(|(start, character)| {
            Cluster::new(
                start..start.saturating_add(character.len_utf8()),
                valid_advance(data, cursor),
            )
        })
        .collect()
}

fn raw_clusters(source: &str, data: &[u8], cursor: &mut usize) -> Vec<Cluster> {
    let count = usize::from(take(data, cursor) % 8);
    let modulus = source.len().saturating_add(2);
    (0..count)
        .map(|_| {
            let start = usize::from(take(data, cursor)) % modulus;
            let end = usize::from(take(data, cursor)) % modulus;
            Cluster::new(start..end, take_i32(data, cursor)).with_frame(frame(take(data, cursor)))
        })
        .collect()
}

fn annotation() -> Option<ShapedText> {
    let size = Size::square(500).ok()?;
    ShapedText::new(
        "注",
        size,
        Frame::FullEm,
        [Cluster::new(0.."注".len(), 500)],
    )
    .ok()
}

fn arbitrary_range(source_len: usize, data: &[u8], cursor: &mut usize) -> std::ops::Range<usize> {
    let modulus = source_len.saturating_add(2);
    let start = usize::from(take(data, cursor)) % modulus;
    let end = usize::from(take(data, cursor)) % modulus;
    start..end
}

fn construct(source_len: usize, data: &[u8], cursor: &mut usize) -> Option<Construct> {
    let selector = take(data, cursor) % 9;
    let range = arbitrary_range(source_len, data, cursor);
    match selector {
        0 => {
            let reading = annotation()?;
            let kind = match take(data, cursor) % 3 {
                0 => RubyKind::Mono,
                1 => RubyKind::Group,
                _ => RubyKind::Jukugo,
            };
            let run = RubyRun::new(range.clone(), 0..reading.source().len());
            Ruby::new(kind, range, reading, [run])
                .ok()
                .map(Construct::ruby)
        },
        1 => Some(Construct::tate_chu_yoko(range)),
        2 => Some(Construct::emphasis_dots(range, '・')),
        3 => Some(Construct::warichu(range)),
        4 => Some(Construct::furawake(
            range,
            u16::from(take(data, cursor)),
            take_i32(data, cursor),
        )),
        5 => Some(Construct::jidori(range, u16::from(take(data, cursor)))),
        6 => Some(Construct::reference_mark(range, annotation()?)),
        7 => Some(Construct::script(range, annotation()?)),
        _ => Some(Construct::formula(range)),
    }
}

fn style(selector: u8) -> Style {
    match selector % 6 {
        0 => Style::jlreq_2020(),
        1 => Style::book_2020(),
        2 => Style::magazine_2020(),
        3 => Style::newspaper_2020(),
        4 => Style::jis_reading_2020(),
        _ => Style::default(),
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let mut cursor = 0;
    let source = SOURCES[usize::from(take(data, &mut cursor)) % SOURCES.len()];
    let selected_size = match take(data, &mut cursor) % 4 {
        0 => Size::new(take_i32(data, &mut cursor), take_i32(data, &mut cursor)),
        1 => Size::square(1),
        2 => Size::square(1_000),
        _ => Size::square(i32::MAX),
    };
    let Ok(size) = selected_size else {
        return;
    };
    let selected_frame = frame(take(data, &mut cursor));
    let clusters = if take(data, &mut cursor) & 1 == 0 {
        raw_clusters(source, data, &mut cursor)
    } else {
        valid_clusters(source, data, &mut cursor)
    };
    let Ok(text) = ShapedText::new(source, size, selected_frame, clusters) else {
        return;
    };

    let extent = match take(data, &mut cursor) % 6 {
        0 => -1,
        1 => 0,
        2 => 1,
        3 => 1_000,
        4 => 4_000,
        _ => i32::MAX,
    };
    let mut breaks = Vec::new();
    for _ in 0..usize::from(take(data, &mut cursor) % 7) {
        let offset = usize::from(take(data, &mut cursor)) % source.len().saturating_add(2);
        breaks.push(match take(data, &mut cursor) % 3 {
            0 => Break::allowed(offset),
            1 => Break::mandatory(offset),
            _ => Break::discretionary(offset),
        });
    }

    let mut constructs = Vec::new();
    for _ in 0..usize::from(take(data, &mut cursor) % 3) {
        if let Some(value) = construct(source.len(), data, &mut cursor) {
            constructs.push(value);
        }
    }

    let mut tab_stops = Vec::new();
    for _ in 0..usize::from(take(data, &mut cursor) % 4) {
        let alignment = match take(data, &mut cursor) % 4 {
            0 => TabAlignment::Start,
            1 => TabAlignment::Center,
            2 => TabAlignment::End,
            _ => TabAlignment::Character('.'),
        };
        if let Ok(stop) = TabStop::new(take_i32(data, &mut cursor), alignment) {
            tab_stops.push(stop);
        }
    }

    let alignment = match take(data, &mut cursor) % 4 {
        0 => Alignment::Start,
        1 => Alignment::Center,
        2 => Alignment::End,
        _ => Alignment::Justify,
    };
    let writing_mode = if take(data, &mut cursor) & 1 == 0 {
        WritingMode::HorizontalTb
    } else {
        WritingMode::VerticalRl
    };
    let widow = Widow::MinimumClusters(u16::from(take(data, &mut cursor)));
    let paragraph = Paragraph::builder(text, extent)
        .breaks(breaks)
        .constructs(constructs)
        .tab_stops(tab_stops)
        .first_line_indent(take_i32(data, &mut cursor))
        .alignment(alignment)
        .widow(widow)
        .writing_mode(writing_mode)
        .build();
    let Ok(paragraph) = paragraph else {
        return;
    };

    black_box((
        paragraph.text(),
        paragraph.line_extent(),
        paragraph.breaks(),
        paragraph.constructs(),
        paragraph.tab_stops(),
        paragraph.first_line_indent(),
        paragraph.alignment(),
        paragraph.widow(),
        paragraph.writing_mode(),
        style(take(data, &mut cursor)),
    ));
});
