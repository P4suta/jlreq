// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reference implementation of the language-independent conformance protocol.

use std::{
    io::{self, BufRead},
    process::ExitCode,
};

use kumihan::{
    Alignment, Break, Cluster, ClusterRole, Construct, CoordinateTransform, Frame, Paragraph, Ruby,
    RubyKind, RubyRun, ShapedText, Size, Style, TabAlignment, TabStop, Widow, WritingMode,
    style::{
        AdjustmentPreference, AmbiguousContext, ExpansionOrder, GroupRubyDistribution,
        GroupedNumeralBeforeWestern, GroupedNumeralQualification, HangingPunctuation,
        IterationMarkAtLineHead, JapaneseLatinExpansionCeiling, JukugoRubyLayout, KinsokuLevel,
        LineEndFullStopComma, LineEndPunctuation, LineHeadOpeningBracket, ReductionTable,
        RelaxationMechanism, Remainder, RubyAlignment, RubyOverhangIndent, RubyOverhangKana,
        SentenceMedialDividingMark, UnlistedCodePoint,
    },
};
use serde_json::{Map, Value, json};

const PROTOCOL: &str = "kumihan.conformance/1";
const SPEC: &str = kumihan::SPECIFICATION;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("kumihan-sample-engine: {error}");
            ExitCode::from(2)
        },
    }
}

