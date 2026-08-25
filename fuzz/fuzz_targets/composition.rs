// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]

use std::hint::black_box;

use jlreq::{
    Alignment, Break, Cluster, Composer, CompositionLimits, Construct, Frame, Paragraph, Ruby,
    RubyKind, RubyRun, ShapedText, Size, Style, TabAlignment, TabStop, Widow, WritingMode,
};
use libfuzzer_sys::fuzz_target;

fn byte(data: &[u8], index: usize) -> u8 {
    data[index % data.len()]
}

fn annotation(source: &str) -> ShapedText {
    let clusters = source.char_indices().map(|(start, character)| {
        Cluster::new(start..start.saturating_add(character.len_utf8()), 500)
    });
    ShapedText::new(
        source,
        Size::square(500).unwrap_or_else(|_| unreachable!()),
        Frame::FullEm,
        clusters,
    )
    .unwrap_or_else(|_| unreachable!())
}

fn structure(selector: u8, base_end: usize) -> Construct {
    let range = 0..base_end;
    match selector % 9 {
        0 => {
            let reading = annotation("かな");
            let run = RubyRun::new(range.clone(), 0..reading.source().len());
            let ruby = Ruby::new(RubyKind::Group, range, reading, [run])
                .unwrap_or_else(|_| unreachable!());
            Construct::ruby(ruby)
        },
        1 => Construct::tate_chu_yoko(range),
        2 => Construct::emphasis_dots(range, '・'),
        3 => Construct::warichu(range),
        4 => Construct::furawake(range, 1, 0),
        5 => Construct::jidori(range, 2),
        6 => Construct::reference_mark(range, annotation("※")),
        7 => Construct::script(range, annotation("注")),
        _ => Construct::formula(range),
    }
}

fn style(selector: u8) -> Style {
    match selector % 5 {
        0 => Style::jlreq_2020(),
        1 => Style::book_2020(),
        2 => Style::magazine_2020(),
        3 => Style::newspaper_2020(),
        _ => Style::jis_reading_2020(),
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let repeated = usize::from(byte(data, 1) % 64).saturating_add(1);
    let source = match byte(data, 0) % 4 {
        0 => "日本Latin、組版。".repeat(repeated),
        1 => "12\t34".to_owned(),
        2 => String::from_utf8_lossy(data).into_owned(),
        _ => "零幅病的入力".repeat(repeated),
    };
    if source.is_empty() {
        return;
    }

    let advance = match byte(data, 2) % 5 {
        0 => 0,
        1 => 1,
        2 => 500,
        3 => 1_000,
        _ => i32::MAX,
    };
    let clusters = source
        .char_indices()
        .map(|(start, character)| {
            Cluster::new(start..start.saturating_add(character.len_utf8()), advance)
        })
        .collect::<Vec<_>>();
    let base_end = clusters[0].range().end;
    let text = ShapedText::new(
        source.clone(),
        Size::square(match byte(data, 3) % 3 {
            0 => 1,
            1 => 1_000,
            _ => i32::MAX,
        })
        .unwrap_or_else(|_| unreachable!()),
        if byte(data, 4) & 1 == 0 {
            Frame::FullEm
        } else {
            Frame::Proportional
        },
        clusters,
    )
    .unwrap_or_else(|_| unreachable!());

    let use_structure = byte(data, 5) & 1 != 0;
    let breaks = if use_structure {
        Vec::new()
    } else {
        source
            .char_indices()
            .skip(1)
            .map(|(offset, _)| {
                if byte(data, offset) % 17 == 0 {
                    Break::mandatory(offset)
                } else if byte(data, offset) & 1 == 0 {
                    Break::allowed(offset)
                } else {
                    Break::discretionary(offset)
                }
            })
            .collect()
    };
    let constructs = use_structure
        .then(|| structure(byte(data, 6), base_end))
        .into_iter();
    let tabs = source.contains('\t').then(|| {
        TabStop::new(2_000, TabAlignment::Character('.')).unwrap_or_else(|_| unreachable!())
    });
    let Ok(paragraph) = Paragraph::builder(
        text,
        match byte(data, 7) % 4 {
            0 => 1,
            1 => 1_000,
            2 => 20_000,
            _ => i32::MAX,
        },
    )
    .breaks(breaks)
    .constructs(constructs)
    .tab_stops(tabs)
    .first_line_indent(if byte(data, 8) & 1 == 0 { 0 } else { i32::MAX })
    .alignment(match byte(data, 9) % 4 {
        0 => Alignment::Start,
        1 => Alignment::Center,
        2 => Alignment::End,
        _ => Alignment::Justify,
    })
    .widow(Widow::MinimumClusters(u16::from(byte(data, 10))))
    .writing_mode(if byte(data, 11) & 1 == 0 {
        WritingMode::HorizontalTb
    } else {
        WritingMode::VerticalRl
    })
    .build() else {
        return;
    };

    let limits = if byte(data, 12) & 1 == 0 {
        CompositionLimits::default()
    } else {
        CompositionLimits::default().with_max_search_transitions(usize::from(byte(data, 13)))
    };
    let mut composer = Composer::with_limits(limits);
    match composer.compose(&paragraph, &style(byte(data, 14))) {
        Ok(layout) => {
            for line in layout.lines() {
                black_box((line.range(), line.clusters(), line.attachments()));
            }
            black_box(layout.diagnostics());
        },
        Err(error) => {
            black_box((
                error.code(),
                error.resource(),
                error.limit(),
                error.observed(),
            ));
        },
    }
});