fn run() -> Result<(), String> {
    let stdin = io::stdin();
    for (line_index, line) in stdin.lock().lines().enumerate() {
        let line = line.map_err(|error| error.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let line_number = line_index.saturating_add(1);
        let envelope: Value =
            serde_json::from_str(&line).map_err(|error| format!("line {line_number}: {error}"))?;
        let object = object(&envelope, "message")?;
        exact_string(object, "protocol", PROTOCOL)?;
        exact_string(object, "spec", SPEC)?;
        let id = string(object, "id")?;
        let request = object
            .get("request")
            .ok_or_else(|| "request is required".to_owned())?;
        let (paragraph, style) = parse_request(request)?;
        let layout = kumihan::compose(&paragraph, &style);
        println!(
            "{}",
            json!({
                "protocol": PROTOCOL,
                "spec": SPEC,
                "id": id,
                "response": layout_json(&layout),
            })
        );
    }
    Ok(())
}

fn parse_request(value: &Value) -> Result<(Paragraph, Style), String> {
    let request = object(value, "request")?;
    let text = parse_shaped_text(value)?;
    let line_extent = integer(request, "line_extent")?;
    let mut builder = Paragraph::builder(text, line_extent);

    if let Some(values) = request.get("breaks") {
        let mut breaks = Vec::new();
        for value in array(values, "breaks")? {
            let entry = object(value, "break")?;
            let offset = usize_integer(entry, "offset")?;
            let opportunity = match string(entry, "kind")? {
                "allowed" => Break::allowed(offset),
                "mandatory" => Break::mandatory(offset),
                "discretionary" => Break::discretionary(offset),
                other => return Err(format!("unknown break kind {other:?}")),
            };
            breaks.push(opportunity);
        }
        builder = builder.breaks(breaks);
    }
    if let Some(values) = request.get("constructs") {
        let mut constructs = Vec::new();
        for value in array(values, "constructs")? {
            constructs.push(parse_construct(value)?);
        }
        builder = builder.constructs(constructs);
    }
    if let Some(values) = request.get("tab_stops") {
        let mut stops = Vec::new();
        for value in array(values, "tab_stops")? {
            let stop = object(value, "tab stop")?;
            let alignment = match string(stop, "alignment")? {
                "start" => TabAlignment::Start,
                "center" => TabAlignment::Center,
                "end" => TabAlignment::End,
                "character" => {
                    let character = one_char(string(stop, "character")?, "tab character")?;
                    TabAlignment::Character(character)
                },
                other => return Err(format!("unknown tab alignment {other:?}")),
            };
            stops.push(
                TabStop::new(integer(stop, "position")?, alignment)
                    .map_err(|error| error.to_string())?,
            );
        }
        builder = builder.tab_stops(stops);
    }
    if let Some(indent) = request.get("first_line_indent").and_then(Value::as_i64) {
        builder = builder.first_line_indent(to_i32(indent, "first_line_indent")?);
    }
    if let Some(alignment) = request.get("alignment").and_then(Value::as_str) {
        builder = builder.alignment(match alignment {
            "start" => Alignment::Start,
            "center" => Alignment::Center,
            "end" => Alignment::End,
            "justify" => Alignment::Justify,
            other => return Err(format!("unknown alignment {other:?}")),
        });
    }
    if let Some(minimum) = request
        .get("widow_minimum_clusters")
        .and_then(Value::as_u64)
    {
        builder = builder.widow(Widow::MinimumClusters(
            u16::try_from(minimum).map_err(|_| "widow minimum is too large".to_owned())?,
        ));
    }
    if let Some(mode) = request.get("writing_mode").and_then(Value::as_str) {
        builder = builder.writing_mode(parse_writing_mode(mode)?);
    }
    let style = request
        .get("style")
        .map_or_else(|| Ok(Style::default()), parse_style)?;
    Ok((builder.build().map_err(|error| error.to_string())?, style))
}

fn parse_shaped_text(value: &Value) -> Result<ShapedText, String> {
    let shaped = object(value, "shaped text")?;
    let source = string(shaped, "source")?;
    let size = parse_size(
        shaped
            .get("size")
            .ok_or_else(|| "size is required".to_owned())?,
    )?;
    let frame = parse_frame(string(shaped, "frame")?)?;
    let mut clusters = Vec::new();
    for value in array(
        shaped
            .get("clusters")
            .ok_or_else(|| "clusters are required".to_owned())?,
        "clusters",
    )? {
        let entry = object(value, "cluster")?;
        let range = parse_range(
            entry
                .get("range")
                .ok_or_else(|| "cluster range is required".to_owned())?,
        )?;
        let mut cluster = Cluster::new(range, integer(entry, "advance")?);
        if let Some(size) = entry.get("size") {
            cluster = cluster.with_size(parse_size(size)?);
        }
        if let Some(frame) = entry.get("frame").and_then(Value::as_str) {
            cluster = cluster.with_frame(parse_frame(frame)?);
        }
        if let Some(role) = entry.get("role").and_then(Value::as_str) {
            cluster = cluster.with_role(match role {
                "text" => ClusterRole::Text,
                "decimal-point" => ClusterRole::DecimalPoint,
                "digit-group-separator" => ClusterRole::DigitGroupSeparator,
                "grouped-numeral" => ClusterRole::GroupedNumeral,
                "unit-symbol" => ClusterRole::UnitSymbol,
                "formula" => ClusterRole::Formula,
                "warichu-bracket" => ClusterRole::WarichuBracket,
                other => return Err(format!("unknown cluster role {other:?}")),
            });
        }
        clusters.push(cluster);
    }
    ShapedText::new(source, size, frame, clusters).map_err(|error| error.to_string())
}

fn parse_construct(value: &Value) -> Result<Construct, String> {
    let entry = object(value, "construct")?;
    let kind = string(entry, "kind")?;
    let range = || {
        parse_range(
            entry
                .get("range")
                .ok_or_else(|| "construct range is required".to_owned())?,
        )
    };
    match kind {
        "ruby" => {
            let base = range()?;
            let ruby_kind = match string(entry, "ruby_kind")? {
                "mono" => RubyKind::Mono,
                "group" => RubyKind::Group,
                "jukugo" => RubyKind::Jukugo,
                other => return Err(format!("unknown ruby kind {other:?}")),
            };
            let annotation = parse_shaped_text(
                entry
                    .get("annotation")
                    .ok_or_else(|| "ruby annotation is required".to_owned())?,
            )?;
            let mut runs = Vec::new();
            for value in array(
                entry
                    .get("runs")
                    .ok_or_else(|| "ruby runs are required".to_owned())?,
                "ruby runs",
            )? {
                let run = object(value, "ruby run")?;
                runs.push(RubyRun::new(
                    parse_range(
                        run.get("base")
                            .ok_or_else(|| "ruby run base is required".to_owned())?,
                    )?,
                    parse_range(
                        run.get("annotation")
                            .ok_or_else(|| "ruby run annotation is required".to_owned())?,
                    )?,
                ));
            }
            let ruby =
                Ruby::new(ruby_kind, base, annotation, runs).map_err(|error| error.to_string())?;
            Ok(Construct::ruby(ruby))
        },
        "tate-chu-yoko" => Ok(Construct::tate_chu_yoko(range()?)),
        "emphasis-dots" => Ok(Construct::emphasis_dots(
            range()?,
            one_char(string(entry, "mark")?, "emphasis mark")?,
        )),
        "warichu" => Ok(Construct::warichu(range()?)),
        "furawake" => Ok(Construct::furawake(
            range()?,
            u16_integer(entry, "columns")?,
            integer(entry, "line_gap")?,
        )),
        "jidori" => Ok(Construct::jidori(range()?, u16_integer(entry, "cells")?)),
        "reference-mark" => Ok(Construct::reference_mark(
            range()?,
            parse_shaped_text(
                entry
                    .get("annotation")
                    .ok_or_else(|| "reference-mark annotation is required".to_owned())?,
            )?,
        )),
        "script" => Ok(Construct::script(
            range()?,
            parse_shaped_text(
                entry
                    .get("annotation")
                    .ok_or_else(|| "script annotation is required".to_owned())?,
            )?,
        )),
        "formula" => Ok(Construct::formula(range()?)),
        other => Err(format!("unknown construct kind {other:?}")),
    }
}

fn parse_style(value: &Value) -> Result<Style, String> {
    if let Some(profile) = value.as_str() {
        return profile_style(profile);
    }
    let settings = object(value, "style")?;
    let mut builder = settings
        .get("profile")
        .and_then(Value::as_str)
        .map_or_else(
            || Ok(Style::builder()),
            |profile| Ok::<_, String>(profile_style(profile)?.to_builder()),
        )?;

    for (name, value) in settings {
        if name == "profile" {
            continue;
        }
        let value = value
            .as_str()
            .ok_or_else(|| format!("style setting {name:?} must be a string"))?;
        builder = match name.as_str() {
            "kinsoku.level" => builder.kinsoku_level(choice(
                name,
                value,
                &[
                    ("very-loose", KinsokuLevel::VeryLoose),
                    ("loose", KinsokuLevel::Loose),
                    ("strict", KinsokuLevel::Strict),
                    ("very-strict", KinsokuLevel::VeryStrict),
                ],
            )?),
            "adjustment.reduction_table" => builder.reduction_table(choice(
                name,
                value,
                &[
                    ("table-3", ReductionTable::Table3),
                    ("table-4", ReductionTable::Table4),
                    ("table-5", ReductionTable::Table5),
                ],
            )?),
            "spacing.line_end_punctuation" => builder.line_end_punctuation(choice(
                name,
                value,
                &[
                    ("half-em", LineEndPunctuation::HalfEm),
                    ("solid", LineEndPunctuation::Solid),
                ],
            )?),
            "spacing.line_end_full_stop_comma" => builder.line_end_full_stop_comma(choice(
                name,
                value,
                &[
                    ("preferred", LineEndFullStopComma::Preferred),
                    ("jis", LineEndFullStopComma::Jis),
                ],
            )?),
            "spacing.line_head_opening_bracket" => builder.line_head_opening_bracket(choice(
                name,
                value,
                &[
                    ("pattern-1", LineHeadOpeningBracket::Pattern1),
                    ("pattern-2", LineHeadOpeningBracket::Pattern2),
                    ("pattern-3", LineHeadOpeningBracket::Pattern3),
                ],
            )?),
            "ruby.overhang_kana" => builder.ruby_overhang_kana(choice(
                name,
                value,
                &[
                    ("kana", RubyOverhangKana::Kana),
                    ("jis", RubyOverhangKana::Jis),
                    ("any", RubyOverhangKana::Any),
                    ("none", RubyOverhangKana::None),
                ],
            )?),
            "ruby.overhang_indent" => builder.ruby_overhang_indent(choice(
                name,
                value,
                &[
                    ("permitted", RubyOverhangIndent::Permitted),
                    ("prohibited", RubyOverhangIndent::Prohibited),
                ],
            )?),
            "ruby.alignment" => builder.ruby_alignment(choice(
                name,
                value,
                &[
                    ("nakatsuki", RubyAlignment::Nakatsuki),
                    ("katatsuki", RubyAlignment::Katatsuki),
                ],
            )?),
            "ruby.group_distribution" => builder.group_ruby_distribution(choice(
                name,
                value,
                &[
                    ("jis", GroupRubyDistribution::Jis),
                    ("flush", GroupRubyDistribution::Flush),
                ],
            )?),
            "ruby.jukugo_layout" => builder.jukugo_ruby_layout(choice(
                name,
                value,
                &[
                    ("group", JukugoRubyLayout::Group),
                    ("phonetic", JukugoRubyLayout::Phonetic),
                ],
            )?),
            "kinsoku.iteration_mark_at_line_head" => builder.iteration_mark_at_line_head(choice(
                name,
                value,
                &[
                    ("prohibited", IterationMarkAtLineHead::Prohibited),
                    ("permitted", IterationMarkAtLineHead::Permitted),
                    ("replaced", IterationMarkAtLineHead::Replaced),
                ],
            )?),
            "adjustment.hanging_punctuation" => builder.hanging_punctuation(choice(
                name,
                value,
                &[
                    ("none", HangingPunctuation::None),
                    ("hanging", HangingPunctuation::Hanging),
                ],
            )?),
            "kinsoku.grouped_numeral_before_western" => {
                builder.grouped_numeral_before_western(choice(
                    name,
                    value,
                    &[
                        ("breakable", GroupedNumeralBeforeWestern::Breakable),
                        ("unbreakable", GroupedNumeralBeforeWestern::Unbreakable),
                    ],
                )?)
            },
            "spacing.sentence_medial_dividing_mark" => {
                builder.sentence_medial_dividing_mark(choice(
                    name,
                    value,
                    &[
                        ("solid", SentenceMedialDividingMark::Solid),
                        ("quarter-em", SentenceMedialDividingMark::QuarterEm),
                    ],
                )?)
            },
            "adjustment.japanese_latin_expansion_ceiling" => builder
                .japanese_latin_expansion_ceiling(choice(
                    name,
                    value,
                    &[
                        ("half-em", JapaneseLatinExpansionCeiling::HalfEm),
                        ("third-em", JapaneseLatinExpansionCeiling::ThirdEm),
                        ("rigid", JapaneseLatinExpansionCeiling::Rigid),
                    ],
                )?),
            "adjustment.expansion_order" => builder.expansion_order(choice(
                name,
                value,
                &[
                    ("jis", ExpansionOrder::Jis),
                    ("implementation", ExpansionOrder::Implementation),
                ],
            )?),
            "adjustment.preference" => builder.adjustment_preference(choice(
                name,
                value,
                &[
                    ("least-adjustment", AdjustmentPreference::LeastAdjustment),
                    ("even-texture", AdjustmentPreference::EvenTexture),
                ],
            )?),
            "adjustment.remainder" => builder.remainder(choice(
                name,
                value,
                &[
                    ("leading", Remainder::Leading),
                    ("trailing", Remainder::Trailing),
                ],
            )?),
            "classification.unlisted_code_point" => builder.unlisted_code_point(choice(
                name,
                value,
                &[
                    ("by-frame", UnlistedCodePoint::ByFrame),
                    ("ideographic", UnlistedCodePoint::Ideographic),
                ],
            )?),
            "classification.ambiguous_context" => builder.ambiguous_context(choice(
                name,
                value,
                &[
                    ("lowest-class", AmbiguousContext::LowestClass),
                    ("highest-class", AmbiguousContext::HighestClass),
                ],
            )?),
            "classification.grouped_numeral_qualification" => builder
                .grouped_numeral_qualification(choice(
                    name,
                    value,
                    &[
                        ("by-width", GroupedNumeralQualification::ByWidth),
                        ("by-role", GroupedNumeralQualification::ByRole),
                    ],
                )?),
            "kinsoku.relaxation_mechanism" => builder.relaxation_mechanism(choice(
                name,
                value,
                &[
                    ("reclassify", RelaxationMechanism::Reclassify),
                    ("matrix", RelaxationMechanism::Matrix),
                ],
            )?),
            other => return Err(format!("unknown style setting {other:?}")),
        };
    }
    builder.build().map_err(|error| error.to_string())
}

fn profile_style(profile: &str) -> Result<Style, String> {
    match profile {
        "jlreq-2020" => Ok(Style::jlreq_2020()),
        "book-2020" => Ok(Style::book_2020()),
        "magazine-2020" => Ok(Style::magazine_2020()),
        "newspaper-2020" => Ok(Style::newspaper_2020()),
        "jis-reading-2020" => Ok(Style::jis_reading_2020()),
        other => Err(format!("unknown style profile {other:?}")),
    }
}

fn choice<T: Copy>(name: &str, value: &str, choices: &[(&str, T)]) -> Result<T, String> {
    choices
        .iter()
        .find_map(|(candidate, choice)| (*candidate == value).then_some(*choice))
        .ok_or_else(|| format!("unknown value {value:?} for style setting {name:?}"))
}

fn layout_json(layout: &kumihan::Layout) -> Value {
    json!({
        "lines": layout.lines().iter().map(|line| json!({
            "range": [line.range().start, line.range().end],
            "inline_origin": line.inline_origin(),
            "block_origin": line.block_origin(),
            "inline_extent": line.inline_extent(),
            "block_extent": line.block_extent(),
            "clusters": line.clusters().iter().map(|placement| json!({
                "origin": match placement.origin() {
                    kumihan::PlacementOrigin::Cluster(ordinal) => json!({"cluster": ordinal}),
                    kumihan::PlacementOrigin::Construct(ordinal) => json!({"construct": ordinal}),
                    _ => json!({"unknown": true}),
                },
                "range": [placement.range().start, placement.range().end],
                "inline": placement.inline(),
                "block": placement.block(),
                "advance": placement.advance(),
                "size": size_json(placement.size()),
                "frame": frame_name(placement.frame()),
                "writing_mode": writing_mode_name(placement.writing_mode()),
                "transform": transform_name(placement.transform()),
            })).collect::<Vec<_>>(),
            "attachments": line.attachments().iter().map(|attachment| json!({
                "construct": attachment.construct(),
                "range": [attachment.range().start, attachment.range().end],
                "inline": attachment.inline(),
                "block": attachment.block(),
                "advance": attachment.advance(),
                "size": size_json(attachment.size()),
                "writing_mode": writing_mode_name(attachment.writing_mode()),
                "transform": transform_name(attachment.transform()),
                "symbol": attachment.symbol().map(|symbol| symbol.to_string()),
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "diagnostics": layout.diagnostics().iter().map(|diagnostic| json!({
            "code": diagnostic.code(),
            "severity": match diagnostic.severity() {
                kumihan::Severity::Info => "info",
                kumihan::Severity::Warning => "warning",
                kumihan::Severity::Error => "error",
                _ => "unknown",
            },
            "range": diagnostic.range().map(|range| [range.start, range.end]),
            "jlreq": diagnostic.jlreq(),
        })).collect::<Vec<_>>(),
    })
}

fn size_json(size: Size) -> Value {
    json!({"inline": size.inline(), "block": size.block()})
}

fn frame_name(frame: Frame) -> &'static str {
    match frame {
        Frame::FullEm => "full-em",
        Frame::Proportional => "proportional",
        Frame::HalfEm => "half-em",
        _ => "unknown",
    }
}

fn writing_mode_name(mode: WritingMode) -> &'static str {
    match mode {
        WritingMode::HorizontalTb => "horizontal-tb",
        WritingMode::VerticalRl => "vertical-rl",
        _ => "unknown",
    }
}

fn transform_name(transform: CoordinateTransform) -> &'static str {
    match transform {
        CoordinateTransform::Identity => "identity",
        CoordinateTransform::RotateClockwise => "rotate-clockwise",
        CoordinateTransform::TateChuYoko => "tate-chu-yoko",
        _ => "unknown",
    }
}

fn parse_size(value: &Value) -> Result<Size, String> {
    let size = object(value, "size")?;
    Size::new(integer(size, "inline")?, integer(size, "block")?).map_err(|error| error.to_string())
}

fn parse_frame(frame: &str) -> Result<Frame, String> {
    match frame {
        "full-em" => Ok(Frame::FullEm),
        "proportional" => Ok(Frame::Proportional),
        "half-em" => Ok(Frame::HalfEm),
        other => Err(format!("unknown frame {other:?}")),
    }
}

fn parse_writing_mode(mode: &str) -> Result<WritingMode, String> {
    match mode {
        "horizontal-tb" => Ok(WritingMode::HorizontalTb),
        "vertical-rl" => Ok(WritingMode::VerticalRl),
        other => Err(format!("unknown writing mode {other:?}")),
    }
}

fn parse_range(value: &Value) -> Result<std::ops::Range<usize>, String> {
    let values = array(value, "range")?;
    if values.len() != 2 {
        return Err("a range must contain exactly two offsets".to_owned());
    }
    let start = values[0]
        .as_u64()
        .ok_or_else(|| "range start must be a non-negative integer".to_owned())?;
    let end = values[1]
        .as_u64()
        .ok_or_else(|| "range end must be a non-negative integer".to_owned())?;
    Ok(
        usize::try_from(start).map_err(|_| "range start is too large".to_owned())?
            ..usize::try_from(end).map_err(|_| "range end is too large".to_owned())?,
    )
}

fn object<'a>(value: &'a Value, name: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{name} must be an object"))
}

fn array<'a>(value: &'a Value, name: &str) -> Result<&'a [Value], String> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{name} must be an array"))
}

fn string<'a>(object: &'a Map<String, Value>, name: &str) -> Result<&'a str, String> {
    object
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{name} must be a string"))
}

fn exact_string(object: &Map<String, Value>, name: &str, expected: &str) -> Result<(), String> {
    let actual = string(object, name)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{name} is {actual:?}, expected {expected:?}"))
    }
}

fn integer(object: &Map<String, Value>, name: &str) -> Result<i32, String> {
    let value = object
        .get(name)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{name} must be an integer"))?;
    to_i32(value, name)
}

fn usize_integer(object: &Map<String, Value>, name: &str) -> Result<usize, String> {
    let value = object
        .get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{name} must be a non-negative integer"))?;
    usize::try_from(value).map_err(|_| format!("{name} is too large"))
}

fn u16_integer(object: &Map<String, Value>, name: &str) -> Result<u16, String> {
    let value = object
        .get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{name} must be a non-negative integer"))?;
    u16::try_from(value).map_err(|_| format!("{name} is too large"))
}

fn to_i32(value: i64, name: &str) -> Result<i32, String> {
    i32::try_from(value).map_err(|_| format!("{name} is outside i32"))
}

fn one_char(value: &str, name: &str) -> Result<char, String> {
    let mut characters = value.chars();
    let character = characters
        .next()
        .ok_or_else(|| format!("{name} is empty"))?;
    if characters.next().is_some() {
        return Err(format!("{name} must contain one Unicode scalar"));
    }
    Ok(character)
}
